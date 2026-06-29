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
    history,
    settings::{self, EffectiveUiLanguage},
    shortcut_hook,
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

fn setup_tray(app: &tauri::AppHandle) -> AppResult<()> {
    let labels = tray_labels(settings::get_effective_ui_language(app)?);
    let open_item = MenuItemBuilder::with_id(MENU_OPEN_ID, labels.open).build(app)?;
    let copy_latest_item = MenuItemBuilder::with_id(MENU_COPY_LATEST_ID, labels.copy_latest)
        .enabled(false)
        .build(app)?;
    let exit_item = MenuItemBuilder::with_id(MENU_EXIT_ID, labels.exit).build(app)?;
    let menu = MenuBuilder::new(app)
        .items(&[&open_item, &copy_latest_item])
        .separator()
        .item(&exit_item)
        .build()?;

    app.state::<BackgroundRuntime>()
        .copy_latest_item
        .lock()
        .map_err(|_| AppError::from("Could not lock tray state"))?
        .replace(copy_latest_item);

    let app_handle = app.clone();

    TrayIconBuilder::with_id("main")
        .tooltip("Transcriber")
        .icon(
            app.default_window_icon()
                .cloned()
                .ok_or("Tray icon was not found")?,
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
                let _ = show_main_window(&app_handle);
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

fn main_window(app: &tauri::AppHandle) -> AppResult<WebviewWindow> {
    app.get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or_else(|| "Main window was not found".into())
}

struct TrayLabels {
    open: &'static str,
    copy_latest: &'static str,
    exit: &'static str,
}

fn tray_labels(language: EffectiveUiLanguage) -> TrayLabels {
    match language {
        EffectiveUiLanguage::Ru => TrayLabels {
            open: "Открыть приложение",
            copy_latest: "Скопировать последнюю расшифровку",
            exit: "Выход",
        },
        EffectiveUiLanguage::En => TrayLabels {
            open: "Open application",
            copy_latest: "Copy latest transcription",
            exit: "Exit",
        },
    }
}
