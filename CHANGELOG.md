# Changelog

All notable changes to Transcriber are documented in this file.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).
Versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- A "Reset app data" action on the About settings tab moves all app data (history, settings, providers, dictionary) to a backup folder and restarts the app from a clean state, after a confirmation. The data is not deleted; it stays in the backup folder.
- If the app data was created by a newer version of the app than the one installed (possible because stable and pre-release builds share the same data), the app now shows a blocking screen explaining this and offering to update or reset, instead of risking data corruption.

### Changed

- Dictation history is now stored in a local database instead of a single JSON file. The app stays responsive while saving and browsing history even when the history is large.

### Fixed

- Dictating into a field inside the app itself now inserts the transcribed text instead of the previous clipboard contents. It was caused by the app briefly freezing while saving a large history, which no longer happens.
- Dictation paste no longer wipes non-text clipboard contents. Images, file lists, HTML, and other memory-backed clipboard formats are now restored after the transcription is pasted, instead of only text.
- Dictionary duplicate detection is now case-sensitive, so words differing only by case (e.g. "Alpha" and "ALPHA") are treated as distinct entries instead of being blocked or removed together.

## [0.1.1] - 2026-07-04

### Added

- On Windows, if speech-to-text or post-processing is not fully configured, starting dictation now shows a native system notification before recording begins. Clicking the notification opens the relevant settings tab.
- Backend-generated error messages now follow the selected app language, including configuration errors shown before recording starts.

### Changed

- Custom prompt fields now show and use the built-in default prompt/template until the user saves an override, even when custom prompts are enabled.
- Post-processing provider/model fields, prompt fields, and the post-processing test panel stay hidden until post-processing is enabled.

### Fixed

- Required provider and model fields in speech-to-text and post-processing settings are now highlighted immediately when the selection is incomplete.
- Custom prompt fields can now be reset back to their default values without disabling the custom prompt toggle.
- Closing the provider settings modal now clears stale validation errors, and existing processing settings are migrated automatically to the updated prompt storage format.

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
