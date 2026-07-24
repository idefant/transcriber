/**
 * Claude Code PostToolUse-хук. После записи файла (Write/Edit) проверяет его на
 * искажённые (mojibake) последовательности и, если находит, возвращает код 2 —
 * тогда агент видит проблему и исправляет её сразу, не дожидаясь общей проверки.
 *
 * Вход: JSON события на stdin (`tool_input.file_path`). Расширение вне списка
 * проверяемых → тихий выход 0. Список маркеров общий с `npm run encoding:check`.
 */

import path from 'node:path';

import { scanFile, shouldCheckFile } from '../../scripts/lib/mojibake.mjs';

const readStdin = async () => {
  let data = '';
  process.stdin.setEncoding('utf8');

  for await (const chunk of process.stdin) {
    data += chunk;
  }

  return data;
};

const input = JSON.parse((await readStdin()) || '{}');
const filePath = input?.tool_input?.file_path;

if (!filePath || !shouldCheckFile(filePath)) {
  process.exit(0);
}

let findings = [];

try {
  findings = await scanFile(filePath);
} catch {
  // Файл мог быть удалён или недоступен к моменту запуска хука — не мешаем работе.
  process.exit(0);
}

if (findings.length === 0) {
  process.exit(0);
}

const relativePath = path.relative(process.cwd(), filePath);
const lines = findings
  .map((finding) => `  - ${relativePath}:${finding.line}:${finding.column}`)
  .join('\n');

process.stderr.write(
  `В файле найдены признаки искажённой кодировки (mojibake):\n${lines}\n\n` +
    'Скорее всего UTF-8-текст сохранён в неверной кодировке. Проверь реальное содержимое файла ' +
    'в UTF-8 и перезапиши повреждённые фрагменты корректным текстом (docs/agent/encoding.md). ' +
    'Не переписывай файл вслепую только из-за того, как он выглядит в терминале.\n',
);
process.exit(2);
