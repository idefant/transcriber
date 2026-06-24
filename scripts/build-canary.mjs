/**
 * Builds Transcriber in canary mode.
 *
 * Passes VITE_APP_CHANNEL=canary so the frontend shows the Canary badge,
 * and uses the canary Tauri config override for the window title and bundle icons.
 *
 * Usage: node scripts/build-canary.mjs
 */

import { spawnSync } from 'node:child_process';

const result = spawnSync(
  'npm',
  ['run', 'tauri', '--', 'build', '--config', 'src-tauri/tauri.canary.conf.json'],
  {
    stdio: 'inherit',
    shell: true,
    env: { ...process.env, VITE_APP_CHANNEL: 'canary' },
  },
);

process.exit(result.status ?? 1);
