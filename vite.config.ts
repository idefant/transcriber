import react from '@vitejs/plugin-react';
import { fileURLToPath, URL } from 'node:url';
import { defineConfig } from 'vite';
import checker from 'vite-plugin-checker';

export default defineConfig({
  build: {
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
});
