# Cancel Hotkey Design

## Problem

The cancel hotkey (default: `Ctrl+Z`) must be suppressed only while a dictation session is active — from recording start to the end of post-processing. Outside a session, the cancel hotkey must pass through to other applications unchanged (so `Ctrl+Z` continues to work as Undo in text fields).

A naive approach — install a new `WH_KEYBOARD_LL` hook when recording starts and
uninstall it when the session ends — has problems: hook installation spawns a thread and
a Windows message loop, and uninstalling from a different thread requires
`UnhookWindowsHookEx`. This is expensive, adds teardown complexity, and creates a
time window where the hook might not be fully registered before the first key event.

## Solution: arm/disarm pattern

The app already runs a single persistent `WH_KEYBOARD_LL` hook for the dictation hotkey
(installed once at startup, lives for the app's lifetime). The cancel hotkey reuses this
same hook by adding a second gated state: `CANCEL_HOTKEY: OnceLock<Mutex<Option<HookHotkey>>>`.

- `None` = disarmed. The hook ignores all keys for cancellation and passes them through.
- `Some(config)` = armed. The hook matches and consumes the configured cancel key.

`arm_cancel_hotkey` is called in `start_dictation_inner` right after the session enters
`Recording`. `disarm_cancel_hotkey` is called in both `finish_session` (normal completion
and async-cancel paths) and `cancel_dictation_inner` (immediate recording-cancel path).

Disarming is idempotent: calling it from both sites is safe.

## Empty / disabled cancel hotkey

An empty `cancelHotkey` setting ("") means the cancel hotkey is disabled. `arm_cancel_hotkey`
checks for an empty/whitespace value and stores `None` instead of parsing — so the hook
never consumes any key. The empty string bypasses `normalize_hotkey` (which rejects empty
input) in both `update_app_settings_inner` and `load_app_settings`.

## In-app (DOM) cancel path

The native hook only fires when the app window is **not** focused (or when the OS routes the event through the hook thread before the webview sees it). When the main window is focused, the webview receives key events via DOM and the hook may not consume them reliably.

To ensure cancel also works when the window is focused, `DictationHotkeyFallback` (the in-app DOM handler) listens for the `dictation-session` event emitted by the backend:

- `start_dictation_inner` emits `{ active: true }` after the session enters `Recording`.
- `finish_session` and `cancel_dictation_inner` emit `{ active: false }` after disarming.

`DictationHotkeyFallback` sets an `isSessionActiveRef` flag from this event. On `keydown` for the cancel hotkey, it calls `cancel_dictation` only when `isSessionActiveRef.current === true`, then calls `event.preventDefault()` to suppress the native Undo action. Outside a session the key passes through untouched.

`cancel_dictation` (`cancel_dictation_inner` on the Rust side) is idempotent — calling it from both the native hook path and the DOM path in the same event cycle is safe.

## Key priority

If the dictation hotkey and the cancel hotkey are set to the same key, `try_consume_dictation_event` wins: `should_consume_event` tries dictation first and returns early on a match. This edge case is acceptable.
