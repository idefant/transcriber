/**
 * Проверяет текстовые файлы на искажённые (mojibake) последовательности кодировки.
 * Использование:
 *   node scripts/check-encoding.mjs             # весь репозиторий
 *   node scripts/check-encoding.mjs <файлы...>  # только указанные файлы
 * Точечный режим используется Claude-хуком `.claude/hooks/encoding-check.mjs`
 * для быстрой проверки конкретной правки.
 */

import { readdir } from 'node:fs/promises';
import path from 'node:path';

import { scanFile, shouldCheckFile } from './lib/mojibake.mjs';

const rootDirectory = process.cwd();
const ignoredDirectories = new Set([
  '.git',
  '.codex',
  'coverage',
  'dist',
  'extensions',
  'node_modules',
  'storybook-static',
  'target',
  'ui-audit-artifacts',
]);

const findTextFiles = async (directory) => {
  const entries = await readdir(directory, { withFileTypes: true });
  const files = [];

  for (const entry of entries) {
    const absolutePath = path.join(directory, entry.name);

    if (entry.isDirectory()) {
      if (!ignoredDirectories.has(entry.name)) {
        files.push(...(await findTextFiles(absolutePath)));
      }

      continue;
    }

    if (entry.isFile() && shouldCheckFile(absolutePath)) {
      files.push(absolutePath);
    }
  }

  return files;
};

const explicitArguments = process.argv.slice(2);
const filesToCheck =
  explicitArguments.length > 0
    ? explicitArguments.map((file) => path.resolve(file)).filter(shouldCheckFile)
    : await findTextFiles(rootDirectory);

const findings = [];

for (const filePath of filesToCheck) {
  for (const finding of await scanFile(filePath)) {
    findings.push({ ...finding, filePath: path.relative(rootDirectory, filePath) });
  }
}

if (findings.length > 0) {
  console.error('Possible mojibake sequences were found:');

  for (const finding of findings) {
    console.error(
      `- ${finding.filePath}:${finding.line}:${finding.column} (${JSON.stringify(
        finding.sequence,
      )})`,
    );
  }

  process.exitCode = 1;
}
