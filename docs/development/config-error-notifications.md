# Configuration Error Notifications

## Problem

Processing readiness (speech-to-text provider and model selected, model available for the provider, provider API key present, plus the same for post-processing when it is enabled) used to be validated only after recording, during the processing phase. The user recorded audio, waited through the transcribing spinner, and only then saw the red error overlay. Configuration mistakes were also never surfaced as OS notifications.

The goal: validate readiness before any recording or reprocessing starts, and for background/overlay flows surface the problem as a native system notification instead of a delayed overlay. Clicking the notification must open the relevant settings tab.

## Pre-flight validation

`validate_processing_ready` in `dictation.rs` reuses `runner::build_stt_snapshot` (and `runner::build_post_process_snapshot` when post-processing is enabled) plus `providers::resolve_provider_api_key`. The snapshot builders are pure configuration checks with no network calls: they verify the provider and model are selected, the model exists in the catalog, and the model is compatible with the provider.

Important subtlety: `build_stt_snapshot` does **not** validate the API key. Key presence is checked separately by `resolve_provider_api_key`, which the normal path only calls at request time. The pre-flight check therefore calls `resolve_provider_api_key` explicitly for the STT provider (and the post-processing provider when enabled), otherwise a missing key would slip past the pre-check and only fail later as a network-style error.

The function returns `Result<(), ConfigError>`, where `ConfigError { section, message }` identifies which settings tab the notification should open (`speechToText` or `postProcessing`) and carries the underlying error message.

The underlying configuration messages come from a tiny shared backend catalog (`i18n.rs`) keyed by `EffectiveUiLanguage`. `runner.rs` and `providers.rs` ask that helper for user-facing config-validation errors instead of hardcoding English strings inline. That keeps the pre-flight notification, the settings test panels, and any other caller of the same snapshot/provider helpers on the same app-language copy without duplicating the `match` logic in each call site.

## Where it hooks

`start_dictation_inner` runs the check before `overlay::show_recording_overlay` and `recording::start_recording`. On failure it drops the session lock, calls `notification::show_config_error`, and returns `Ok(())` — deliberately not `Err`. Returning `Err` would trigger the error branch in `start_dictation` (`emit_dictation_error` + `hide_recording_overlay`); we own the user-facing error through the notification instead.

`begin_repeat_latest_history_record` (the "repeat latest" hotkey flow) runs the same check before `overlay::show_transcribing_overlay` and returns `Ok(None)`, so the caller never spawns the processing task.

Both call sites leave the session `Idle` when the check fails. That is what keeps a hotkey release, a repeated press, or a cancel from erroring out: `take_recording` returns `Ok(None)` for any non-`Recording` state, and `cancel_dictation_inner` treats `Idle` as a no-op. Nothing was started, so nothing needs unwinding.

## Why only these two flows

The in-app History repeat buttons (`repeat_history_transcription`, `repeat_history_record`, `repeat_history_post_processing`) run with the main window open and already surface configuration errors inline on the record via `save_repeated_stt_error`. They do not use the overlay, so a system notification would be redundant and inconsistent with how the rest of the in-app UI reports errors. They are intentionally left unchanged.

The residual readiness check inside `process_recording_inner` is also kept as a safety net for the rare case where settings change mid-recording; that path still ends on the error overlay.

## Native notification (Windows-only)

`notification.rs` builds the toast with `tauri-winrt-notification` (`Toast`). The official `tauri-plugin-notification` was rejected because its action/click handling is mobile-only — it cannot run code when the user clicks a desktop toast on Windows, which is exactly what "click opens the relevant settings tab" requires.

Important caveat from the `tauri-winrt-notification` crate itself: an unpackaged desktop app should not use its own bundle identifier as the toast AppUserModelID. If the app is not installed and the AUMID is not registered, Windows can silently drop the toast even though `show()` returns `Ok(())`.

Because this project uses a separate dev identifier (`com.transcriber.desktop.dev`) for `tauri dev`, `notification.rs` treats identifiers ending with `.dev` as an unpackaged/dev build and uses `Toast::POWERSHELL_APP_ID` explicitly. That makes the toast visible in development (it appears as a PowerShell notification). Installed builds still use the real bundle identifier, so the toast is attributed to Transcriber normally.

`on_activated` runs on a WinRT background thread. Window operations are dispatched through `AppHandle::run_on_main_thread`, which shows the main window and emits `open-settings`. Windows closes the toast automatically on activation.

The module is `#[cfg(windows)]` with a no-op stub for other platforms, so the call sites compile unconditionally. `ConfigError` and `ConfigErrorSection` are platform-independent.

## Frontend

`App.tsx` subscribes to the `open-settings` event and calls `useUiStore.openSettings(section)`. The backend shows the main window before emitting, so the settings modal opens over an already focused window.
