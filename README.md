# Transcriber

React + TypeScript + Vite starter with React Router, SCSS modules, strict ESLint, Stylelint, Prettier, Husky, lint-staged, and `#/*` path aliases.

## Requirements

- Node.js 24+
- npm 11+

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

## Build

```bash
npm.cmd run build
```

The build script runs TypeScript checks first and then creates a production bundle in `dist`.

Preview the production build:

```bash
npm.cmd run preview
```

## Available Commands

```bash
# Start the Vite development server.
npm.cmd run dev

# Run typecheck and build the production bundle.
npm.cmd run build

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

## Git Hooks

Husky runs lint-staged before commits:

- JS/TS files: ESLint fix + Prettier
- CSS/SCSS files: Stylelint fix + Prettier
- HTML/JSON/Markdown/YAML files: Prettier
