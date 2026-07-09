# Hotkey Architecture

This document describes the two-path hotkey system: the native Windows hook and the in-app DOM handler, how they interact, the supported hotkey actions, the left/right modifier format, and a note on dev/prod settings divergence.

## Two paths

The app does **not** use Tauri's global-shortcut plugin. Instead it maintains two mechanisms and switches between them based on whether the main window is focused:

**Native `WH_KEYBOARD_LL` hook** (`src-tauri/src/shortcut_hook.rs`) — installed by `dictation::register_dictation_shortcut`, but enabled only while the main window is **not** focused. Runs on a dedicated thread with its own Windows message loop. In that state it operates globally and consumes (returns `1`) only key events that match one of the configured hotkeys; everything else passes through `CallNextHookEx`.

**In-app DOM handler** (`src/app/DictationHotkeyFallback/DictationHotkeyFallback.tsx`) — mounted in the main window only (`App.tsx`). Listens to `keydown`/`keyup` on `globalThis` with `capture: true`. It is the only hotkey path while the main window is focused and the webview receives key events. Handles the dictation hotkey, the cancel hotkey (gated by session state), the "copy latest transcription" hotkey, the "paste latest transcription" hotkey, and the "repeat latest transcription" hotkey.

## Supported actions

The hotkey layer currently supports five actions:

- dictation start/stop;
- dictation cancel;
- copy the latest final history text;
- paste the latest final history text;
- repeat processing for the latest history record.

The "copy latest", "paste latest", and "repeat latest" actions are optional and disabled when their stored hotkey string is empty.

## Paste-latest modifier conflict

The "paste latest transcription" hotkey uses clipboard staging plus a synthetic `Ctrl+V`. This creates a conflict when the triggering hotkey itself contains modifiers such as `Ctrl+Shift+V`: if the app sends `Ctrl+V` while the original chord is still physically held, the target application can inherit the held modifier state and treat the paste as a different shortcut.

One investigated alternative was to recognize the hotkey on `keydown`, immediately send synthetic key-up events for the hotkey keys, and then send `Ctrl+V` without waiting for physical release. That approach looked attractive because it would remove the user-visible delay and treat the hotkey as a consumed chord.

In practice, this does not reliably solve the problem on Windows when implemented with `SendInput`. Microsoft documents that `SendInput` does not reset the keyboard's current state, which means physically held keys can still interfere with injected input. In focused-window testing, synthetic key-up events for the triggering hotkey did not reliably neutralize the held modifiers before the following synthetic paste.

Because of that platform constraint, the code currently keeps the more conservative behavior: wait until the paste hotkey is released before sending the synthetic `Ctrl+V`. If future work revisits this area, it should assume that "programmatically release the user's held modifiers and paste immediately" is not a reliable `SendInput`-only strategy.

## Focus boundary

When the main window is **not** focused, the native hook handles everything. When the main window **is** focused, the backend uninstalls the low-level hook and leaves the focused-window case entirely to the DOM handler.

This is not just an optimization. Keeping the `WH_KEYBOARD_LL` hook installed while the Transcriber window was focused caused unrelated AutoHotkey shortcuts to stop firing, even when the pressed key was not one of the app's configured hotkeys. The problem reproduced with keys such as `F1`, `F13`, `F14`, `F15`, and `F16`, and the decisive diagnostic was that AHK started seeing the key again as soon as the native hook was fully removed. Tightening the hook's event-filtering logic was not sufficient; the fix had to be lifecycle-based.

The focus switch is driven by Tauri window events in `src-tauri/src/background.rs`. `WindowEvent::Focused(true)` calls `shortcut_hook::set_main_window_focused(true)` and tears the hook down. `WindowEvent::Focused(false)` enables it again, and startup also synchronizes the initial focus state so the hook is not left active when the app opens directly into a focused window.

Because of that split, there is no longer any intentional overlap between native and DOM handling in the focused-window case. The DOM handler still skips non-cancel processing when the hotkey capture lock is active (during hotkey recording in settings).

## DOM dispatch thread

The DOM-triggered commands (`dictation_shortcut_pressed`, `dictation_shortcut_released`, `cancel_dictation`) are declared as synchronous `#[tauri::command] pub fn`, which Tauri runs on the main event-loop thread. Starting or stopping dictation does slow, blocking work — overlay window creation, a WASAPI stream build, and COM audio-endpoint calls for "mute while recording" — and running that directly on the main thread used to freeze window dragging and title-bar buttons (the event loop stops pumping messages) and could deadlock the STA-threaded WebView2 event loop against COM marshaling, since the main thread blocked instead of pumping the messages that marshaling needs. This only affected the focused-window DOM path; the unfocused native hook path already ran the equivalent work on its own thread.

To fix this, the three commands in `dictation.rs` only enqueue a `DictationJob` onto an `mpsc` channel and return immediately. A single dedicated thread (`ensure_dictation_dispatch_thread`, started lazily and also eagerly from `register_dictation_shortcut`) drains the channel and calls the same `handle_dom_shortcut_pressed`/`handle_dom_shortcut_released`/`cancel_dictation_inner` functions the command handlers used to call directly. Because it is a single thread reading a FIFO channel, `pressed` is still guaranteed to be processed before a later `released` — the ordering the hold-mode `activation_id` invariant depends on (see "Hold-mode activation identity" below) is preserved. This mirrors `shortcut_hook::ensure_event_dispatch_thread`, which already does the same thing for the native hook path.

If you add a new DOM-triggered dictation command, route it through this dispatch thread rather than doing session/overlay/recording work directly in the command handler.

## Repeat-cancel boundary

The "repeat latest transcription" flow has the same cancel invariant as ordinary dictation: once the session is cancelled, the overlay must stay hidden and the pipeline must not enter a new visible phase.

The risky boundary is the hand-off from repeated STT to post-processing. A cancel can arrive after STT finishes but before post-processing starts. If the code shows the `processing` overlay without re-checking the live session state at that exact boundary, the cancelled session can resurrect the overlay and leave it orphaned because the cancel hotkey was already disarmed.

To avoid that regression, the repeat hotkey path must treat "enter post-processing" as a guarded transition, not as an unconditional continuation after successful STT. The transition helper in `dictation.rs` re-checks the session while switching from `Transcribing` to `Processing`, then verifies again after showing the overlay and hides it immediately if a late cancel won the race.

## Hold-mode activation identity

The focused-window DOM path has one extra race that the native hook path does not naturally expose: `keydown` / `cancel` / `keyup` are sent to the backend as separate async commands. If a hold-mode session is cancelled and the user immediately starts a new hold-mode session, the late `keyup` from the old activation can arrive after the new recording has already started.

That `keyup` must not be treated as "stop whatever is currently recording". Each DOM hold activation therefore gets its own monotonically increasing `activationId` in `DictationHotkeyFallback`, and both `dictation_shortcut_pressed` and `dictation_shortcut_released` carry that id into Rust.

`dictation.rs` stores only the currently active DOM hold activation id. A `Released` event stops recording only when its `activationId` still matches the active recording. If the session was cancelled and restarted, the stale release is ignored.

The same identity rule applies to the focused-window cancel hotkey. `dictation-session` now carries the active `sessionId`, and the DOM cancel path sends that id back with `cancel_dictation`. A late cancel from session A must not be allowed to cancel session B.

If future work touches the dictation hotkey protocol, preserve this invariant: transport events from the focused DOM path are not ordered strongly enough to infer identity from timing alone.

## Local cancellation vs remote cancellation

Cancelling dictation now aborts the local async task that is waiting on STT/post-processing. This is intentionally stronger than the old "mark the session cancelled and ignore the result later" behavior, because otherwise a cancelled session can keep a live task around long enough to race with the next session's UI updates.

This does **not** guarantee that the upstream AI provider stops processing the request. The local task is aborted, but the remote server may already have accepted the request and may continue its own work. The important application invariant is narrower: once the session is cancelled, the app must not keep waiting locally, must not transition the cancelled session into a new visible phase, and must not let stale completion handlers update overlay state for a later session.

There is one more UI-side invariant here: `hide_recording_overlay` uses a delayed native window hide to avoid flicker during state transitions, so each hide request must be tied to an overlay visibility generation. Without that guard, a delayed hide from session A can physically hide the already visible overlay of session B even when the React state and dictation session state are both correct.

## Left/right modifier format

Hotkey strings use an optional side prefix on each modifier token:

| Token   | Meaning                                |
| ------- | -------------------------------------- |
| `Ctrl`  | either side — any Ctrl key triggers    |
| `LCtrl` | left side only — right Ctrl must be up |
| `RCtrl` | right side only — left Ctrl must be up |

The same pattern applies to `Alt`/`LAlt`/`RAlt`, `Shift`/`LShift`/`RShift`, and `Win`/`LWin`/`RWin`.

Old settings stored without a prefix (`"Ctrl+Space"`) are backward-compatible: they parse as `Either` (any side). Normalization on load rewrites tokens to the canonical casing (`LCtrl`, `RCtrl`, `Ctrl`) so the format is consistent on disk.

Side matching is strict: recording with only the left Ctrl held stores `LCtrl`, which then requires the right Ctrl to be **up** at trigger time. Recording with both sides held stores `Ctrl` (either side).

### Rust

`enum ModifierSide { None, Either, Left, Right }` lives in `shortcut_hook.rs`. `Hotkey::parse` recognises `lctrl/rctrl`, `lalt/ralt`, `lshift/rshift`, `lwin/rwin`. `to_normalized_string` outputs `LCtrl/RCtrl/Ctrl`. `modifiers_match` calls `modifier_side_matches` with side-specific VK codes (`VK_LCONTROL 0xA2`, `VK_RCONTROL 0xA3`, etc.) via `GetAsyncKeyState`.

### Frontend

`src/shared/hotkey/` is the shared module:

- `parseHotkey` — parses a hotkey string into `ParsedHotkey { ctrl, alt, shift, meta: ModifierSide, key }`.
- `matchesHotkey(event, pressedModifierCodes, hotkey)` — compares an event against a parsed hotkey using a `Set<string>` of currently pressed modifier `event.code` values.
- `formatHotkeyFromEvent(event, pressedModifierCodes)` — converts a key event to a hotkey string with side-specific modifier tokens.
- `MODIFIER_CODES` — `Set<string>` of all modifier `event.code` values (both sides).
- `CODE_TO_KEY` — maps `event.code` to canonical key names mirroring Rust `parse_main_key` output.

Both `DictationHotkeyFallback` and `HotkeyInput` track pressed modifier codes themselves (a `Set<string>` built from `keydown`/`keyup` events for modifier codes) and pass the set into `matchesHotkey` / `formatHotkeyFromEvent`.

## Dev/prod settings divergence

DEV (`npm run dev:tauri`) and PROD use different Tauri `identifier` values (`com.transcriber.desktop.dev` vs `com.transcriber.desktop`), which means different app data directories and therefore **separate `settings.json` files**. A hotkey configured in one build does not appear in the other. If testing behavior across both builds, copy the settings file or configure the hotkey in both instances.
