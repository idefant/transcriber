# Release Pipeline

This document describes how Transcriber releases are built, published, and delivered to users via automatic updates.

## Overview

Releases are fully automated through GitHub Actions. The trigger is pushing a git tag that matches `v*` (e.g. `v1.2.3` or `v1.2.3-beta.1`). No manual intervention is needed except tagging.

The update delivery uses two channels — **stable** and **unstable** — published as JSON manifest files on GitHub Pages. The Tauri updater fetches the appropriate manifest at runtime.

## Versioning

The single source of truth for the version is the git tag. CI runs `node scripts/set-version.mjs <version>` before building, which writes the same version into three files:

- `package.json` → `version`
- `src-tauri/tauri.conf.json` → `version`
- `src-tauri/Cargo.toml` → `[package] version`

These files should not be edited manually for releases.

### Stable vs pre-release

A tag is a pre-release if its version contains a `-` (e.g. `v1.2.0-beta.1`, `v2.0.0-alpha.3`). CI sets `prerelease: true` for such tags. GitHub only marks a release as «Latest» for non-pre-release tags.

## Release Workflow (`.github/workflows/release.yml`)

Steps:

1. `actions/checkout` with `fetch-depth: 0` (needed to inspect full history for CHANGELOG).
2. Node 24 + Rust stable + `swatinem/rust-cache` for Cargo artifacts.
3. Version is extracted from the tag (`v1.2.3` → `1.2.3`); pre-release flag derived from `-` in version.
4. `node scripts/set-version.mjs` syncs the version into all three manifests.
5. `node scripts/extract-changelog.mjs` extracts the release-notes section from `CHANGELOG.md`.
6. `npm ci` installs frontend dependencies.
7. `tauri-apps/tauri-action@v0` builds the NSIS installer, signs the update artifact, creates the GitHub Release, and attaches `latest.json` (the Tauri updater manifest).
8. `gh release download` fetches the built `latest.json`.
9. The manifest is copied to `unstable.json` unconditionally, and to `stable.json` only for non-pre-release tags, then committed to the `gh-pages` branch.
10. The workflow uploads those JSON files as a GitHub Pages artifact and deploys them with `actions/deploy-pages`.

Required secrets: `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.

## Update Channels

Two JSON files on GitHub Pages serve as update manifests:

| File            | Updated on                | Used when                                          |
| --------------- | ------------------------- | -------------------------------------------------- |
| `stable.json`   | Non-pre-release tags only | Default; `isOfferUnstableVersionsEnabled` is false |
| `unstable.json` | Every tag                 | `isOfferUnstableVersionsEnabled` is true           |

Both files live on the `gh-pages` branch of the repository and are also deployed to GitHub Pages by the release workflow. GitHub Pages must be enabled for the repository with source set to `GitHub Actions` rather than `Deploy from a branch`.

## Signing Keys

The Tauri updater requires a minisign key pair to verify update authenticity.

Generate the key pair once:

```bash
npm run tauri signer generate -- -w transcriber-updater.key
```

- `transcriber-updater.key` — private key. **Never commit this file.** Store it securely.
- `transcriber-updater.key.pub` — public key. Committed to the repository and copied into `tauri.conf.json` under `plugins.updater.pubkey`.

Add the private key and its password to GitHub repository secrets:

- `TAURI_SIGNING_PRIVATE_KEY` — contents of `transcriber-updater.key`.
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — the password chosen during key generation.

**Losing the private key means existing installations can never be updated automatically.** Back it up in a secure location.

## Update Delivery in the App

The Rust side lives in `src-tauri/src/updater.rs`. Two Tauri commands are exposed:

- `check_for_update(offer_unstable: bool)` — queries the appropriate endpoint, stores the discovered `Update` in `PendingUpdate` managed state, returns `UpdateInfo { version, notes }` or `null`.
- `download_and_install_update()` — takes the stored `Update`, downloads it, emits `updater://progress` events with `{ downloaded, total }` payload, then calls `app.restart()`.

The frontend bridges these via `src/shared/updaterApi.ts`. Shared frontend state for update discovery, cached pending version, and install progress lives in `src/stores/updaterStore.ts`, while the settings modal visibility/active section live in `src/stores/uiStore.ts`.

On startup, `UpdateChecker` in `App.tsx` runs a silent check once after settings load only when `isUpdateNotificationsEnabled` is true. If an update is found, a bottom-right notification appears with a `Download` action, Ant Design's built-in 10-second progress bar (`showProgress: true`), and `pauseOnHover: true`. Clicking `Download` does not start the updater; it opens the existing `About` settings tab.

The full update UI (manual check button, cached pending version, install button, download progress, update-notification switch, unstable-channel switch) lives in `src/app/AppSettingsModal/AboutSettingsTab`. Entering the `About` tab always triggers a fresh update check, but the cached result from `updaterStore` is shown immediately so the install action is already visible if startup detection found a version earlier.

When `isOfferUnstableVersionsEnabled` is true, the app still checks `unstable.json` exactly as before. That manifest always points to the newest published release overall, so users on the unstable channel can be offered a stable release when it is the latest one available.

## CHANGELOG Format

`CHANGELOG.md` uses [Keep a Changelog](https://keepachangelog.com/en/1.1.0/) format. Each release section starts with `## [X.Y.Z] - YYYY-MM-DD`. The `scripts/extract-changelog.mjs` script extracts text between the matching `## [X.Y.Z]` heading and the next `## ` heading; the result is used as the GitHub Release body.

## Canary Branding for Pre-releases

Pre-release builds (tags containing `-`, e.g. `v0.1.0-alpha.1`) are built with a separate canary
variant that differs visually from stable builds while remaining the same application (same
`productName` and `identifier` so the installer path and user data are shared).

Canary-specific changes applied during build:

| Aspect           | Value                              |
| ---------------- | ---------------------------------- |
| Bundle icons     | `src-tauri/icons-canary/`          |
| Window title     | `Transcriber Canary`               |
| Frontend channel | `VITE_APP_CHANNEL=canary`          |
| About tab badge  | «Canary» tag shown next to version |

### Config override

`src-tauri/tauri.canary.conf.json` is a partial Tauri config that overrides only the window title
and `bundle.icon`. It is applied via `--config src-tauri/tauri.canary.conf.json`.

Important: because the override also redefines `app.windows`, shared structural window fields must
be repeated there explicitly instead of assuming they will safely inherit from `tauri.conf.json`.
Keep these in sync with the base config when changing shell behavior:

- `decorations`
- `shadow`
- `width` / `height`
- `minWidth` / `minHeight`
- `visible`

### CI automation

In `.github/workflows/release.yml`, the `Build and publish Tauri release` step sets:

- `env.VITE_APP_CHANNEL` — `canary` for pre-release tags, `stable` otherwise.
- `with.args` — `--config src-tauri/tauri.canary.conf.json` for pre-release tags, empty otherwise.

Pre-releases continue to flow into the `unstable` update channel as before.

### Local canary build

```bash
npm run build:tauri:canary
```

This uses `cross-env` to set `VITE_APP_CHANNEL=canary` and passes the canary config override to `tauri build`.

## SmartScreen Note

The minisign signature verifies update integrity within the Tauri updater. It is **not** an Authenticode code-signing certificate. Without an Authenticode certificate, Windows SmartScreen may show a warning when users first install (but not on silent updates). Code-signing is a separate, paid step not covered by this pipeline.
