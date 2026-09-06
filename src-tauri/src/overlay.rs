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
    i18n,
    settings::{self, OverlayScreenMode, OverlayVariant},
};

const OVERLAY_LABEL_PREFIX: &str = "recording_overlay_";
const OVERLAY_SHADOW_LABEL_PREFIX: &str = "recording_shadow_";

/// Размеры карточки — должны совпадать с размерами `.overlay` в SCSS компонента.
/// Окно карточки равно им один в один, поэтому карточка не ловит клики за
/// пределами своей рамки.
const BOTTOM_CARD_WIDTH: f64 = 180.0;
const BOTTOM_CARD_HEIGHT: f64 = 40.0;
const CENTER_CARD_WIDTH: f64 = 220.0;
/// Высота центральной карточки зафиксирована в SCSS (`height`), а не выводится
/// из содержимого, чтобы окно совпадало с карточкой во всех состояниях.
const CENTER_CARD_HEIGHT: f64 = 200.0;

/// Прозрачное поле вокруг карточки в окне тени, достаточное, чтобы вместить
/// CSS-тень без обрезания её (прямоугольным) окном. Поле принадлежит отдельному
/// окну с `set_ignore_cursor_events`, поэтому клики сквозь него проходят и его
/// размер ничего не стоит.
///
/// Считается как `|offset| + 3σ`, где `σ = blur / 2`, по `box-shadow` из
/// `src/overlay/shadow.scss`: `0 10px 26px` даёт 49, `0 24px 56px` даёт 108.
/// Урезать поле нельзя — обрезанный хвост тени виден как ступенька по границе
/// окна (на белом фоне примерно #fcfcfc против #ffffff).
const BOTTOM_SHADOW_MARGIN: f64 = 52.0;
const CENTER_SHADOW_MARGIN: f64 = 112.0;

/// Расстояние от нижнего края экрана до нижнего края карточки варианта bottom.
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

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayShowPayload {
    state: &'static str,
    variant: OverlayVariant,
    /// Запись истории, которую нужно открыть из действий оверлея error/warning.
    /// `None` для обычных состояний recording/transcribing/processing.
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

/// Последнее запрошенное содержимое оверлея. Вновь созданные окна для каждого
/// монитора читают его при монтировании через `get_overlay_state`, поэтому они
/// корректно отрисовываются, даже если пропустили событие `show-overlay`,
/// разосланное до готовности их webview.
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

fn shadow_label(index: usize) -> String {
    format!("{OVERLAY_SHADOW_LABEL_PREFIX}{index}")
}

fn shadow_index(label: &str) -> Option<usize> {
    label
        .strip_prefix(OVERLAY_SHADOW_LABEL_PREFIX)
        .and_then(|rest| rest.parse::<usize>().ok())
}

pub fn create_recording_overlay(app: &tauri::AppHandle) -> AppResult<()> {
    // Прогреваем по одному webview каждого вида при запуске, чтобы первая
    // диктовка отображалась без задержки на инициализацию webview.
    // Дополнительные окна для каждого монитора создаются лениво.
    build_shadow_window(app, &shadow_label(0))?;
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

/// Скрывает оверлей по запросу. Используется кнопкой закрытия оверлея
/// error/warning и её таймером автоскрытия на фронтенде.
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

/// Показывает оверлей приостановленной записи: без свечей уровня микрофона,
/// с жёлтым акцентом.
pub fn show_paused_overlay(app: &tauri::AppHandle) -> AppResult<()> {
    show_overlay_state(app, "paused", None)
}

pub fn show_transcribing_overlay(app: &tauri::AppHandle) -> AppResult<()> {
    show_overlay_state(app, "transcribing", None)
}

/// Показывает оверлей локального определения речи перед отправкой аудио на STT.
pub fn show_vad_overlay(app: &tauri::AppHandle) -> AppResult<()> {
    show_overlay_state(app, "vad", None)
}

pub fn show_processing_overlay(app: &tauri::AppHandle) -> AppResult<()> {
    show_overlay_state(app, "processing", None)
}

/// Показывает красный оверлей ошибки (например, сбой распознавания речи).
/// `record_id` включает действие «открыть запись»; передайте `None` для сбоев
/// без сохранённой записи.
pub fn show_error_overlay(app: &tauri::AppHandle, record_id: Option<String>) -> AppResult<()> {
    show_overlay_state(app, "error", record_id)
}

/// Показывает жёлтый оверлей предупреждения (постобработка завершилась ошибкой,
/// но текст распознавания речи всё же был вставлен). `record_id` включает
/// действие «открыть запись».
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
        return Err(AppError::from(i18n::text(
            app,
            "overlay-no-monitor-available",
        )));
    }

    let payload = OverlayShowPayload {
        state,
        variant: variant.clone(),
        record_id,
    };

    // Сохраняем состояние до создания окон, чтобы любое поздно смонтированное
    // окно могло восстановить его через `get_overlay_state`.
    if let Ok(mut current) = current_overlay().lock() {
        *current = Some(payload.clone());
    }

    if is_notice_overlay_state(state) {
        arm_notice_auto_hide(app);
    } else {
        clear_notice_auto_hide(app);
    }

    for (index, monitor) in monitors.iter().enumerate() {
        let shadow_window = build_shadow_window(app, &shadow_label(index))?;
        let card_window = build_overlay_window(app, &overlay_label(index))?;

        position_overlay(&card_window, &shadow_window, monitor, &variant)?;

        // Порядок важен: оба окна topmost, и внутри этой группы наверху
        // оказывается поднятое последним. Карточка должна лежать над тенью.
        for window in [&shadow_window, &card_window] {
            window.show()?;
            window.set_always_on_top(true)?;
            refresh_topmost(window);
        }
    }

    // Скрываем окна оверлея для мониторов, которые больше не являются целевыми
    // (например, после переключения с «на всех экранах» на «у курсора» или
    // при отключении монитора).
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

/// Окно, которое рисует только тень карточки.
///
/// Тень живёт в отдельном окне, потому что окно карточки равно карточке и не
/// может нарисовать тень внутри себя, а нативная тень DWM не подчиняется
/// CSS-анимации и потому рассинхронизируется с появлением и скрытием карточки.
/// `set_ignore_cursor_events` снимает с прозрачного поля перехват мыши:
/// в отличие от окна карточки, это окно не имеет интерактивных элементов.
fn build_shadow_window(app: &tauri::AppHandle, label: &str) -> AppResult<WebviewWindow> {
    if let Some(window) = app.get_webview_window(label) {
        return Ok(window);
    }

    #[cfg_attr(not(all(debug_assertions, target_os = "windows")), allow(unused_mut))]
    let mut builder = WebviewWindowBuilder::new(
        app,
        label,
        WebviewUrl::App("src/overlay/shadow.html".into()),
    )
    .title("Recording Shadow")
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
    .focused(false);

    // browser_extensions_enabled должен совпадать у всех окон приложения — см.
    // комментарий в `build_overlay_window`.
    #[cfg(all(debug_assertions, target_os = "windows"))]
    {
        builder = builder.browser_extensions_enabled(true);

        let extensions_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/extensions");
        if std::path::Path::new(extensions_dir).exists() {
            builder = builder.extensions_path(extensions_dir);
        }
    }

    let window = builder.build()?;
    window.set_ignore_cursor_events(true)?;

    Ok(window)
}

/// Все окна оверлея — и карточки, и тени.
fn overlay_windows(app: &tauri::AppHandle) -> Vec<WebviewWindow> {
    app.webview_windows()
        .into_iter()
        .filter_map(|(label, window)| {
            (overlay_index(&label).is_some() || shadow_index(&label).is_some()).then_some(window)
        })
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

/// Раскладывает пару окон одного монитора: карточку и лежащее под ней окно тени.
fn position_overlay(
    card_window: &WebviewWindow,
    shadow_window: &WebviewWindow,
    monitor: &Monitor,
    variant: &OverlayVariant,
) -> AppResult<()> {
    let scale = monitor.scale_factor();
    let anchor_area = resolve_overlay_anchor_area(monitor, variant);
    let card = compute_overlay_card_frame(anchor_area, variant, scale);
    let shadow = compute_overlay_shadow_frame(card, variant, scale);

    place_window(shadow_window, shadow)?;
    place_window(card_window, card)?;

    Ok(())
}

fn place_window(window: &WebviewWindow, frame: PhysicalFrame) -> AppResult<()> {
    window.set_size(Size::Physical(PhysicalSize::new(frame.width, frame.height)))?;
    window.set_position(Position::Physical(PhysicalPosition::new(frame.x, frame.y)))?;

    Ok(())
}

fn monitor_bounds(monitor: &Monitor) -> PhysicalFrame {
    let position = monitor.position();
    let size = monitor.size();

    PhysicalFrame::new(position.x, position.y, size.width, size.height)
}

fn card_size(variant: &OverlayVariant) -> (f64, f64) {
    match variant {
        OverlayVariant::Bottom => (BOTTOM_CARD_WIDTH, BOTTOM_CARD_HEIGHT),
        OverlayVariant::Center => (CENTER_CARD_WIDTH, CENTER_CARD_HEIGHT),
    }
}

fn shadow_margin(variant: &OverlayVariant) -> f64 {
    match variant {
        OverlayVariant::Bottom => BOTTOM_SHADOW_MARGIN,
        OverlayVariant::Center => CENTER_SHADOW_MARGIN,
    }
}

/// Прямоугольник карточки в физических пикселях экрана. Окно карточки совпадает
/// с ним точно, поэтому позиционирование считается один раз и переиспользуется
/// окном тени.
fn compute_overlay_card_frame(
    anchor_area: PhysicalFrame,
    variant: &OverlayVariant,
    scale: f64,
) -> PhysicalFrame {
    let (card_width, card_height) = card_size(variant);
    let width = (card_width * scale).round();
    let height = (card_height * scale).round();

    let x = anchor_area.x + ((anchor_area.width as f64 - width) / 2.0).round() as i32;
    let y = match variant {
        OverlayVariant::Center => {
            anchor_area.y + ((anchor_area.height as f64 - height) / 2.0).round() as i32
        }
        OverlayVariant::Bottom => {
            (anchor_area.bottom() as f64 - OVERLAY_BOTTOM_OFFSET * scale - height).round() as i32
        }
    };

    PhysicalFrame::new(x, y, width as u32, height as u32)
}

/// Прямоугольник окна тени — карточка, расширенная прозрачным полем под CSS-тень.
fn compute_overlay_shadow_frame(
    card: PhysicalFrame,
    variant: &OverlayVariant,
    scale: f64,
) -> PhysicalFrame {
    let margin = (shadow_margin(variant) * scale).round() as u32;

    PhysicalFrame::new(
        card.x - margin as i32,
        card.y - margin as i32,
        card.width + margin * 2,
        card.height + margin * 2,
    )
}

fn resolve_overlay_anchor_area(monitor: &Monitor, variant: &OverlayVariant) -> PhysicalFrame {
    match variant {
        OverlayVariant::Bottom => resolve_bottom_overlay_anchor_area(monitor),
        OverlayVariant::Center => monitor_bounds(monitor),
    }
}

#[cfg(target_os = "windows")]
fn resolve_bottom_overlay_anchor_area(monitor: &Monitor) -> PhysicalFrame {
    // В Windows компактный оверлей должен следовать той же доступной рабочей
    // области, что и развёрнутое окно, поэтому автоскрытие/показ панели задач
    // меняет его точку привязки.
    resolve_monitor_work_area(monitor).unwrap_or_else(|| monitor_bounds(monitor))
}

#[cfg(not(target_os = "windows"))]
fn resolve_bottom_overlay_anchor_area(monitor: &Monitor) -> PhysicalFrame {
    // Tauri здесь не предоставляет кроссплатформенный доступ к рабочим областям
    // отдельных мониторов, поэтому сборки не для Windows используют полные
    // границы монитора.
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
        let index = overlay_index(&label).or_else(|| shadow_index(&label));

        if index.is_some_and(|index| index >= active_count) {
            let _ = window.hide();
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

        let card = compute_overlay_card_frame(anchor_area, &OverlayVariant::Bottom, 1.0);

        assert_eq!(card.x, 870);
        // 1040 (низ области) − 16 (OVERLAY_BOTTOM_OFFSET) − 40 (высота карточки).
        assert_eq!(card.y, 984);
        assert_eq!(card.bottom(), 1040 - OVERLAY_BOTTOM_OFFSET as i32);
    }

    #[test]
    fn bottom_overlay_moves_with_work_area_changes() {
        let tall_area = PhysicalFrame::new(0, 0, 1920, 1080);
        let short_area = PhysicalFrame::new(0, 0, 1920, 1040);

        let tall = compute_overlay_card_frame(tall_area, &OverlayVariant::Bottom, 1.0);
        let short = compute_overlay_card_frame(short_area, &OverlayVariant::Bottom, 1.0);

        assert_eq!(tall.y - short.y, 40);
    }

    #[test]
    fn bottom_overlay_centers_inside_available_area() {
        let anchor_area = PhysicalFrame::new(80, 0, 1840, 1040);

        let card = compute_overlay_card_frame(anchor_area, &OverlayVariant::Bottom, 1.0);

        assert_eq!(card.x, 910);
    }

    #[test]
    fn bottom_overlay_position_scales_in_physical_pixels() {
        let anchor_area = PhysicalFrame::new(0, 0, 2560, 1440);

        let card = compute_overlay_card_frame(anchor_area, &OverlayVariant::Bottom, 1.5);

        assert_eq!(card.x, 1145);
        // Те же слагаемые, что и при масштабе 1.0, умноженные на 1.5:
        // 1440 − 24 − 60.
        assert_eq!(card.y, 1356);
        assert_eq!(card.width, 270);
        assert_eq!(card.height, 60);
    }

    #[test]
    fn card_window_matches_card_size() {
        let anchor_area = PhysicalFrame::new(0, 0, 1920, 1040);

        let card = compute_overlay_card_frame(anchor_area, &OverlayVariant::Center, 1.0);

        // Тень живёт в отдельном окне, поэтому внутри окна карточки запаса нет.
        assert_eq!(card.width, CENTER_CARD_WIDTH as u32);
        assert_eq!(card.height, CENTER_CARD_HEIGHT as u32);
    }

    #[test]
    fn shadow_window_surrounds_card_evenly() {
        let anchor_area = PhysicalFrame::new(0, 0, 1920, 1040);
        let card = compute_overlay_card_frame(anchor_area, &OverlayVariant::Bottom, 1.0);

        let shadow = compute_overlay_shadow_frame(card, &OverlayVariant::Bottom, 1.0);

        let margin = BOTTOM_SHADOW_MARGIN as i32;
        assert_eq!(card.x - shadow.x, margin);
        assert_eq!(card.y - shadow.y, margin);
        assert_eq!(shadow.bottom() - card.bottom(), margin);
        assert_eq!(shadow.width - card.width, margin as u32 * 2);
    }

    #[test]
    fn shadow_margin_scales_with_the_monitor() {
        let anchor_area = PhysicalFrame::new(0, 0, 2560, 1440);
        let card = compute_overlay_card_frame(anchor_area, &OverlayVariant::Center, 1.5);

        let shadow = compute_overlay_shadow_frame(card, &OverlayVariant::Center, 1.5);

        assert_eq!(card.x - shadow.x, (CENTER_SHADOW_MARGIN * 1.5) as i32);
    }

    #[test]
    fn shadow_labels_do_not_collide_with_card_labels() {
        assert_eq!(shadow_index(&shadow_label(3)), Some(3));
        assert_eq!(overlay_index(&shadow_label(3)), None);
        assert_eq!(shadow_index(&overlay_label(3)), None);
    }
}
