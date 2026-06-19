# Transcriber

React + TypeScript + Vite starter with React Router, SCSS modules, strict ESLint, Stylelint, Prettier, Husky, lint-staged, and `#/*` path aliases.

## Requirements

- Node.js 24+
- npm 11+
- Rust stable toolchain

On this Windows setup, use `npm.cmd` if PowerShell blocks `npm.ps1`.

## Install

```bash
npm.cmd install
```

The project includes `.npmrc` with `legacy-peer-deps=true`, so a plain install works with the current ESLint 10 plugin peer ranges.

`lint-staged` is pinned to a Git 2.30-compatible version for this local environment.

## Development

```bash
npm.cmd run dev
```

The dev server runs Vite with fast TypeScript transpilation. TypeScript, ESLint, and Stylelint diagnostics are shown in the terminal and in the browser overlay through `vite-plugin-checker`.

Default local URL:

```text
http://localhost:5173
```

Run the desktop application through Tauri:

```bash
npm.cmd run dev:tauri
```

## Build

```bash
npm.cmd run build
```

The build script runs TypeScript checks first and then creates a production bundle in `dist`.

Preview the production build:

```bash
npm.cmd run preview
```

Build the desktop bundle:

```bash
npm.cmd run build:tauri
```

## Available Commands

```bash
# Start the Vite development server.
npm.cmd run dev

# Start the Tauri desktop app in development mode.
npm.cmd run dev:tauri

# Run typecheck and build the production bundle.
npm.cmd run build

# Build the Tauri desktop bundle.
npm.cmd run build:tauri

# Run the Tauri CLI.
npm.cmd run tauri

# Serve the production build locally.
npm.cmd run preview

# Check TypeScript without emitting files.
npm.cmd run typecheck

# Run ESLint.
npm.cmd run lint

# Run ESLint and apply safe fixes.
npm.cmd run lint:fix

# Run Stylelint for CSS and SCSS files.
npm.cmd run stylelint

# Run Stylelint and apply safe fixes.
npm.cmd run stylelint:fix

# Format the project with Prettier.
npm.cmd run format

# Check Prettier formatting without writing changes.
npm.cmd run format:check

# Run the full quality gate: TypeScript, ESLint, Stylelint, Prettier check, and production build.
npm.cmd run check
```

## Model Testing

Post-processing model evals are documented in [docs/model-testing.md](docs/model-testing.md).

## Git Hooks

Husky runs lint-staged before commits:

- JS/TS files: ESLint fix + Prettier
- CSS/SCSS files: Stylelint fix + Prettier
- HTML/JSON/Markdown/YAML files: Prettier

## ToDo

- [Тестирование качества моделей](todo/model-testing.md) — отдельный CLI-харнесс (promptfoo) для массового тестирования качества ответов на любых моделях: тест-кейсы, шаблонные объекты-параметры, дешёвые прогоны.
- Локализация промптов — дефолтные системные промпты и шаблоны (STT и постобработка) должны зависеть от языка интерфейса (`uiLanguage` из настроек). Сейчас они захардкожены только на английском в `src-tauri/src/processing.rs` (`DEFAULT_STT_SYSTEM_PROMPT`, `DEFAULT_POST_PROCESS_SYSTEM_PROMPT`, `DEFAULT_POST_PROCESS_USER_TEMPLATE`) и отдаются через `get_default_prompts` без учёта языка.
