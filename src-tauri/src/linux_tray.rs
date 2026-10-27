use std::sync::atomic::Ordering;

use ksni::{menu::StandardItem, MenuItem, TrayMethods};
use tauri::Manager;

use crate::{
    background::{self, BackgroundRuntime},
    dictation,
    error::{AppError, AppResult},
    i18n,
};

/** Linux-трей с отдельной активацией по ЛКМ и контекстным меню по ПКМ. */
pub(crate) struct LinuxTray {
    app: tauri::AppHandle,
    icon: Vec<u8>,
    width: i32,
    height: i32,
    can_copy: bool,
}

impl ksni::Tray for LinuxTray {
    fn id(&self) -> String {
        self.app.config().identifier.clone()
    }

    fn title(&self) -> String {
        "Transcriber".into()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        vec![ksni::Icon {
            width: self.width,
            height: self.height,
            data: self.icon.clone(),
        }]
    }

    fn activate(&mut self, _x: i32, _y: i32) {
        self.open();
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        vec![
            StandardItem {
                label: i18n::text(&self.app, "tray-open"),
                activate: Box::new(Self::open),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: i18n::text(&self.app, "tray-copy-latest"),
                enabled: self.can_copy,
                activate: Box::new(|tray: &mut Self| {
                    let app = tray.app.clone();
                    let dispatcher = app.clone();
                    let _ = dispatcher.run_on_main_thread(move || {
                        let _ = dictation::copy_latest_history_text_to_clipboard(&app);
                        background::refresh_tray_history_state(&app);
                    });
                }),
                ..Default::default()
            }
            .into(),
            MenuItem::Separator,
            StandardItem {
                label: i18n::text(&self.app, "tray-exit"),
                activate: Box::new(|tray: &mut Self| {
                    tray.app
                        .state::<BackgroundRuntime>()
                        .is_exiting
                        .store(true, Ordering::SeqCst);
                    tray.app.exit(0);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}

impl LinuxTray {
    fn open(&mut self) {
        let app = self.app.clone();
        let dispatcher = app.clone();
        let _ = dispatcher.run_on_main_thread(move || {
            let _ = background::show_main_window(&app);
        });
    }
}

/** Регистрирует StatusNotifierItem и сохраняет управление им на время работы приложения. */
pub(crate) fn setup(app: &tauri::AppHandle) -> AppResult<()> {
    let icon = app
        .default_window_icon()
        .ok_or_else(|| AppError::from(i18n::text(app, "tray-icon-not-found")))?;
    // StatusNotifierItem принимает ARGB32 в сетевом порядке байтов, Tauri хранит RGBA.
    let mut pixels = icon.rgba().to_vec();
    for pixel in pixels.as_chunks_mut::<4>().0 {
        pixel.rotate_right(1);
    }
    let tray = LinuxTray {
        app: app.clone(),
        icon: pixels,
        width: icon.width() as i32,
        height: icon.height() as i32,
        can_copy: false,
    };
    let handle = tauri::async_runtime::block_on(async {
        tokio::time::timeout(
            std::time::Duration::from_secs(5),
            tray.assume_sni_available(true).spawn(),
        )
        .await
    })
    .map_err(|error| AppError::from(error.to_string()))?
    .map_err(|error| AppError::from(error.to_string()))?;
    app.state::<BackgroundRuntime>()
        .tray
        .lock()
        .map_err(|_| AppError::from(i18n::text(app, "tray-state-lock-failed")))?
        .replace(handle);
    background::refresh_tray_history_state(app);
    Ok(())
}

/** Обновляет доступность копирования без блокировки главного потока на D-Bus. */
pub(crate) fn refresh_history(app: &tauri::AppHandle, can_copy: bool) {
    let handle = app
        .state::<BackgroundRuntime>()
        .tray
        .lock()
        .ok()
        .and_then(|tray| tray.clone());
    if let Some(handle) = handle {
        tauri::async_runtime::spawn(async move {
            handle.update(|tray| tray.can_copy = can_copy).await;
        });
    }
}
