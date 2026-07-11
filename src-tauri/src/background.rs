use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

use tauri::{
    menu::{MenuBuilder, MenuItem, MenuItemBuilder},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, WebviewWindow, WindowEvent, Wry,
};

use crate::{
    autostart::HIDDEN_START_ARG,
    dictation,
    error::{AppError, AppResult},
    history, i18n, shortcut_hook,
};

const MAIN_WINDOW_LABEL: &str = "main";
const MENU_OPEN_ID: &str = "open";
const MENU_COPY_LATEST_ID: &str = "copy_latest";
const MENU_EXIT_ID: &str = "exit";

#[derive(Default)]
pub struct BackgroundRuntime {
    copy_latest_item: Mutex<Option<MenuItem<Wry>>>,
    is_exiting: AtomicBool,
}

pub fn setup_background_mode(app: &tauri::AppHandle) -> AppResult<()> {
    setup_main_window_event_handlers(app)?;
    suppress_alt_menu_activation(app)?;
    setup_tray(app)?;
    apply_startup_window_visibility(app)?;
    sync_main_window_focus_state(app)?;

    Ok(())
}

pub fn refresh_tray_history_state(app: &tauri::AppHandle) {
    let can_copy = history::latest_history_text(app)
        .map(|text| !text.trim().is_empty())
        .unwrap_or(false);

    if let Some(item) = app
        .state::<BackgroundRuntime>()
        .copy_latest_item
        .lock()
        .ok()
        .and_then(|item| item.clone())
    {
        let _ = item.set_enabled(can_copy);
    }
}

fn setup_main_window_event_handlers(app: &tauri::AppHandle) -> AppResult<()> {
    let window = main_window(app)?;
    let app_handle = app.clone();

    window.on_window_event(move |event| match event {
        WindowEvent::CloseRequested { api, .. } => {
            let runtime = app_handle.state::<BackgroundRuntime>();

            if runtime.is_exiting.load(Ordering::SeqCst) {
                return;
            }

            api.prevent_close();
            let _ = main_window(&app_handle).and_then(|window| {
                window.hide()?;
                Ok(())
            });
        }
        WindowEvent::Focused(focused) => {
            shortcut_hook::set_main_window_focused(*focused);
        }
        _ => {}
    });

    Ok(())
}

/// Stops a tap on Alt from opening the window menu, which would swallow the next keystroke.
#[cfg(target_os = "windows")]
fn suppress_alt_menu_activation(app: &tauri::AppHandle) -> AppResult<()> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        UI::{
            Shell::{DefSubclassProc, SetWindowSubclass},
            WindowsAndMessaging::{SC_KEYMENU, WM_SYSCOMMAND},
        },
    };

    // Only has to be unique per subclass procedure, and this is the app's single subclass.
    const SUBCLASS_ID: usize = 1;

    unsafe extern "system" fn subclass_proc(
        hwnd: HWND,
        message: u32,
        w_param: WPARAM,
        l_param: LPARAM,
        _subclass_id: usize,
        _reference_data: usize,
    ) -> LRESULT {
        // Pressing and releasing Alt makes DefWindowProc post SC_KEYMENU, which puts the window
        // into the modal menu loop; the loop then eats the next keystroke before WebView2 sees it,
        // killing every in-app hotkey. tao keeps WS_SYSMENU even on an undecorated window (it
        // hides the frame through WM_NCCALCSIZE instead), so the message really does open the
        // invisible system menu. Neither tao, wry nor tauri-runtime-wry intercepts it. Swallow it
        // here: this subclass is installed last, so it runs before all of theirs. The low four
        // bits of w_param are reserved for internal use and must be masked off.
        if message == WM_SYSCOMMAND && (w_param & 0xFFF0) == SC_KEYMENU as usize {
            return 0;
        }

        DefSubclassProc(hwnd, message, w_param, l_param)
    }

    let window = main_window(app)?;

    let handle = window
        .window_handle()
        .map_err(|error| AppError::from(format!("Window handle error: {error}")))?;

    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return Err(AppError::from("Unsupported window handle".to_string()));
    };

    let is_installed = unsafe {
        SetWindowSubclass(
            handle.hwnd.get() as HWND,
            Some(subclass_proc),
            SUBCLASS_ID,
            0,
        )
    };

    if is_installed == 0 {
        return Err(AppError::from(
            "Could not subclass the main window".to_string(),
        ));
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn suppress_alt_menu_activation(_app: &tauri::AppHandle) -> AppResult<()> {
    Ok(())
}

fn setup_tray(app: &tauri::AppHandle) -> AppResult<()> {
    let open_item =
        MenuItemBuilder::with_id(MENU_OPEN_ID, i18n::text(app, "tray-open")).build(app)?;
    let copy_latest_item =
        MenuItemBuilder::with_id(MENU_COPY_LATEST_ID, i18n::text(app, "tray-copy-latest"))
            .enabled(false)
            .build(app)?;
    let exit_item =
        MenuItemBuilder::with_id(MENU_EXIT_ID, i18n::text(app, "tray-exit")).build(app)?;
    let menu = MenuBuilder::new(app)
        .items(&[&open_item, &copy_latest_item])
        .separator()
        .item(&exit_item)
        .build()?;

    app.state::<BackgroundRuntime>()
        .copy_latest_item
        .lock()
        .map_err(|_| AppError::from(i18n::text(app, "tray-state-lock-failed")))?
        .replace(copy_latest_item);

    let app_handle = app.clone();

    TrayIconBuilder::with_id("main")
        .tooltip("Transcriber")
        .icon(
            app.default_window_icon()
                .cloned()
                .ok_or_else(|| i18n::text(app, "tray-icon-not-found"))?,
        )
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_tray_icon_event(move |_tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                let _ = toggle_main_window(&app_handle);
            }
        })
        .on_menu_event(|app, event| match event.id().as_ref() {
            MENU_OPEN_ID => {
                let _ = show_main_window(app);
            }
            MENU_COPY_LATEST_ID => {
                let _ = dictation::copy_latest_history_text_to_clipboard(app);
                refresh_tray_history_state(app);
            }
            MENU_EXIT_ID => {
                app.state::<BackgroundRuntime>()
                    .is_exiting
                    .store(true, Ordering::SeqCst);
                app.exit(0);
            }
            _ => {}
        })
        .build(app)?;

    refresh_tray_history_state(app);

    Ok(())
}

fn apply_startup_window_visibility(app: &tauri::AppHandle) -> AppResult<()> {
    if std::env::args().any(|argument| argument == HIDDEN_START_ARG) {
        return Ok(());
    }

    show_main_window(app)
}

fn sync_main_window_focus_state(app: &tauri::AppHandle) -> AppResult<()> {
    let window = main_window(app)?;
    let is_focused = window.is_focused()?;

    shortcut_hook::set_main_window_focused(is_focused);

    Ok(())
}

pub(crate) fn show_main_window(app: &tauri::AppHandle) -> AppResult<()> {
    let window = main_window(app)?;

    window.show()?;
    window.unminimize()?;
    window.set_focus()?;

    Ok(())
}

/// Tray left-click behavior: hide the window only when the user can actually see it,
/// otherwise bring it back. Every failed check falls back to the "bring it back" side.
fn toggle_main_window(app: &tauri::AppHandle) -> AppResult<()> {
    let window = main_window(app)?;

    // `is_visible` maps to `IsWindowVisible`, which stays true for a minimized window and for
    // a window parked on another virtual desktop. Only `hide()` turns it false, so being
    // minimized has to be tested separately.
    let is_hidden = !window.is_visible().unwrap_or(false);
    let is_minimized = window.is_minimized().unwrap_or(false);

    if is_hidden || is_minimized {
        return show_main_window(app);
    }

    // A window on another virtual desktop is invisible to the user, so hiding it would look
    // like the click did nothing. Focus it instead: `SetForegroundWindow` makes Windows switch
    // to that desktop. When the query fails, assume the current desktop and keep the toggle.
    if !is_on_current_virtual_desktop(&window).unwrap_or(true) {
        window.set_focus()?;

        return Ok(());
    }

    window.hide()?;

    Ok(())
}

#[cfg(target_os = "windows")]
fn is_on_current_virtual_desktop(window: &WebviewWindow) -> AppResult<bool> {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows::Win32::{
        Foundation::HWND,
        System::Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_APARTMENTTHREADED,
        },
        UI::Shell::{IVirtualDesktopManager, VirtualDesktopManager},
    };

    let handle = window
        .window_handle()
        .map_err(|error| AppError::from(format!("Window handle error: {error}")))?;

    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return Err(AppError::from("Unsupported window handle".to_string()));
    };

    let hwnd = HWND(handle.hwnd.get() as *mut core::ffi::c_void);

    unsafe {
        // The tray handler runs on the main thread, which tao already put into a COM STA.
        // should_uninit is true only when this call entered an apartment (S_OK or S_FALSE).
        // On RPC_E_CHANGED_MODE COM stays usable but CoUninitialize must not run, otherwise it
        // would drop tao's OleInitialize reference. Apartment-threaded on purpose: a cold thread
        // must not become MTA, that would break OLE drag-and-drop.
        let com_hr = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        let should_uninit = com_hr.is_ok();

        let result = (|| -> windows::core::Result<bool> {
            let manager: IVirtualDesktopManager =
                CoCreateInstance(&VirtualDesktopManager, None, CLSCTX_ALL)?;

            Ok(manager.IsWindowOnCurrentVirtualDesktop(hwnd)?.as_bool())
        })();

        if should_uninit {
            CoUninitialize();
        }

        result.map_err(|error| AppError::from(format!("Virtual desktop query error: {error}")))
    }
}

#[cfg(not(target_os = "windows"))]
fn is_on_current_virtual_desktop(_window: &WebviewWindow) -> AppResult<bool> {
    Ok(true)
}

fn main_window(app: &tauri::AppHandle) -> AppResult<WebviewWindow> {
    app.get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or_else(|| i18n::text(app, "main-window-not-found").into())
}
