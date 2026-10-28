import { execFileSync } from 'node:child_process';
import { copyFile, mkdir, stat } from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

// Хук выполняется после компиляции, но до упаковки и подписи обновлений.
if (process.platform === 'linux' && process.env.TRANSCRIBER_PREPARE_LINUX_RELEASE === '1') {
  const required = ['SENTRY_AUTH_TOKEN', 'SENTRY_DSN', 'SENTRY_ORG', 'SENTRY_PROJECT_RUST'];
  if (required.some((name) => !process.env[name])) {
    console.log('Sentry configuration is incomplete; keeping Linux debug information in packages.');
  } else {
    const root = fileURLToPath(new URL('../', import.meta.url));
    const binary = path.join(root, 'src-tauri/target/release/transcriber');
    const directory = path.join(process.env.RUNNER_TEMP, 'transcriber-symbols');
    await mkdir(directory, { recursive: true });
    const symbols = path.join(directory, 'transcriber');
    const stripped = path.join(directory, 'transcriber-stripped');
    const cli = path.join(root, 'node_modules/@sentry/cli/bin/sentry-cli');
    const env = { ...process.env, SENTRY_PROJECT: process.env.SENTRY_PROJECT_RUST };
    const sentry = (...args) =>
      execFileSync(process.execPath, [cli, ...args], { cwd: root, env, encoding: 'utf8' });
    const inspect = (file) => JSON.parse(sentry('debug-files', 'check', '--json', file));

    await copyFile(binary, symbols);
    const original = inspect(symbols);
    if (
      !original.is_usable ||
      !original.features.includes('debug') ||
      original.variants.length === 0
    ) {
      throw new Error('Linux executable has no usable Sentry debug information.');
    }

    // Полный ELF содержит также таблицы раскрутки стека; исходный контекст прикладывает CLI.
    console.log(sentry('releases', 'new', `transcriber@${process.env.SENTRY_RELEASE_VERSION}`));
    console.log(
      sentry(
        'debug-files',
        'upload',
        '--include-sources',
        '--wait',
        '--require-all',
        '--id',
        original.variants[0].debug_id,
        symbols,
      ),
    );

    await copyFile(binary, stripped);
    execFileSync('strip', ['--strip-debug', stripped]);
    const compact = inspect(stripped);
    if (
      !compact.is_usable ||
      compact.features.includes('debug') ||
      !compact.features.includes('unwind') ||
      JSON.stringify(compact.variants) !== JSON.stringify(original.variants)
    ) {
      throw new Error('Stripping changed Linux debug identifiers or removed unwind information.');
    }
    const before = await stat(binary);
    const after = await stat(stripped);
    await copyFile(stripped, binary);
    console.log(
      `Linux executable: ${before.size} -> ${after.size} bytes; symbols uploaded to Sentry.`,
    );
  }
}
