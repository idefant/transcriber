use std::{thread, time::Duration};

use tauri::{
    Emitter, Manager, PhysicalPosition, Position, WebviewUrl, WebviewWindow, WebviewWindowBuilder,
};

use crate::error::{AppError, AppResult};

const OVERLAY_LABEL: &str = "recording_overlay";
const OVERLAY_WIDTH: f64 = 180.0;
const OVERLAY_HEIGHT: f64 = 40.0;
const OVERLAY_BOTTOM_OFFSET: i32 = 72;
const HIDE_DELAY_MS: u64 = 250;

pub fn create_recording_overlay(app: &tauri::AppHandle) -> AppResult<()> {
    if app.get_webview_window(OVERLAY_LABEL).is_some() {
        return Ok(());
    }

    WebviewWindowBuilder::new(
        app,
        OVERLAY_LABEL,
        WebviewUrl::App("src/overlay/index.html".into()),
    )
    .title("Recording")
    .inner_size(OVERLAY_WIDTH, OVERLAY_HEIGHT)
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .resizable(false)
    .visible(false)
    .focused(false)
    .accept_first_mouse(true)
    .build()?;

    update_overlay_position(app)?;

    Ok(())
}

pub fn show_recording_overlay(app: &tauri::AppHandle) -> AppResult<()> {
    show_overlay_state(app, "recording")
}

pub fn show_transcribing_overlay(app: &tauri::AppHandle) -> AppResult<()> {
    show_overlay_state(app, "transcribing")
}

pub fn show_processing_overlay(app: &tauri::AppHandle) -> AppResult<()> {
    show_overlay_state(app, "processing")
}

pub fn hide_recording_overlay(app: &tauri::AppHandle) -> AppResult<()> {
    let Some(window) = overlay_window(app) else {
        return Ok(());
    };

    let _ = window.emit("hide-overlay", ());
    let window_to_hide = window.clone();

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(HIDE_DELAY_MS));
        let _ = window_to_hide.hide();
    });

    Ok(())
}

pub fn update_overlay_position(app: &tauri::AppHandle) -> AppResult<()> {
    let Some(window) = overlay_window(app) else {
        return Ok(());
    };

    let monitor = match app.cursor_position() {
        Ok(position) => app
            .monitor_from_point(position.x, position.y)?
            .or(window.primary_monitor()?),
        Err(_) => window.primary_monitor()?,
    }
    .ok_or("No monitor is available for recording overlay")?;

    let monitor_position = monitor.position();
    let monitor_size = monitor.size();
    let x = monitor_position.x + ((monitor_size.width as f64 - OVERLAY_WIDTH) / 2.0).round() as i32;
    let y = monitor_position.y + monitor_size.height as i32
        - OVERLAY_HEIGHT as i32
        - OVERLAY_BOTTOM_OFFSET;

    window.set_position(Position::Physical(PhysicalPosition::new(x, y)))?;

    Ok(())
}

pub fn emit_mic_levels(app: &tauri::AppHandle, levels: Vec<f32>) {
    if let Some(window) = overlay_window(app) {
        let _ = window.emit("mic-level", levels);
    }
}

fn show_overlay_state(app: &tauri::AppHandle, state: &str) -> AppResult<()> {
    let Some(window) = overlay_window(app) else {
        return Err(AppError::from("Recording overlay window is not available"));
    };

    update_overlay_position(app)?;
    window.show()?;
    window.set_always_on_top(true)?;
    refresh_topmost(&window);
    window.emit("show-overlay", state)?;

    Ok(())
}

fn overlay_window(app: &tauri::AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window(OVERLAY_LABEL)
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
