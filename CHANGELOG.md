# Changelog

All notable changes to Transcriber are documented in this file.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.0-alpha.3] - 2026-06-24

### Added

- Canary release channel: pre-release (unstable) builds ship with a distinct application icon and are
  marked as «Canary» in the About settings tab. Canary and stable builds share the same product name
  and identifier, so they install to the same location and share settings and history.

## [0.1.0] - 2026-06-24

### Added

- Initial release of Transcriber desktop application.
- Speech-to-text via OpenAI, Groq, and OpenRouter providers.
- Post-processing of transcriptions with configurable prompts.
- Global hotkey for recording (default: Ctrl+Space).
- Cancel hotkey for discarding in-progress dictation.
- History view with per-month grouping and detail panel.
- Dictionary of custom words for transcription hints.
- Tray icon with quick actions (open, copy last transcription, quit).
- Launch at login and background mode.
- Light/dark/auto theme.
- Interface language: Russian, English, or system-detected.
- Debug logging (local only, no data sent to servers).
- Automatic storage migrations with corrupt-file backup.
- About section showing the installed version.
