# Development

This note collects project setup, local development, build commands, and development tools for Transcriber.

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

Start the Vite development server:

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

Build the frontend production bundle:

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

# Start the Tauri desktop app in development mode (uses tauri.dev.conf.json override).
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

## Release Pipeline

The tag-driven CI/CD workflow, update channels (stable/unstable), minisign key setup, and how the Tauri updater delivers updates are documented in [release-pipeline.md](release-pipeline.md).

## Storage Migrations

The `_meta.json` versioning scheme, `migrations::run` call order, first-run detection, `load_json_or_default` vs `load_json_strict`, atomic `save_json`, and how to add a migration step are documented in [storage-migrations.md](storage-migrations.md).

## Model Testing

Post-processing model evals are documented in [model-testing.md](model-testing.md).

## Debug Logging

Local model-call debug logging is documented in [debug-logging.md](debug-logging.md).

## Hotkey Architecture

The two-path hotkey system (native hook + in-app DOM handler), left/right modifier format, and dev/prod settings divergence are documented in [hotkeys.md](hotkeys.md).

## Cancel Hotkey

The arm/disarm pattern, in-app DOM cancel path, and session gating are documented in [cancel-hotkey.md](cancel-hotkey.md).

## State Management

The Zustand store architecture, canonical sort order rule, history event subscription, and component-local vs. store state decisions are documented in [state-management.md](state-management.md).

## Git Hooks

Husky runs lint-staged before commits:

- JS/TS files: ESLint fix + Prettier
- CSS/SCSS files: Stylelint fix + Prettier
- HTML/JSON/Markdown/YAML files: Prettier
