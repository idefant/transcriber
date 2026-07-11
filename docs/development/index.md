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

## Environment Variables

Copy `.env.example` to `.env` and fill in the values:

```bash
cp .env.example .env
```

`.env` holds the API keys used by model testing (`scripts/model-testing`) and is gitignored — never commit real secrets. WebView2 remote debugging is intentionally not configured in `.env`; it is enabled by `npm run dev:tauri:debug` (see Development below), because `.env` does not reach the Tauri app process.

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

Run the desktop app with WebView2 remote debugging enabled. This opens a Chrome DevTools Protocol endpoint on port 9222 so Playwright (or the Playwright MCP) can attach for UI debugging and screenshots. Dev-only; never use it for production builds.

```bash
npm run dev:tauri:debug
```

It works from any shell (PowerShell, CMD, Git Bash) because it uses `cross-env` to set the WebView2 debug argument rather than shell-specific syntax. The CDP attach workflow is described in [../agent/screenshot-testing.md](../agent/screenshot-testing.md).

### React DevTools

Inspect React components (Components / Profiler panels) directly inside the app's DevTools. React DevTools is loaded into WebView2 as an unpacked browser extension. This is Windows-only (only WebView2 supports browser extensions) and dev-only: it is gated behind a debug build and `browserExtensionsEnabled` in `src-tauri/tauri.dev.conf.json`, so it never ships in production builds.

Provide the extension once; the folder is gitignored. Copy an installed Chrome React Developer Tools build into `src-tauri/extensions/react-devtools/`, for example from `C:\Users\<user>\AppData\Local\Google\Chrome\User Data\Default\Extensions\fmkadmapgofadopljbjfkapdkoienihi\<version>`.

1. Run `npm run dev:tauri`. No extra flag is needed: both dev commands pass `--unsafely-disable-devtools-self-xss-warnings` via `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS`, so the DevTools console also accepts pasted code without the "allow pasting" self-XSS prompt.
2. Open the in-app DevTools with F12.
3. Use the **Components** and **Profiler** tabs.

The extension is registered through `extensions_path` on the recording overlay window (`src-tauri/src/overlay.rs`); because Chromium extensions live at the profile level, it is then available in the main window's DevTools as well. Both windows must use the same `browserExtensionsEnabled` value, otherwise WebView2 requires separate data directories for them. On a profile's very first run the overlay installs the extension after the main window has already mounted React, so the Components/Profiler tabs may be missing until you relaunch the app once; the extension then persists in the WebView2 profile and loads early enough on every later run.

## Tauri Build Variants

The project uses three Tauri config files with different roles:

| File                               | Used by                                                                | Purpose                                |
| ---------------------------------- | ---------------------------------------------------------------------- | -------------------------------------- |
| `src-tauri/tauri.conf.json`        | `npm run build:tauri`, stable release builds, installed production app | Base production config                 |
| `src-tauri/tauri.dev.conf.json`    | `npm run dev:tauri`, `npm run dev:tauri:debug`                         | Dev-only overrides for the desktop app |
| `src-tauri/tauri.canary.conf.json` | `npm run build:tauri:canary`, GitHub Actions prerelease tags           | Canary/pre-release overrides           |

The important distinction is that `npm run build` is **not** a desktop build. It only runs TypeScript checks and produces the frontend bundle in `dist`. Window options such as `decorations`, bundle metadata, updater settings, and NSIS packaging are only applied when a real Tauri build runs.

### Command Matrix

| Command                      | What it builds or starts                              | Tauri config                                          |
| ---------------------------- | ----------------------------------------------------- | ----------------------------------------------------- |
| `npm run dev`                | Vite dev server only                                  | none                                                  |
| `npm run dev:tauri`          | Desktop app in dev mode                               | `tauri.conf.json` + `tauri.dev.conf.json` override    |
| `npm run dev:tauri:debug`    | Desktop app in dev mode with WebView2 CDP port `9222` | `tauri.conf.json` + `tauri.dev.conf.json` override    |
| `npm run build`              | Frontend production bundle only                       | none                                                  |
| `npm run build:tauri`        | Stable desktop bundle / installer                     | `tauri.conf.json`                                     |
| `npm run build:tauri:canary` | Canary desktop bundle / installer                     | `tauri.conf.json` + `tauri.canary.conf.json` override |

### Config Responsibilities

- `tauri.conf.json` is the canonical production baseline. Shared window behavior such as undecorated titlebar, size limits, updater public key, bundle targets, and default icons belongs here.
- `tauri.dev.conf.json` should only contain development-only differences, for example a different app title/identifier and `browserExtensionsEnabled` for React DevTools.
- `tauri.canary.conf.json` should only contain prerelease branding and other intentional canary-specific differences.

### Merge Gotcha: `app.windows`

Tauri config overrides are not deep-merged in a way that safely preserves every nested window field for array items. In practice, if an override file redefines `app.windows`, treat each overridden window object as needing all required shared fields to be repeated explicitly.

For this project that means a canary/dev override must keep structural window flags in sync with the base config, for example:

- `decorations: false`
- `shadow: true`
- `minWidth` / `minHeight`
- `visible`

Otherwise a prerelease or dev build can silently drift from stable behavior even if the base `tauri.conf.json` is correct.

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

# Start the Tauri desktop app in development mode (uses tauri.dev.conf.json override; loads React DevTools and disables the DevTools console self-XSS prompt).
npm run dev:tauri

# Start the Tauri desktop app with WebView2 remote debugging on port 9222 (for Playwright/MCP screenshots).
npm run dev:tauri:debug

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

# Run rustfmt --check and clippy with warnings denied.
npm run rust:check

# Run the full quality gate: TypeScript, ESLint, Stylelint, Prettier check, encoding check, Rust checks, and production build.
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

## Clipboard Snapshot and Restore

Why dictation paste snapshots every clipboard format, which formats cannot be copied, why only `CF_DIB` is kept out of the DIB pair, why `CF_BITMAP` synthesis is forced after a restore, and what `clipboard-win` does not cover are documented in [clipboard-restore.md](clipboard-restore.md).

## State Management

The Zustand store architecture, canonical sort order rule, history event subscription, and component-local vs. store state decisions are documented in [state-management.md](state-management.md).

## Resettable Default-Backed Fields

The shared contract for settings fields that store `string | null`, show a built-in default when unset, and support reset-to-default behavior is documented in [resettable-default-fields.md](resettable-default-fields.md).

## Configuration Error Notifications

The pre-flight readiness check before recording and hotkey repeat, why it reuses the snapshot builders plus `resolve_provider_api_key`, the native WinRT toast (and why the official notification plugin was rejected), and the `open-settings` click flow are documented in [config-error-notifications.md](config-error-notifications.md).

## Recording Prewarm

Why the capture stream is built once at startup and reused (paused) across sessions, how this removes the `build_input_stream` latency from the dictation-start hot path, the mute/settings-read moves off that path, default-device rebuilds, why the microphone indicator stays honest, and the mutex lock ordering are documented in [recording-prewarm.md](recording-prewarm.md).

## Alt and the Window Menu Loop

Why an undecorated window still keeps `WS_SYSMENU`, how a lone Alt tap drops it into the modal menu loop and swallows the next in-app hotkey, and why the `SC_KEYMENU` interception has to live in the app's own window subclass are documented in [alt-menu-key.md](alt-menu-key.md).

## Tray Window Toggle

Why the tray left-click cannot rely on `is_visible()` or `is_focused()` alone, why monitors are not compared while virtual desktops are, how `set_focus()` switches a virtual desktop, and the COM apartment rules for `IVirtualDesktopManager` are documented in [tray-window-toggle.md](tray-window-toggle.md).

## Git Hooks

Husky runs two layers before commits:

- `lint-staged` for staged frontend/docs files:
- JS/TS files: ESLint fix + Prettier
- CSS/SCSS files: Stylelint fix + Prettier
- HTML/JSON/Markdown/YAML files: Prettier
- `npm run rust:check` for the Tauri crate as a whole (`cargo fmt --check` + `cargo clippy --all-targets -- -D warnings`)
