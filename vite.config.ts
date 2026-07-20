/// <reference types="vitest/config" />
import { sentryVitePlugin } from '@sentry/vite-plugin';
import { storybookTest } from '@storybook/addon-vitest/vitest-plugin';
import react from '@vitejs/plugin-react';
import { playwright } from '@vitest/browser-playwright';
import { readFileSync } from 'node:fs';
import path from 'node:path';
import { fileURLToPath, URL } from 'node:url';
import { defineConfig } from 'vite';
import checker from 'vite-plugin-checker';

const packageJson = JSON.parse(readFileSync(new URL('package.json', import.meta.url), 'utf8')) as {
  version: string;
};
const sentryAuthToken = process.env.SENTRY_AUTH_TOKEN;
const sentryOrganization = process.env.SENTRY_ORG;
const sentryProject = process.env.SENTRY_PROJECT_REACT;
const isSentryUploadEnabled = Boolean(sentryAuthToken && sentryOrganization && sentryProject);
const sentryRelease = `transcriber@${process.env.SENTRY_RELEASE_VERSION ?? packageJson.version}`;
const dirname =
  typeof __dirname === 'undefined' ? path.dirname(fileURLToPath(import.meta.url)) : __dirname;

// Подробнее: https://storybook.js.org/docs/next/writing-tests/integrations/vitest-addon
export default defineConfig({
  define: {
    __APP_VERSION__: JSON.stringify(packageJson.version),
  },
  build: {
    sourcemap: isSentryUploadEnabled,
    rollupOptions: {
      input: {
        main: fileURLToPath(new URL('index.html', import.meta.url)),
        overlay: fileURLToPath(new URL('src/overlay/index.html', import.meta.url)),
      },
    },
  },
  clearScreen: false,
  plugins: [
    react(),
    ...(isSentryUploadEnabled
      ? [
          sentryVitePlugin({
            authToken: sentryAuthToken,
            org: sentryOrganization,
            project: sentryProject,
            release: {
              name: sentryRelease,
            },
            sourcemaps: {
              assets: './dist/**',
              filesToDeleteAfterUpload: './dist/**/*.map',
            },
          }),
        ]
      : []),
    checker({
      eslint: {
        lintCommand: 'eslint "./src/**/*.{ts,tsx}" "./*.{js,ts}"',
      },
      overlay: {
        initialIsOpen: true,
        position: 'br',
      },
      stylelint: {
        lintCommand: 'stylelint "src/**/*.{css,scss}"',
      },
      terminal: true,
      typescript: true,
    }),
  ],
  resolve: {
    alias: {
      '#': fileURLToPath(new URL('src', import.meta.url)),
    },
  },
  server: {
    port: 5173,
    strictPort: true,
    watch: {
      ignored: ['**/src-tauri/**'],
    },
  },
  test: {
    projects: [
      {
        extends: true,
        test: {
          environment: 'node',
          exclude: ['src/**/*.stories.tsx'],
          include: ['src/**/*.test.ts', 'src/**/*.test.tsx'],
          name: 'unit',
        },
      },
      {
        extends: true,
        plugins: [
          // Этот плагин запускает тесты для историй (stories), определённых в конфигурации Storybook
          // Опции см. здесь: https://storybook.js.org/docs/next/writing-tests/integrations/vitest-addon#storybooktest
          storybookTest({
            configDir: path.join(dirname, '.storybook'),
          }),
        ],
        test: {
          name: 'storybook',
          browser: {
            enabled: true,
            headless: true,
            provider: playwright({}),
            instances: [
              {
                browser: 'chromium',
              },
            ],
          },
        },
      },
    ],
  },
});
