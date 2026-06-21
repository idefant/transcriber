# Transcriber

React + TypeScript + Vite starter with React Router, SCSS modules, strict ESLint, Stylelint, Prettier, Husky, lint-staged, and `#/*` path aliases.

## Requirements

- Node.js 24+
- npm 11+
- Rust stable toolchain

Use `npm` for project commands. In Windows PowerShell, `npm` can resolve to `npm.ps1`; if execution policy blocks it, use `npm.cmd` for the same command or run the command from Git Bash/Command Prompt.

## Install

```bash
npm install
```

The project includes `.npmrc` with `legacy-peer-deps=true`, so a plain install works with the current ESLint 10 plugin peer ranges.

## Development

```bash
npm run dev
```

The dev server runs Vite with fast TypeScript transpilation. TypeScript, ESLint, and Stylelint diagnostics are shown in the terminal and in the browser overlay through `vite-plugin-checker`.

Default local URL:

```text
http://localhost:5173
```

Run the desktop application through Tauri:

```bash
npm run dev:tauri
```

## Build

```bash
npm run build
```

The build script runs TypeScript checks first and then creates a production bundle in `dist`.

Preview the production build:

```bash
npm run preview
```

Build the desktop bundle:

```bash
npm run build:tauri
```

## Available Commands

```bash
# Start the Vite development server.
npm run dev

# Start the Tauri desktop app in development mode.
npm run dev:tauri

# Run typecheck and build the production bundle.
npm run build

# Build the Tauri desktop bundle.
npm run build:tauri

# Run the Tauri CLI.
npm run tauri

# Serve the production build locally.
npm run preview

# Check TypeScript without emitting files.
npm run typecheck

# Run ESLint.
npm run lint

# Run ESLint and apply safe fixes.
npm run lint:fix

# Run Stylelint for CSS and SCSS files.
npm run stylelint

# Run Stylelint and apply safe fixes.
npm run stylelint:fix

# Format the project with Prettier.
npm run format

# Check Prettier formatting without writing changes.
npm run format:check

# Check text files for common UTF-8/Windows-codepage mojibake sequences.
npm run encoding:check

# Run the full quality gate: TypeScript, ESLint, Stylelint, Prettier check, encoding check, and production build.
npm run check
```

## Model Testing

Post-processing model evals are documented in [docs/model-testing.md](docs/model-testing.md).

## Available Post-processing Models

| Модель                | Провайдер          | Рекомендуется |
| --------------------- | ------------------ | ------------- |
| gpt-4o-mini           | OpenAI, OpenRouter | ✅            |
| gpt-4.1-mini          | OpenAI, OpenRouter | ✅            |
| gpt-5.4-mini          | OpenAI, OpenRouter | ✅            |
| gpt-5-mini            | OpenAI, OpenRouter | ❌            |
| Qwen 3.6 27B          | Groq, OpenRouter   | ✅            |
| GPT OSS 120B          | Groq, OpenRouter   | ✅            |
| Llama 4 Scout         | Groq, OpenRouter   | ✅            |
| Gemini 2.5 Flash      | OpenRouter         | ✅            |
| Gemini 2.5 Flash Lite | OpenRouter         | ✅            |
| Gemini 3.1 Flash Lite | OpenRouter         | ✅            |
| Claude Haiku 4.5      | OpenRouter         | ✅            |

## Git Hooks

Husky runs lint-staged before commits:

- JS/TS files: ESLint fix + Prettier
- CSS/SCSS files: Stylelint fix + Prettier
- HTML/JSON/Markdown/YAML files: Prettier

## ToDo

- Локализация промптов — дефолтные системные промпты и шаблоны (STT и постобработка) должны зависеть от языка интерфейса (`uiLanguage` из настроек). Сейчас они захардкожены только на английском в `src-tauri/src/processing.rs` (`DEFAULT_STT_SYSTEM_PROMPT`, `DEFAULT_POST_PROCESS_SYSTEM_PROMPT`, `DEFAULT_POST_PROCESS_USER_TEMPLATE`) и отдаются через `get_default_prompts` без учёта языка.
