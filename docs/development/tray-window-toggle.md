# Tray Window Toggle

A left click on the tray icon toggles the main window: it hides a window the user can see, and restores a window the user cannot. The whole decision lives in `toggle_main_window` in `src-tauri/src/background.rs`. The tray menu item `Открыть приложение` deliberately keeps calling `show_main_window` instead, because "Open" must never close anything.

## Why `is_visible()` alone is not enough

`WebviewWindow::is_visible()` reaches tao's `util::is_visible`, which is `IsWindowVisible(hwnd)`. That WinAPI call only reports the `WS_VISIBLE` style bit. It stays `true` in two cases where the user sees nothing:

- the window is minimized to the taskbar;
- the window sits on another Windows virtual desktop.

It becomes `false` only after an explicit `hide()` (`SW_HIDE`), which is exactly the "parked in tray" state produced by the `CloseRequested` handler.

So a naive `if window.is_visible() { window.hide() }` would hide an already-invisible window, and the click would look like it did nothing. The checks must run in this order:

1. `!is_visible() || is_minimized()` → `show_main_window` (show + unminimize + focus). `is_minimized()` maps to `IsIconic`, which is an honest signal, so it has to be tested separately rather than inferred from `is_visible()`.
2. visible, not minimized, but on another virtual desktop → `set_focus()` only.
3. otherwise → `hide()`.

## Why focus is not part of the decision

An obvious alternative is "hide when the window is focused, raise it otherwise". It cannot work here: clicking the tray icon moves the foreground window to the taskbar before the handler runs, so tao has already delivered `WindowEvent::Focused(false)` and `is_focused()` returns `false` for a window that was active a moment ago. Reconstructing the pre-click focus would need a timestamp heuristic on the last focus loss.

The product decision is therefore simpler and deterministic: a visible window on the current desktop is always hidden, even when another window occludes it.

## Why monitors are not compared

`overlay.rs` already knows how to resolve a monitor from a point (`cursor_position()` + `monitor_from_point()`), so comparing the window's monitor with the tray icon's monitor is cheap. It is intentionally not done. A window on a neighbouring monitor is still on screen, so hiding it matches what the user asked for. Only a different virtual desktop makes the window genuinely unreachable.

## How `set_focus()` crosses a virtual desktop

tao's `set_focus()` is guarded by `is_visible && !is_minimized && !is_foreground`, evaluated against its cached `WindowFlags`. A window on another virtual desktop satisfies all three (it keeps `VISIBLE`, is not `MINIMIZED`, and `GetForegroundWindow()` returns a window of the current desktop), so tao calls `force_window_active`, which ends in `SetForegroundWindow`. Windows responds by switching to the desktop that owns the window and activating it.

This is why branch 2 above calls `set_focus()` and nothing else. Calling `show()` first would be a no-op, and `hide()` would strand the window.

## COM apartment for `IVirtualDesktopManager`

`is_on_current_virtual_desktop` creates `IVirtualDesktopManager` (`windows::Win32::UI::Shell`, feature `Win32_UI_Shell`) and calls `IsWindowOnCurrentVirtualDesktop` with the HWND taken from `raw_window_handle`, the same way `overlay.rs::refresh_topmost` does.

The tray handler runs on the main thread, which tao has already put into a COM STA via `OleInitialize`. The guard mirrors `audio_mute.rs`:

```rust
let com_hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
let should_uninit = com_hr.is_ok();
// ...
if should_uninit {
    CoUninitialize();
}
```

Two rules encoded here:

- `CoUninitialize` must run only when this call actually entered an apartment (`S_OK` or `S_FALSE`). On `RPC_E_CHANGED_MODE` the thread already belongs to a different apartment; COM is still usable, but uninitializing would drop a reference the app does not own.
- The apartment must be `COINIT_APARTMENTTHREADED`, not `COINIT_MULTITHREADED` as in `audio_mute.rs`. `audio_mute` runs on its own worker thread, while this code runs on the UI thread — turning that thread into an MTA would break OLE drag-and-drop and WebView2.

## Error fallbacks

Nothing here is worth showing to the user, so `toggle_main_window` is invoked as `let _ = toggle_main_window(&app_handle)` and every probe has a default:

- `is_visible()` / `is_minimized()` → `unwrap_or(false)`. A failed probe then lands in the "restore" branch: showing a window that was already shown is harmless, hiding one the user wanted is not.
- `is_on_current_virtual_desktop()` → `unwrap_or(true)`. Treating an unknown desktop as the current one keeps the plain toggle working on single-desktop machines and when the COM call fails.

Non-Windows builds compile a stub that returns `Ok(true)`, so the toggle degrades to "visible window always hides".
