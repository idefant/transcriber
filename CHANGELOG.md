# Changelog

All notable changes to Transcriber are documented in this file.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

## [0.1.0] - 2026-07-02

### Added

- Initial release of Transcriber desktop application.
- Speech-to-text via OpenAI, Groq, and OpenRouter providers.
- Post-processing of transcriptions with configurable prompts; if post-processing fails, the recognized text is still inserted, with a warning shown on the overlay.
- Global hotkey for recording (default: Ctrl+Space); hotkeys can bind a specific left/right modifier key (e.g. left Ctrl only) instead of either side.
- Cancel hotkey for discarding in-progress dictation.
- Hotkeys for copying, pasting, and re-processing the latest transcription without reopening the app.
- Recording overlay shown as a compact bottom bar or a large centered panel, on the cursor's screen only or on every screen; the compact bar tracks the taskbar if it moves or resizes.
- Overlay shows an error or warning status when recognition or post-processing fails, with an expandable response body and a link that brings the app to the front and opens the matching history record.
- History view with per-month grouping and detail panel; result and history text is selectable and copyable.
- Dictionary of custom words for transcription hints.
- Tray icon with quick actions (open, copy last transcription, quit).
- Launch at login and background mode.
- Single running instance; relaunching brings the existing window to the front instead of opening a second one.
- Custom in-app window header with drag region and minimize/maximize/close controls, replacing the OS title bar.
- Right-click context menu disabled throughout the app.
- Global hotkeys are suspended while the main app window is focused, so typing inside the app doesn't trigger them.
- Light/dark/auto theme.
- Interface language: Russian, English, or system-detected.
- Debug logging (local only, no data sent to servers).
- Automatic storage migrations with corrupt-file backup.
- Deleting a provider that's selected for speech-to-text or post-processing automatically clears the now-invalid selection.
- Clipboard is restored to its previous contents after dictated text is inserted.
- Update notifications can be toggled on/off; the update-available banner shows a progress countdown, pauses on hover, and its download button opens the About settings tab.
- Canary release channel: pre-release (unstable) builds ship with a distinct application icon and are marked as «Canary» in the About settings tab. Canary and stable builds share the same product name and identifier, so they install to the same location and share settings and history.
- About section showing the installed version.
