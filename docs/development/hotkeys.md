# Hotkey Architecture

This document describes the two-path hotkey system: the native Windows hook and the in-app DOM handler, how they interact, the left/right modifier format, and a note on dev/prod settings divergence.

## Two paths

The app does **not** use Tauri's global-shortcut plugin. Instead it maintains two independent mechanisms that work together:

**Native `WH_KEYBOARD_LL` hook** (`src-tauri/src/shortcut_hook.rs`) — installed once at startup (`dictation::register_dictation_shortcut`) and lives for the app's lifetime. Runs on a dedicated thread with its own Windows message loop. Operates globally and is focus-independent. Consumes (returns `1`) a key event only when it matches the configured dictation or cancel hotkey; everything else passes through `CallNextHookEx`.

**In-app DOM handler** (`src/app/DictationHotkeyFallback/DictationHotkeyFallback.tsx`) — mounted in the main window only (`App.tsx`). Listens to `keydown`/`keyup` on `globalThis` with `capture: true`. Only fires when the main window is focused and the webview receives key events. Handles both the dictation hotkey and the cancel hotkey (gated by session state). Acts as the fallback for the focused-window case.

## Focus boundary

When the main window is **not** focused, the native hook handles everything. When it **is** focused, the native hook still runs but the webview's DOM handler also sees key events. For most hotkeys both paths fire; `cancel_dictation` is idempotent so a double invocation is harmless. The DOM handler skips processing when the hotkey capture lock is active (during hotkey recording in settings).

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
