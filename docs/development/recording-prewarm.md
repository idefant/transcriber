# Recording Prewarm (Warm Capture Stream)

## Problem

Starting a dictation felt laggy: the recording overlay appeared instantly, but the microphone only began capturing a few hundred milliseconds later, so the first fraction of the user's speech was lost.

Measurements of the start path (Windows, cpal 0.16 over WASAPI) pinned the cost on building the capture stream, not on showing the overlay:

| Stage                                                        |           avg | cold (first dictation after launch) |
| ------------------------------------------------------------ | ------------: | ----------------------------------: |
| `default_input_device()` + `default_input_config()`          |         ~4 ms |                              ~23 ms |
| **`build_input_stream()`**                                   | **34–231 ms** |                    **up to 511 ms** |
| `stream.play()`                                              |      ~0.01 ms |                            ~0.01 ms |
| time to first audio callback                                 |      ~3–16 ms |                              ~20 ms |
| `OutputMuteGuard::new()` (when "mute while recording" is on) |     +17–22 ms |                          +57–171 ms |

`build_input_stream` (WASAPI `IAudioClient::Activate` + `Initialize`) dominated, and its cost was highly variable depending on how "warm" the audio engine was. Because `start_dictation_inner` showed the overlay first and then built and started the stream synchronously on the hotkey dispatch thread, the overlay was an inaccurate "you can talk now" signal.

## Solution: a prepared, reusable capture stream

`recording::PreparedRecorder` builds the capture stream once and keeps it paused between sessions. The expensive `build_input_stream` is paid off the hot path; starting a dictation is then only a buffer reset plus `stream.play()`.

- `prepare_recorder` does all the expensive work (device enumeration, config negotiation, `build_input_stream`) and returns a paused recorder.
- `PreparedRecorder::start` clears leftover samples, sets the `active` flag, and calls `stream.play()`. This is the entire dictation-start hot path and measures ~3 ms to the first real audio callback, stably.
- `PreparedRecorder::stop_to_audio` pauses, takes the captured samples, and encodes WAV, leaving the recorder paused and empty for reuse.
- `PreparedRecorder::abort` pauses and discards samples (used on cancel), also leaving it reusable.

The audio callback only appends samples while `active` is set, so a paused-but-alive stream accumulates nothing. `active` is set before `play()` so the very first callbacks are recorded, and cleared before `pause()` so no late callback appends stray samples.

The stream is prewarmed at startup on a background thread via `dictation::prewarm_recorder`, called from the Tauri `setup` hook in `lib.rs` right after `create_recording_overlay`. Prewarm failure is non-fatal: if no warm stream exists on the first dictation, it is built on demand (the old, slow behavior) and then kept for reuse.

## Lifecycle and ownership

The prepared recorder lives in `DictationRuntime::prepared_recorder` (`Mutex<Option<PreparedRecorder>>`), not inside the session. This is what makes it survive across sessions.

`DictationSession::Recording` now carries only a lightweight `RecordingHandle` holding the session `started_at` and the output-mute guard. On stop, `finish_recording` reads the audio from the shared prepared recorder and then drops the handle to un-mute; on cancel, `cancel_dictation_inner` calls `release_recording` (abort) and drops the handle. `cpal::Stream` is `Send` on this platform (it was already stored in Tauri-managed state before this change), so keeping it in managed state is fine.

## Mute moved off the pre-capture path (Variant B)

`OutputMuteGuard::new()` (COM: `CoInitialize` + endpoint enumeration + `SetMute`) used to run before `build_input_stream`, adding 17–171 ms in front of the microphone opening. It now runs in `acquire_output_mute` **after** capture has started, so its cost no longer delays the first captured sample. The functional spec already allows recording to proceed if muting fails, so a best-effort guard after start is compliant. On a very cold first dictation the output stays briefly un-muted (tens to ~170 ms) before mute applies; this is acceptable and only affects the first run.

## Single settings read on the start path (Variant C)

`start_dictation_inner` reads `AppSettings` once and reuses it for both the mute flag and the cancel hotkey. `prepare_recorder` reads no settings at all (the old `start_recording` read them twice for UI language and the mute flag).

## Default input device changes

A prewarmed stream is bound to the device that was default when it was built. `begin_recording` checks `PreparedRecorder::is_for_current_default_device` (compares the current default input device name) and rebuilds when it differs, so plugging in a headset is honored. The rebuild is the slow path again, but only on the rare start right after a device change.

## Microphone indicator and privacy

Keeping an initialized `IAudioClient` alive between sessions does **not** keep the OS microphone in use. The privacy indicator and "in use" state track the capture stream being _started_ (`IAudioClient::Start` / `stream.play()`), not merely initialized. `stop_to_audio` and `abort` call `stream.pause()` (`IAudioClient::Stop`) immediately, which turns the indicator off and stops any data flow; only a paused, empty, initialized client persists. This is why the user-facing spec ("the app releases the microphone immediately after recording stops") is unchanged and did not need editing.

## Lock ordering

Two mutexes are involved: `DictationRuntime::session` and `DictationRuntime::prepared_recorder`. The only nested acquisition is `session` → `prepared_recorder` (in `start_dictation_inner`/`begin_recording` and in `cancel_dictation_inner`/`release_recording`). No path acquires `session` while holding `prepared_recorder` (`finish_recording`, `prewarm_recorder`, and `begin_recording`'s reuse path touch only `prepared_recorder`), so there is no inverse ordering and no deadlock. Keep it that way when adding code that touches both.

## Constraints to preserve

- Do not leave `active` set between sessions; the callback must never accumulate while paused.
- Always `pause()` on stop and cancel so the microphone indicator turns off promptly.
- Un-mute by dropping the `RecordingHandle`'s guard after the microphone is released, not before.
- Rebuild the prepared recorder when the default input device changes; do not silently record from the old device.
- If prewarm or reuse is ever removed, the on-demand build in `begin_recording` must remain as the fallback.
