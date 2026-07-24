/**
 * Claude Code Stop-хук. Когда агент собирается завершить ответ, смотрит в
 * `git status`: если в рабочем дереве есть незакоммиченные изменения кода,
 * стилей, конфигурации или документации, возвращает код 2 с напоминанием
 * прогнать проверки из docs/agent/after-work-checks.md. Тяжёлые проверки сам
 * НЕ запускает — только напоминает; решение и запуск остаются за агентом.
 *
 * Защита от зацикливания: если Stop уже вызван из-за срабатывания этого хука
 * (`stop_hook_active`), выходим молча, чтобы не уходить в бесконечный цикл.
 */

import { execSync } from 'node:child_process';
import path from 'node:path';

const relevantExtensions = new Set([
  '.cjs',
  '.css',
  '.js',
  '.jsx',
  '.json',
  '.md',
  '.mjs',
  '.rs',
  '.scss',
  '.toml',
  '.ts',
  '.tsx',
  '.yaml',
  '.yml',
]);

const readStdin = async () => {
  let data = '';
  process.stdin.setEncoding('utf8');

  for await (const chunk of process.stdin) {
    data += chunk;
  }

  return data;
};

const input = JSON.parse((await readStdin()) || '{}');

if (input.stop_hook_active) {
  process.exit(0);
}

let porcelain = '';

try {
  // --untracked-files=all разворачивает новые каталоги в отдельные файлы,
  // иначе git показал бы каталог одной записью без расширения.
  porcelain = execSync('git status --porcelain --untracked-files=all', { encoding: 'utf8' });
} catch {
  // Не git-репозиторий или git недоступен — молчим.
  process.exit(0);
}

const changedFiles = porcelain
  .split('\n')
  .map((line) => line.slice(3).trim())
  .filter(Boolean)
  // Переименования в porcelain выглядят как «old -> new»; берём итоговый путь.
  .map((entry) => (entry.includes(' -> ') ? entry.split(' -> ').at(-1) : entry));

const relevant = changedFiles.filter((file) =>
  relevantExtensions.has(path.extname(file).toLowerCase()),
);

if (relevant.length === 0) {
  process.exit(0);
}

const preview = relevant
  .slice(0, 10)
  .map((file) => `  - ${file}`)
  .join('\n');
const more = relevant.length > 10 ? `\n  …и ещё ${relevant.length - 10}` : '';

process.stderr.write(
  `В рабочем дереве есть незакоммиченные изменения кода/стилей/доков:\n${preview}${more}\n\n` +
    'Прежде чем завершать, прогони проверки из docs/agent/after-work-checks.md ' +
    '(npm run typecheck / lint / stylelint / format:check / encoding:check / rust:check / rust:test, ' +
    'при необходимости build) и сообщи их результат. ' +
    'Если проверки уже пройдены в этой сессии — подтверди это и заверши.\n',
);
process.exit(2);
