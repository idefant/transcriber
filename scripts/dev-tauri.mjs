import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { fileURLToPath } from 'node:url';

const args = process.argv.slice(2);
const debug = args[0] === '--debug';
if (debug) args.shift();

const linux = process.platform === 'linux';
if (linux) {
  await import('./setup-linux-runtime.mjs');
  if (debug) process.env.WEBKIT_INSPECTOR_HTTP_SERVER = '127.0.0.1:9222';
} else if (process.platform === 'win32') {
  const browserArgs = [
    process.env.WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS,
    '--unsafely-disable-devtools-self-xss-warnings',
    debug ? '--remote-debugging-port=9222' : undefined,
  ];
  process.env.WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = browserArgs.filter(Boolean).join(' ');
}

// Отдельный процесс сохраняет цикл событий Node.js до завершения Tauri CLI.
const cli = spawn(
  process.execPath,
  [
    fileURLToPath(import.meta.resolve('@tauri-apps/cli/tauri.js')),
    'dev',
    '--config',
    linux ? 'src-tauri/tauri.dev.linux.conf.json' : 'src-tauri/tauri.dev.conf.json',
    ...args,
  ],
  { stdio: 'inherit' },
);
const [code] = await once(cli, 'exit');
process.exitCode = code ?? 1;
