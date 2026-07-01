use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicU64, Ordering},
        Mutex, OnceLock,
    },
    thread,
    time::{Duration, Instant},
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
const OVERLAY_BOTTOM_OFFSET: f64 = 16.0;

const HIDE_DELAY_MS: u64 = 250;
const NOTICE_AUTO_HIDE_DELAY: Duration = Duration::from_secs(5);
const NOTICE_LEAVE_HIDE_DELAY: Duration = Duration::from_secs(2);

static OVERLAY_VISIBILITY_EPOCH: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PhysicalFrame {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

impl PhysicalFrame {
    const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    fn bottom(self) -> i32 {
        self.y + self.height as i32
    }

    fn center_point(self) -> (i32, i32) {
        (
            self.x + (self.width / 2) as i32,
            self.y + (self.height / 2) as i32,
        )
    }
}

#[derive(Clone, Copy, Debug)]
struct OverlayWindowGeometry {
    card_height: f64,
    physical_width: f64,
    physical_height: f64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayShowPayload {
    state: &'static str,
    variant: OverlayVariant,
    /// History record to reveal from the error/warning overlay actions. `None`
    /// for the regular recording/transcribing/processing states.
    record_id: Option<String>,
}

#[derive(Default)]
pub struct OverlayNoticeRuntime(Mutex<NoticeAutoHideTracker>);

#[derive(Default)]
struct NoticeAutoHideTracker {
    next_generation: u64,
    session: Option<NoticeAutoHideSession>,
}

struct NoticeAutoHideSession {
    generation: u64,
    original_deadline: Instant,
    current_deadline: Option<Instant>,
    armed_windows: HashSet<String>,
    hovered_windows: HashSet<String>,
}

#[derive(Clone, Copy)]
struct ScheduledDismissal {
    deadline: Instant,
    generation: u64,
}

impl NoticeAutoHideTracker {
    fn show_notice(&mut self, now: Instant) -> ScheduledDismissal {
        self.next_generation += 1;

        let deadline = now + NOTICE_AUTO_HIDE_DELAY;
        let generation = self.next_generation;

        self.session = Some(NoticeAutoHideSession {
            generation,
            original_deadline: deadline,
            current_deadline: Some(deadline),
            armed_windows: HashSet::new(),
            hovered_windows: HashSet::new(),
        });

        ScheduledDismissal {
            deadline,
            generation,
        }
    }

    fn clear(&mut self) {
        self.session = None;
    }

    fn mouse_move(&mut self, label: &str, now: Instant) {
        let Some(session) = self.session.as_mut() else {
            return;
        };

        if now >= session.original_deadline {
            return;
        }

        session.armed_windows.insert(label.to_string());
        session.hovered_windows.insert(label.to_string());
        session.current_deadline = None;
    }

    fn mouse_leave(&mut self, label: &str, now: Instant) -> Option<ScheduledDismissal> {
        let session = self.session.as_mut()?;

        if !session.armed_windows.contains(label) {
            return None;
        }

        session.hovered_windows.remove(label);

        if !session.hovered_windows.is_empty() {
            return None;
        }

        let deadline = if now >= session.original_deadline {
            now
        } else {
            session.original_deadline.min(now + NOTICE_LEAVE_HIDE_DELAY)
        };

        session.current_deadline = Some(deadline);

        Some(ScheduledDismissal {
            deadline,
            generation: session.generation,
        })
    }

    fn should_dismiss(&self, generation: u64, deadline: Instant, now: Instant) -> bool {
        let Some(session) = self.session.as_ref() else {
            return false;
        };

        session.generation == generation
            && session.current_deadline == Some(deadline)
            && session.hovered_windows.is_empty()
            && now >= deadline
    }
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

#[tauri::command]
pub fn overlay_notice_mouse_move(app: tauri::AppHandle, window: tauri::WebviewWindow) {
    if let Ok(mut tracker) = app.state::<OverlayNoticeRuntime>().0.lock() {
        tracker.mouse_move(window.label(), Instant::now());
    }
}

#[tauri::command]
pub fn overlay_notice_mouse_leave(app: tauri::AppHandle, window: tauri::WebviewWindow) {
    let dismissal = app
        .state::<OverlayNoticeRuntime>()
        .0
        .lock()
        .ok()
        .and_then(|mut tracker| tracker.mouse_leave(window.label(), Instant::now()));

    if let Some(dismissal) = dismissal {
        schedule_notice_dismissal(app, dismissal);
    }
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

    let hide_epoch = OVERLAY_VISIBILITY_EPOCH.fetch_add(1, Ordering::Relaxed) + 1;
    clear_notice_auto_hide(app);

    let windows = overlay_windows(app);

    if windows.is_empty() {
        return Ok(());
    }

    let _ = app.emit("hide-overlay", ());

    thread::spawn(move || {
        thread::sleep(Duration::from_millis(HIDE_DELAY_MS));
        if OVERLAY_VISIBILITY_EPOCH.load(Ordering::Relaxed) != hide_epoch {
            return;
        }
        for window in windows {
            let _ = window.hide();
        }
    });

    Ok(())
}

pub fn emit_mic_level(app: &tauri::AppHandle, level: f32) {
    let _ = app.emit("mic-level", level);
}

fn show_overlay_state(
    app: &tauri::AppHandle,
    state: &'static str,
    record_id: Option<String>,
) -> AppResult<()> {
    OVERLAY_VISIBILITY_EPOCH.fetch_add(1, Ordering::Relaxed);

    let app_settings = settings::load_app_settings(app)?;
    let variant = app_settings.overlay_variant().clone();
    let screen_mode = app_settings.overlay_screen_mode().clone();

    let base = build_overlay_window(app, &overlay_label(0))?;
    let monitors = target_monitors(app, &base, &screen_mode)?;

    if monitors.is_empty() {
        return Err(AppError::from(
            "No monitor is available for recording overlay",
        ));
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

    if is_notice_overlay_state(state) {
        arm_notice_auto_hide(app);
    } else {
        clear_notice_auto_hide(app);
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

fn is_notice_overlay_state(state: &str) -> bool {
    state == "error" || state == "warning"
}

fn arm_notice_auto_hide(app: &tauri::AppHandle) {
    let dismissal = app
        .state::<OverlayNoticeRuntime>()
        .0
        .lock()
        .ok()
        .map(|mut tracker| tracker.show_notice(Instant::now()));

    if let Some(dismissal) = dismissal {
        schedule_notice_dismissal(app.clone(), dismissal);
    }
}

fn clear_notice_auto_hide(app: &tauri::AppHandle) {
    if let Ok(mut tracker) = app.state::<OverlayNoticeRuntime>().0.lock() {
        tracker.clear();
    }
}

fn schedule_notice_dismissal(app: tauri::AppHandle, dismissal: ScheduledDismissal) {
    thread::spawn(move || {
        thread::sleep(dismissal.deadline.saturating_duration_since(Instant::now()));

        let should_dismiss = app
            .state::<OverlayNoticeRuntime>()
            .0
            .lock()
            .map(|tracker| {
                tracker.should_dismiss(dismissal.generation, dismissal.deadline, Instant::now())
            })
            .unwrap_or(false);

        if should_dismiss {
            let _ = hide_recording_overlay(&app);
        }
    });
}

fn build_overlay_window(app: &tauri::AppHandle, label: &str) -> AppResult<WebviewWindow> {
    if let Some(window) = app.get_webview_window(label) {
        return Ok(window);
    }

    #[cfg_attr(not(all(debug_assertions, target_os = "windows")), allow(unused_mut))]
    let mut builder =
        WebviewWindowBuilder::new(app, label, WebviewUrl::App("src/overlay/index.html".into()))
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
    let geometry = compute_overlay_window_geometry(variant, scale);

    window.set_size(Size::Physical(PhysicalSize::new(
        geometry.physical_width as u32,
        geometry.physical_height as u32,
    )))?;

    let anchor_area = resolve_overlay_anchor_area(monitor, variant);
    let (x, y) = compute_overlay_position(anchor_area, variant, scale, geometry);

    window.set_position(Position::Physical(PhysicalPosition::new(x, y)))?;

    Ok(())
}

fn monitor_bounds(monitor: &Monitor) -> PhysicalFrame {
    let position = monitor.position();
    let size = monitor.size();

    PhysicalFrame::new(position.x, position.y, size.width, size.height)
}

fn compute_overlay_window_geometry(variant: &OverlayVariant, scale: f64) -> OverlayWindowGeometry {
    // The window is the card plus a transparent margin that holds the card's CSS
    // drop shadow (the card is centered inside the window via `place-items`).
    let (card_width, card_height, margin) = match variant {
        OverlayVariant::Bottom => (BOTTOM_CARD_WIDTH, BOTTOM_CARD_HEIGHT, BOTTOM_SHADOW_MARGIN),
        OverlayVariant::Center => (CENTER_CARD_WIDTH, CENTER_CARD_HEIGHT, CENTER_SHADOW_MARGIN),
    };

    OverlayWindowGeometry {
        card_height,
        physical_width: ((card_width + margin * 2.0) * scale).round(),
        physical_height: ((card_height + margin * 2.0) * scale).round(),
    }
}

fn compute_overlay_position(
    anchor_area: PhysicalFrame,
    variant: &OverlayVariant,
    scale: f64,
    geometry: OverlayWindowGeometry,
) -> (i32, i32) {
    let x =
        anchor_area.x + ((anchor_area.width as f64 - geometry.physical_width) / 2.0).round() as i32;
    let y = match variant {
        OverlayVariant::Center => {
            anchor_area.y
                + ((anchor_area.height as f64 - geometry.physical_height) / 2.0).round() as i32
        }
        OverlayVariant::Bottom => compute_bottom_overlay_top(anchor_area, scale, geometry),
    };

    (x, y)
}

fn compute_bottom_overlay_top(
    anchor_area: PhysicalFrame,
    scale: f64,
    geometry: OverlayWindowGeometry,
) -> i32 {
    let card_bottom = anchor_area.bottom() as f64 - OVERLAY_BOTTOM_OFFSET * scale;
    let window_top =
        card_bottom - (geometry.card_height * scale) / 2.0 - geometry.physical_height / 2.0;

    window_top.round() as i32
}

fn resolve_overlay_anchor_area(monitor: &Monitor, variant: &OverlayVariant) -> PhysicalFrame {
    match variant {
        OverlayVariant::Bottom => resolve_bottom_overlay_anchor_area(monitor),
        OverlayVariant::Center => monitor_bounds(monitor),
    }
}

#[cfg(target_os = "windows")]
fn resolve_bottom_overlay_anchor_area(monitor: &Monitor) -> PhysicalFrame {
    // On Windows the compact overlay should follow the same available work area
    // that a maximized window uses, so taskbar auto-hide/show changes its anchor.
    resolve_monitor_work_area(monitor).unwrap_or_else(|| monitor_bounds(monitor))
}

#[cfg(not(target_os = "windows"))]
fn resolve_bottom_overlay_anchor_area(monitor: &Monitor) -> PhysicalFrame {
    // Tauri does not expose per-monitor work areas cross-platform here, so
    // non-Windows builds fall back to the full monitor bounds.
    monitor_bounds(monitor)
}

#[cfg(target_os = "windows")]
fn resolve_monitor_work_area(monitor: &Monitor) -> Option<PhysicalFrame> {
    use windows_sys::Win32::{
        Foundation::POINT,
        Graphics::Gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST},
    };

    let bounds = monitor_bounds(monitor);
    let (center_x, center_y) = bounds.center_point();
    let handle = unsafe {
        MonitorFromPoint(
            POINT {
                x: center_x,
                y: center_y,
            },
            MONITOR_DEFAULTTONEAREST,
        )
    };

    if handle.is_null() {
        return None;
    }

    let mut monitor_info = unsafe { std::mem::zeroed::<MONITORINFO>() };
    monitor_info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;

    let result = unsafe { GetMonitorInfoW(handle, &mut monitor_info) };

    if result == 0 {
        return None;
    }

    Some(physical_frame_from_rect(monitor_info.rcWork))
}

#[cfg(target_os = "windows")]
fn physical_frame_from_rect(rect: windows_sys::Win32::Foundation::RECT) -> PhysicalFrame {
    let width = (rect.right - rect.left).max(0) as u32;
    let height = (rect.bottom - rect.top).max(0) as u32;

    PhysicalFrame::new(rect.left, rect.top, width, height)
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

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW_A: &str = "recording_overlay_0";
    const WINDOW_B: &str = "recording_overlay_1";

    #[test]
    fn notice_without_hover_closes_after_five_seconds() {
        let mut tracker = NoticeAutoHideTracker::default();
        let now = Instant::now();
        let dismissal = tracker.show_notice(now);

        assert!(!tracker.should_dismiss(
            dismissal.generation,
            dismissal.deadline,
            now + Duration::from_secs(4)
        ));
        assert!(tracker.should_dismiss(
            dismissal.generation,
            dismissal.deadline,
            dismissal.deadline
        ));
    }

    #[test]
    fn mouse_move_blocks_deadline_until_leave() {
        let mut tracker = NoticeAutoHideTracker::default();
        let now = Instant::now();
        let dismissal = tracker.show_notice(now);

        tracker.mouse_move(WINDOW_A, now + Duration::from_secs(1));

        assert!(!tracker.should_dismiss(
            dismissal.generation,
            dismissal.deadline,
            dismissal.deadline + Duration::from_millis(1),
        ));
    }

    #[test]
    fn mouse_leave_uses_two_seconds_when_it_is_earlier_than_original_deadline() {
        let mut tracker = NoticeAutoHideTracker::default();
        let now = Instant::now();

        tracker.show_notice(now);
        tracker.mouse_move(WINDOW_A, now + Duration::from_secs(1));

        let leave_time = now + Duration::from_secs(2);
        let dismissal = tracker
            .mouse_leave(WINDOW_A, leave_time)
            .expect("leave should schedule dismissal");

        assert_eq!(dismissal.deadline, leave_time + Duration::from_secs(2));
    }

    #[test]
    fn mouse_leave_near_end_keeps_original_deadline() {
        let mut tracker = NoticeAutoHideTracker::default();
        let now = Instant::now();

        let initial = tracker.show_notice(now);
        tracker.mouse_move(WINDOW_A, now + Duration::from_secs(1));

        let leave_time = now + Duration::from_millis(4_500);
        let dismissal = tracker
            .mouse_leave(WINDOW_A, leave_time)
            .expect("leave should schedule dismissal");

        assert_eq!(dismissal.deadline, initial.deadline);
    }

    #[test]
    fn mouse_leave_after_original_deadline_closes_immediately() {
        let mut tracker = NoticeAutoHideTracker::default();
        let now = Instant::now();

        tracker.show_notice(now);
        tracker.mouse_move(WINDOW_A, now + Duration::from_secs(1));

        let leave_time = now + Duration::from_secs(6);
        let dismissal = tracker
            .mouse_leave(WINDOW_A, leave_time)
            .expect("leave should schedule dismissal");

        assert_eq!(dismissal.deadline, leave_time);
        assert!(tracker.should_dismiss(dismissal.generation, dismissal.deadline, leave_time));
    }

    #[test]
    fn hover_on_one_window_holds_all_until_last_window_leaves() {
        let mut tracker = NoticeAutoHideTracker::default();
        let now = Instant::now();
        let initial = tracker.show_notice(now);

        tracker.mouse_move(WINDOW_A, now + Duration::from_secs(1));
        tracker.mouse_move(WINDOW_B, now + Duration::from_secs(2));

        assert!(tracker
            .mouse_leave(WINDOW_A, now + Duration::from_secs(3))
            .is_none());
        assert!(!tracker.should_dismiss(
            initial.generation,
            initial.deadline,
            initial.deadline + Duration::from_secs(1),
        ));

        let dismissal = tracker
            .mouse_leave(WINDOW_B, now + Duration::from_secs(4))
            .expect("last leave should schedule dismissal");

        assert_eq!(dismissal.deadline, initial.deadline);
    }

    #[test]
    fn mouse_move_after_return_cancels_pending_dismissal() {
        let mut tracker = NoticeAutoHideTracker::default();
        let now = Instant::now();

        tracker.show_notice(now);
        tracker.mouse_move(WINDOW_A, now + Duration::from_secs(1));

        let leave_time = now + Duration::from_secs(2);
        let dismissal = tracker
            .mouse_leave(WINDOW_A, leave_time)
            .expect("leave should schedule dismissal");

        tracker.mouse_move(WINDOW_A, leave_time + Duration::from_millis(500));

        assert!(!tracker.should_dismiss(
            dismissal.generation,
            dismissal.deadline,
            dismissal.deadline + Duration::from_millis(1),
        ));
    }

    #[test]
    fn bottom_overlay_uses_work_area_bottom_offset() {
        let anchor_area = PhysicalFrame::new(0, 0, 1920, 1040);
        let geometry = compute_overlay_window_geometry(&OverlayVariant::Bottom, 1.0);

        let (x, y) = compute_overlay_position(anchor_area, &OverlayVariant::Bottom, 1.0, geometry);

        assert_eq!(x, 834);
        assert_eq!(y, 892);
    }

    #[test]
    fn bottom_overlay_moves_with_work_area_changes() {
        let tall_area = PhysicalFrame::new(0, 0, 1920, 1080);
        let short_area = PhysicalFrame::new(0, 0, 1920, 1040);
        let geometry = compute_overlay_window_geometry(&OverlayVariant::Bottom, 1.0);

        let (_, tall_y) =
            compute_overlay_position(tall_area, &OverlayVariant::Bottom, 1.0, geometry);
        let (_, short_y) =
            compute_overlay_position(short_area, &OverlayVariant::Bottom, 1.0, geometry);

        assert_eq!(tall_y - short_y, 40);
    }

    #[test]
    fn bottom_overlay_centers_inside_available_area() {
        let anchor_area = PhysicalFrame::new(80, 0, 1840, 1040);
        let geometry = compute_overlay_window_geometry(&OverlayVariant::Bottom, 1.0);

        let (x, _) = compute_overlay_position(anchor_area, &OverlayVariant::Bottom, 1.0, geometry);

        assert_eq!(x, 874);
    }

    #[test]
    fn bottom_overlay_keeps_card_bottom_at_offset_despite_shadow_margin() {
        let anchor_area = PhysicalFrame::new(0, 0, 1920, 1040);
        let geometry = compute_overlay_window_geometry(&OverlayVariant::Bottom, 1.0);
        let top = compute_bottom_overlay_top(anchor_area, 1.0, geometry);
        let card_bottom = top as f64 + geometry.physical_height / 2.0 + geometry.card_height / 2.0;

        assert_eq!(card_bottom, 968.0);
    }

    #[test]
    fn bottom_overlay_position_scales_in_physical_pixels() {
        let anchor_area = PhysicalFrame::new(0, 0, 2560, 1440);
        let geometry = compute_overlay_window_geometry(&OverlayVariant::Bottom, 1.5);

        let (x, y) = compute_overlay_position(anchor_area, &OverlayVariant::Bottom, 1.5, geometry);

        assert_eq!(x, 1091);
        assert_eq!(y, 1218);
    }
}
