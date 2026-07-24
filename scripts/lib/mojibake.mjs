/**
 * Общие утилиты поиска искажённых (mojibake) последовательностей, возникающих
 * при неверной перекодировке UTF-8 ↔ кодовая страница Windows. Используются и
 * CLI-проверкой `scripts/check-encoding.mjs`, и Claude-хуком
 * `.claude/hooks/encoding-check.mjs`, чтобы список маркеров оставался единым.
 */

import { readFile } from 'node:fs/promises';
import path from 'node:path';

/** Расширения текстовых файлов, которые имеет смысл проверять на mojibake. */
export const checkedExtensions = new Set([
  '.css',
  '.html',
  '.js',
  '.jsx',
  '.json',
  '.md',
  '.rs',
  '.scss',
  '.toml',
  '.ts',
  '.tsx',
]);

// Код-поинты заданы hex-строками намеренно: иначе сам этот файл содержал бы
// искажённые последовательности, и `npm run encoding:check` пометил бы его как заражённый.
/** Последовательности-маркеры искажённой кодировки UTF-8/Windows-1251. */
export const suspiciousSequences = [
  '0420 045f',
  '0420 0459',
  '0420 045a',
  '0420 045b',
  '0420 040e',
  '0420 201d',
  '0420 2019',
  '0420 0405',
  '0420 00b5',
  '0421 0402',
  '0421 0403',
  '0421 201a',
  '0421 040a',
  '0421 2039',
  '0421 040b',
  '0421 040f',
  '0432 0402',
  '00c2',
  '00d0',
  '00d1',
].map((group) => String.fromCodePoint(...group.split(' ').map((hex) => Number.parseInt(hex, 16))));

const getLineAndColumn = (text, index) => {
  const precedingText = text.slice(0, index);
  const lines = precedingText.split('\n');

  return {
    column: lines.at(-1).length + 1,
    line: lines.length,
  };
};

/** Сканирует уже прочитанный текст и возвращает найденные подозрительные последовательности с координатами `{ line, column, sequence }`. */
export const scanText = (text) => {
  const findings = [];

  for (const sequence of suspiciousSequences) {
    const index = text.indexOf(sequence);

    if (index === -1) {
      continue;
    }

    findings.push({ ...getLineAndColumn(text, index), sequence });
  }

  return findings;
};

/** Возвращает `true`, если файл с таким путём подлежит проверке кодировки. Сравнение расширения регистронезависимо. */
export const shouldCheckFile = (filePath) =>
  checkedExtensions.has(path.extname(filePath).toLowerCase());

/** Читает файл в UTF-8 и возвращает найденные mojibake-последовательности. Пустой массив означает, что файл чист. */
export const scanFile = async (filePath) => scanText(await readFile(filePath, 'utf8'));
