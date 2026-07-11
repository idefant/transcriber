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

/// Не даёт нажатию Alt открыть меню окна, которое иначе поглотило бы следующее нажатие клавиши.
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

    // Должен быть уникальным только в пределах процедуры подкласса, а это единственный подкласс в приложении.
    const SUBCLASS_ID: usize = 1;

    unsafe extern "system" fn subclass_proc(
        hwnd: HWND,
        message: u32,
        w_param: WPARAM,
        l_param: LPARAM,
        _subclass_id: usize,
        _reference_data: usize,
    ) -> LRESULT {
        // Нажатие и отпускание Alt заставляет DefWindowProc отправить SC_KEYMENU, что переводит окно
        // в модальный цикл меню; этот цикл затем поглощает следующее нажатие клавиши до того, как его увидит WebView2,
        // убивая любой хоткей внутри приложения. tao сохраняет WS_SYSMENU даже у окна без рамки (вместо этого
        // оно скрывает рамку через WM_NCCALCSIZE), поэтому сообщение действительно открывает
        // невидимое системное меню. Ни tao, ни wry, ни tauri-runtime-wry его не перехватывают. Поглощаем его
        // здесь: этот подкласс устанавливается последним, поэтому выполняется раньше всех остальных. Младшие четыре
        // бита w_param зарезервированы для внутреннего использования, и их нужно замаскировать.
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

/// Поведение при клике левой кнопкой по иконке в трее: скрывать окно только тогда, когда пользователь
/// действительно может его видеть, иначе возвращать его на экран. Любая неудавшаяся проверка
/// приводит к варианту «вернуть на экран».
fn toggle_main_window(app: &tauri::AppHandle) -> AppResult<()> {
    let window = main_window(app)?;

    // `is_visible` соответствует `IsWindowVisible`, которое остаётся true и для свёрнутого окна, и для
    // окна, находящегося на другом виртуальном рабочем столе. В false его переводит только `hide()`,
    // поэтому свёрнутое состояние нужно проверять отдельно.
    let is_hidden = !window.is_visible().unwrap_or(false);
    let is_minimized = window.is_minimized().unwrap_or(false);

    if is_hidden || is_minimized {
        return show_main_window(app);
    }

    // Окно на другом виртуальном рабочем столе невидимо пользователю, поэтому его скрытие выглядело бы
    // так, будто клик ничего не сделал. Вместо этого фокусируем его: `SetForegroundWindow` заставляет Windows
    // переключиться на этот рабочий стол. Если запрос не удался, считаем, что окно на текущем рабочем столе, и сохраняем переключение.
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
        // Обработчик трея выполняется в главном потоке, который tao уже перевёл в COM STA.
        // should_uninit равен true, только если этот вызов вошёл в апартамент (S_OK или S_FALSE).
        // При RPC_E_CHANGED_MODE COM остаётся пригодным для использования, но CoUninitialize вызывать нельзя, иначе
        // это сбросило бы ссылку tao на OleInitialize. Apartment-threaded выбран намеренно: холодный поток
        // не должен становиться MTA, это сломало бы OLE drag-and-drop.
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
