# Alt and the Window Menu Loop

Tapping Alt inside the main window used to kill the next keystroke: any in-app hotkey pressed right after Alt did nothing at all. `suppress_alt_menu_activation` in `src-tauri/src/background.rs` fixes this by subclassing the main window and swallowing `WM_SYSCOMMAND` when `wParam` masks down to `SC_KEYMENU`.

## Why an undecorated window still has a menu

`tauri.conf.json` sets `decorations: false`, so it is tempting to assume the window has no menu and no system menu. It does have one. tao builds every window with `WS_CAPTION | WS_CLIPSIBLINGS | WS_SYSMENU` in `WindowFlags::to_window_styles` and never removes `WS_SYSMENU` for undecorated windows — the frame disappears because tao intercepts `WM_NCCALCSIZE`, not because the styles changed. tao even calls `GetSystemMenu` to grey out `SC_CLOSE`, which only works because the system menu exists.

## The failure chain

1. The user presses and releases Alt. `DefWindowProc` interprets a lone Alt tap as a request to activate the window menu and posts `WM_SYSCOMMAND` with `wParam == SC_KEYMENU` to the top-level window.
2. `DefWindowProc` handles `SC_KEYMENU` by entering the modal menu loop. There is no menu bar to open, but the loop still runs and captures the keyboard.
3. The next keystroke is treated as a menu mnemonic and consumed. It never reaches the WebView2 child window, so no DOM `keydown` fires.
4. Meanwhile the native `WH_KEYBOARD_LL` hook is uninstalled, because the window is focused (see [hotkeys.md](hotkeys.md), "Focus boundary"). The focused window has exactly one hotkey path — the DOM handler — and the menu loop just ate its input.

The result is that every in-app hotkey silently stops working until the menu loop exits, including the ones the native hook suppresses globally. Clicking inside the window or tapping Alt a second time leaves the loop and restores normal behavior, which is the quickest way to confirm this diagnosis.

## Why the fix has to live here

Nothing upstream intercepts `SC_KEYMENU`:

- tao's `WM_SYSCOMMAND` branch handles only `SC_RESTORE`, `SC_MINIMIZE`, and `SC_SCREENSAVE`, then falls through to `DefWindowProc`;
- wry's `parent_subclass_proc` handles size/move/focus/destroy messages and forwards the rest to `DefSubclassProc`;
- `tauri-runtime-wry`'s `subclass_parent` (from `undecorated_resizing.rs`) handles `WM_SIZE` and shadow updates only, and is attached solely when the window is both resizable and undecorated.

So the app installs its own subclass with `SetWindowSubclass`. Subclass procedures run last-installed-first, and this one is attached from `setup_background_mode`, i.e. after tao, wry, and `tauri-runtime-wry` have registered theirs. It therefore sees `WM_SYSCOMMAND` before any of them and can return `0` without calling `DefSubclassProc`, so the message never reaches `DefWindowProc` and the menu loop never starts.

Two details matter:

- Mask with `& 0xFFF0`. The low four bits of `wParam` in `WM_SYSCOMMAND` are reserved for internal use, so comparing the raw value against `SC_KEYMENU` misses real messages.
- Removing `WS_SYSMENU` from `GWL_STYLE` is not a substitute. tao reapplies its styles in `apply_diff`, so the style would come back.

## What this intentionally disables

`SC_KEYMENU` is also how Windows opens the system menu on Alt+Space and how it resolves Alt+letter menu mnemonics. Swallowing it removes both. The window has no menu bar, and its system menu is invisible on an undecorated window, so neither is a loss.

Alt+F4 is unaffected: it arrives as `SC_CLOSE`, not `SC_KEYMENU`, and still reaches the `CloseRequested` handler that hides the window into the tray.
