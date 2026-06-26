use std::{
    sync::{Mutex, OnceLock},
    thread,
    time::Duration,
};

use serde::Serialize;
use tauri::{
    Emitter, Manager, Monitor, PhysicalPosition, PhysicalSize, Position, Size, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder,
};

use crate::{
    error::{AppError, AppResult},
    settings::{self, OverlayScreenMode, OverlayVariant},
};

const OVERLAY_LABEL_PREFIX: &str = "recording_overlay_";

/// Card sizes — must match the `.overlay` dimensions in the component SCSS.
const BOTTOM_CARD_WIDTH: f64 = 180.0;
const BOTTOM_CARD_HEIGHT: f64 = 40.0;
const CENTER_CARD_WIDTH: f64 = 220.0;
/// Upper bound on the center card height. The card is centered inside the
/// window, so the exact value only needs to be ≥ the tallest state.
const CENTER_CARD_HEIGHT: f64 = 220.0;

/// Transparent margin around the card, large enough to fit the card's CSS drop
/// shadow without the (rectangular) window clipping it. The native window
/// shadow is disabled, so this margin stays fully transparent.
const BOTTOM_SHADOW_MARGIN: f64 = 36.0;
const CENTER_SHADOW_MARGIN: f64 = 80.0;

/// Distance from the screen bottom to the bottom edge of the bottom-variant card.
const OVERLAY_BOTTOM_OFFSET: f64 = 72.0;

const HIDE_DELAY_MS: u64 = 250;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayShowPayload {
    state: &'static str,
    variant: OverlayVariant,
    /// History record to reveal from the error/warning overlay actions. `None`
    /// for the regular recording/transcribing/processing states.
    record_id: Option<String>,
}

/// Last requested overlay content. Newly created per-monitor windows read it on
/// mount via `get_overlay_state`, so they render correctly even if they missed
/// the `show-overlay` event that was broadcast before their webview was ready.
fn current_overlay() -> &'static Mutex<Option<OverlayShowPayload>> {
    static CURRENT: OnceLock<Mutex<Option<OverlayShowPayload>>> = OnceLock::new();
    CURRENT.get_or_init(|| Mutex::new(None))
}

fn overlay_label(index: usize) -> String {
    format!("{OVERLAY_LABEL_PREFIX}{index}")
}

fn overlay_index(label: &str) -> Option<usize> {
    label
        .strip_prefix(OVERLAY_LABEL_PREFIX)
        .and_then(|rest| rest.parse::<usize>().ok())
}

pub fn create_recording_overlay(app: &tauri::AppHandle) -> AppResult<()> {
    // Warm up a single overlay webview at startup so the first dictation shows
    // without webview-init lag. Extra per-monitor windows are created lazily.
    build_overlay_window(app, &overlay_label(0))?;

    Ok(())
}

#[tauri::command]
pub fn get_overlay_state() -> Option<OverlayShowPayload> {
    current_overlay()
        .lock()
        .ok()
        .and_then(|guard| guard.clone())
}

/// Dismiss the overlay on demand. Used by the error/warning overlay close button
/// and its auto-hide timer in the frontend.
#[tauri::command]
pub fn dismiss_overlay(app: tauri::AppHandle) {
    let _ = hide_recording_overlay(&app);
}

pub fn show_recording_overlay(app: &tauri::AppHandle) -> AppResult<()> {
    show_overlay_state(app, "recording", None)
}

pub fn show_transcribing_overlay(app: &tauri::AppHandle) -> AppResult<()> {
    show_overlay_state(app, "transcribing", None)
}

pub fn show_processing_overlay(app: &tauri::AppHandle) -> AppResult<()> {
    show_overlay_state(app, "processing", None)
}

/// Show the red error overlay (e.g. speech-to-text failed). `record_id` enables
/// the "open record" action; pass `None` for failures without a saved record.
pub fn show_error_overlay(app: &tauri::AppHandle, record_id: Option<String>) -> AppResult<()> {
    show_overlay_state(app, "error", record_id)
}

/// Show the amber warning overlay (post-processing failed but the speech-to-text
/// text was still inserted). `record_id` enables the "open record" action.
pub fn show_warning_overlay(app: &tauri::AppHandle, record_id: Option<String>) -> AppResult<()> {
    show_overlay_state(app, "warning", record_id)
}

pub fn hide_recording_overlay(app: &tauri::AppHandle) -> AppResult<()> {
    if let Ok(mut current) = current_overlay().lock() {
        *current = None;
    }

    let windows = overlay_windows(app);

    if windows.is_empty() {
        return Ok(());
    }

    let _ = app.emit("hide-overlay", ());

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(HIDE_DELAY_MS));
        for window in windows {
            let _ = window.hide();
        }
    });

    Ok(())
}

pub fn emit_mic_levels(app: &tauri::AppHandle, levels: Vec<f32>) {
    let _ = app.emit("mic-level", levels);
}

fn show_overlay_state(
    app: &tauri::AppHandle,
    state: &'static str,
    record_id: Option<String>,
) -> AppResult<()> {
    let app_settings = settings::load_app_settings(app)?;
    let variant = app_settings.overlay_variant().clone();
    let screen_mode = app_settings.overlay_screen_mode().clone();

    let base = build_overlay_window(app, &overlay_label(0))?;
    let monitors = target_monitors(app, &base, &screen_mode)?;

    if monitors.is_empty() {
        return Err(AppError::from("No monitor is available for recording overlay"));
    }

    let payload = OverlayShowPayload {
        state,
        variant: variant.clone(),
        record_id,
    };

    // Store the state before building windows so any window that mounts late can
    // recover it through `get_overlay_state`.
    if let Ok(mut current) = current_overlay().lock() {
        *current = Some(payload.clone());
    }

    for (index, monitor) in monitors.iter().enumerate() {
        let window = build_overlay_window(app, &overlay_label(index))?;

        position_overlay(&window, monitor, &variant)?;
        window.show()?;
        window.set_always_on_top(true)?;
        refresh_topmost(&window);
    }

    // Hide overlay windows for monitors that are no longer targeted (e.g. after
    // switching from "every screen" to "cursor" or unplugging a monitor).
    hide_surplus_overlays(app, monitors.len());

    app.emit("show-overlay", payload)?;

    Ok(())
}

fn build_overlay_window(app: &tauri::AppHandle, label: &str) -> AppResult<WebviewWindow> {
    if let Some(window) = app.get_webview_window(label) {
        return Ok(window);
    }

    #[cfg_attr(not(all(debug_assertions, target_os = "windows")), allow(unused_mut))]
    let mut builder = WebviewWindowBuilder::new(
        app,
        label,
        WebviewUrl::App("src/overlay/index.html".into()),
    )
    .title("Recording")
    .inner_size(
        BOTTOM_CARD_WIDTH + BOTTOM_SHADOW_MARGIN * 2.0,
        BOTTOM_CARD_HEIGHT + BOTTOM_SHADOW_MARGIN * 2.0,
    )
    .decorations(false)
    .transparent(true)
    .shadow(false)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .visible(false)
    .focused(false)
    .accept_first_mouse(true);

    // React DevTools (только dev, только Windows): включаем расширения WebView2 и
    // загружаем распакованное расширение в общий профиль. Расширения Chromium живут
    // на уровне профиля, поэтому панель Components доступна и в DevTools главного
    // окна. Значение browser_extensions_enabled должно совпадать с browserExtensionsEnabled
    // главного окна (tauri.dev.conf.json), иначе WebView2 требует разные data-каталоги.
    #[cfg(all(debug_assertions, target_os = "windows"))]
    {
        builder = builder.browser_extensions_enabled(true);

        let extensions_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/extensions");
        if std::path::Path::new(extensions_dir).exists() {
            builder = builder.extensions_path(extensions_dir);
        }
    }

    Ok(builder.build()?)
}

fn overlay_windows(app: &tauri::AppHandle) -> Vec<WebviewWindow> {
    app.webview_windows()
        .into_iter()
        .filter_map(|(label, window)| overlay_index(&label).map(|_| window))
        .collect()
}

fn target_monitors(
    app: &tauri::AppHandle,
    base: &WebviewWindow,
    screen_mode: &OverlayScreenMode,
) -> AppResult<Vec<Monitor>> {
    match screen_mode {
        OverlayScreenMode::All => {
            let monitors = base.available_monitors()?;

            if monitors.is_empty() {
                Ok(base.primary_monitor()?.into_iter().collect())
            } else {
                Ok(monitors)
            }
        }
        OverlayScreenMode::Cursor => {
            let monitor = match app.cursor_position() {
                Ok(position) => app
                    .monitor_from_point(position.x, position.y)?
                    .or(base.primary_monitor()?),
                Err(_) => base.primary_monitor()?,
            };

            Ok(monitor.into_iter().collect())
        }
    }
}

fn position_overlay(
    window: &WebviewWindow,
    monitor: &Monitor,
    variant: &OverlayVariant,
) -> AppResult<()> {
    let scale = monitor.scale_factor();

    // The window is the card plus a transparent margin that holds the card's CSS
    // drop shadow (the card is centered inside the window via `place-items`).
    let (card_height, margin) = match variant {
        OverlayVariant::Bottom => (BOTTOM_CARD_HEIGHT, BOTTOM_SHADOW_MARGIN),
        OverlayVariant::Center => (CENTER_CARD_HEIGHT, CENTER_SHADOW_MARGIN),
    };
    let card_width = match variant {
        OverlayVariant::Bottom => BOTTOM_CARD_WIDTH,
        OverlayVariant::Center => CENTER_CARD_WIDTH,
    };

    let physical_width = ((card_width + margin * 2.0) * scale).round();
    let physical_height = ((card_height + margin * 2.0) * scale).round();

    window.set_size(Size::Physical(PhysicalSize::new(
        physical_width as u32,
        physical_height as u32,
    )))?;

    let monitor_position = monitor.position();
    let monitor_size = monitor.size();

    // Center the window (and therefore the card) horizontally.
    let x = monitor_position.x + ((monitor_size.width as f64 - physical_width) / 2.0).round() as i32;
    let y = match variant {
        OverlayVariant::Center => {
            monitor_position.y + ((monitor_size.height as f64 - physical_height) / 2.0).round() as i32
        }
        OverlayVariant::Bottom => {
            // The card is centered in the window, so its center sits at the window
            // center. Place the window so the card bottom ends OVERLAY_BOTTOM_OFFSET
            // above the screen bottom.
            let card_bottom = monitor_size.height as f64 - OVERLAY_BOTTOM_OFFSET * scale;
            let window_top = card_bottom - (card_height * scale) / 2.0 - physical_height / 2.0;
            monitor_position.y + window_top.round() as i32
        }
    };

    window.set_position(Position::Physical(PhysicalPosition::new(x, y)))?;

    Ok(())
}

fn hide_surplus_overlays(app: &tauri::AppHandle, active_count: usize) {
    for (label, window) in app.webview_windows() {
        if let Some(index) = overlay_index(&label) {
            if index >= active_count {
                let _ = window.hide();
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn refresh_topmost(window: &WebviewWindow) {
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        SetWindowPos, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW,
    };

    let Ok(handle) = window.window_handle() else {
        return;
    };

    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return;
    };

    unsafe {
        let _ = SetWindowPos(
            handle.hwnd.get() as _,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOACTIVATE | SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
        );
    }
}

#[cfg(not(target_os = "windows"))]
fn refresh_topmost(_window: &WebviewWindow) {}
