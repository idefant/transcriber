use std::sync::{
    atomic::{AtomicU64, Ordering},
    mpsc::{self, Sender},
    Mutex, OnceLock,
};

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::Serialize;
use tauri::{Emitter, Manager};
use uuid::Uuid;

use crate::{
    audio_mute::OutputMuteGuard,
    debug_log::{ModelRunLogContext, ModelRunSource},
    error::{AppError, AppResult},
    history, i18n, keyboard,
    media_control::{self, MediaPauseGuard},
    notification::{self, ConfigError, ConfigErrorSection},
    overlay,
    processing::load_processing_config,
    providers,
    recording::{self, PreparedRecorder, RecordedAudio},
    runner,
    settings::{self, RecordingAudioMode, TriggerMode},
    shortcut_hook::{self, ShortcutState},
};

#[derive(Default)]
pub struct DictationRuntime {
    session: Mutex<DictationSession>,
    active_hold_activation_id: Mutex<Option<u64>>,
    active_task: Mutex<Option<ActiveDictationTask>>,
    next_session_id: AtomicU64,
    /// Заранее собранный, приостановленный поток захвата, переиспользуемый между сессиями, чтобы
    /// диктовка запускалась без затрат на сборку WASAPI-потока на горячем пути.
    prepared_recorder: Mutex<Option<PreparedRecorder>>,
}

struct ActiveDictationTask {
    session_id: u64,
    handle: tauri::async_runtime::JoinHandle<()>,
}

/// Лёгкое состояние выполняющейся записи. Сам поток захвата
/// живёт в `DictationRuntime::prepared_recorder`; здесь хранятся только данные конкретной сессии
/// и guard приглушения системного звука, чтобы звук возвращался по завершении сессии.
struct RecordingHandle {
    started_at: DateTime<Utc>,
    /// Возвращает системный звук при drop. Пауза диктовки снимает guard, а продолжение
    /// записи берёт его заново, поэтому поле изменяемое, а не только ради Drop.
    audio_guard: Option<RecordingAudioGuard>,
}

/// Как приглушён системный звук на время записи; drop возвращает всё как было.
///
/// Варианты взаимоисключающие: их выбирает настройка режима звука при записи. Guard'ы
/// хранятся ради побочного эффекта Drop, читать их не нужно — отсюда префикс `_`.
enum RecordingAudioGuard {
    MediaPause { _guard: MediaPauseGuard },
    Mute { _guard: OutputMuteGuard },
}

#[derive(Default)]
enum DictationSession {
    #[default]
    Idle,
    Recording {
        id: u64,
        handle: RecordingHandle,
        /// Запись приостановлена хоткеем паузы: поток захвата стоит на паузе,
        /// но уже накопленные сэмплы сохраняются до продолжения или остановки.
        is_paused: bool,
    },
    Transcribing {
        id: u64,
    },
    Processing {
        id: u64,
    },
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DictationErrorPayload {
    message: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DictationSessionPayload {
    active: bool,
    session_id: Option<u64>,
    /// `true`, пока идёт запись с микрофона (в том числе на паузе). Хоткей отмены
    /// действует всю сессию, а хоткей паузы — только во время записи, поэтому
    /// обработчику DOM мало одного флага `active`: он бы перехватывал паузу и во
    /// время распознавания.
    is_recording: bool,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DictationShortcutPayload {
    activation_id: u64,
}

/// Результат обработки записи, используется, чтобы решить, как завершится оверлей.
enum DictationOutcome {
    /// Успех (текст вставлен) либо отменённая/заменённая сессия — скрыть оверлей.
    Completed,
    /// Распознавание речи не удалось; текст не вставлен. Показать красный оверлей ошибки.
    /// `record_id` равен `Some`, если запись истории была сохранена.
    SttError { record_id: Option<String> },
    /// Постобработка не удалась, но текст распознавания речи был вставлен. Показать
    /// янтарный оверлей предупреждения, связанный с сохранённой записью.
    PostProcessError { record_id: String },
}

pub fn register_dictation_shortcut(app: &tauri::AppHandle) -> AppResult<()> {
    ensure_dictation_dispatch_thread(app);

    let settings = settings::load_app_settings(app)?;

    shortcut_hook::install_dictation_shortcut(app.clone(), settings.hotkey())?;
    shortcut_hook::set_copy_latest_hotkey(settings.copy_latest_hotkey())?;
    shortcut_hook::set_paste_latest_hotkey(settings.paste_latest_hotkey())?;
    shortcut_hook::set_repeat_latest_hotkey(settings.repeat_latest_hotkey())?;

    Ok(())
}

/// Задача, передаваемая из DOM-команды (главный поток) выделенному
/// потоку диспетчеризации диктовки. Отражает `shortcut_hook::HookEvent`: путь через нативный
/// хук уже выполняет ту же работу вне главного потока, а это даёт
/// пути через DOM сфокусированного окна ту же гарантию.
enum DictationJob {
    DomPressed { activation_id: u64 },
    DomReleased { activation_id: u64 },
    Cancel { session_id: Option<u64> },
    TogglePause { session_id: Option<u64> },
}

static DICTATION_JOB_SENDER: OnceLock<Sender<DictationJob>> = OnceLock::new();

/// Сериализованный воркер, выполняющий вне главного потока действия диктовки, инициированные
/// из DOM. Обработчики `#[tauri::command] pub fn` (не-async) выполняются в главном
/// потоке цикла событий, а запуск и остановка диктовки — это медленная блокирующая работа
/// (создание окна оверлея, сборка WASAPI-потока, вызовы COM audio-endpoint) —
/// её выполнение там замораживает перетаскивание окна и кнопки и может привести к deadlock
/// STA-потока цикла событий WebView2 при взаимодействии с COM marshaling. Единственный поток,
/// вычитывающий mpsc-канал, гарантирует строгий порядок: `pressed` всегда перед
/// `released`, точно так же, как это делает `shortcut_hook::ensure_event_dispatch_thread` для
/// пути через нативный хук.
fn ensure_dictation_dispatch_thread(app: &tauri::AppHandle) {
    if DICTATION_JOB_SENDER.get().is_some() {
        return;
    }

    let (sender, receiver) = mpsc::channel::<DictationJob>();

    if DICTATION_JOB_SENDER.set(sender).is_err() {
        // Другой поток выиграл гонку за инициализацию; его воркер уже выполняется.
        return;
    }

    let app = app.clone();

    std::thread::spawn(move || {
        for job in receiver {
            match job {
                DictationJob::DomPressed { activation_id } => {
                    handle_dom_shortcut_pressed(&app, DictationShortcutPayload { activation_id });
                }
                DictationJob::DomReleased { activation_id } => {
                    handle_dom_shortcut_released(&app, DictationShortcutPayload { activation_id });
                }
                DictationJob::Cancel { session_id } => {
                    if let Err(error) = cancel_dictation_inner(app.clone(), session_id) {
                        emit_dictation_error(&app, error.into_message());
                    }
                }
                DictationJob::TogglePause { session_id } => {
                    toggle_pause(&app, session_id);
                }
            }
        }
    });
}

fn enqueue_dictation_job(app: &tauri::AppHandle, job: DictationJob) {
    ensure_dictation_dispatch_thread(app);

    if let Some(sender) = DICTATION_JOB_SENDER.get() {
        let _ = sender.send(job);
    }
}

pub fn update_dictation_shortcut(app: &tauri::AppHandle) -> AppResult<()> {
    let settings = settings::load_app_settings(app)?;

    shortcut_hook::set_dictation_hotkey(settings.hotkey())?;
    shortcut_hook::set_copy_latest_hotkey(settings.copy_latest_hotkey())?;
    shortcut_hook::set_paste_latest_hotkey(settings.paste_latest_hotkey())?;
    shortcut_hook::set_repeat_latest_hotkey(settings.repeat_latest_hotkey())
}

pub fn handle_shortcut_event(app: &tauri::AppHandle, state: ShortcutState) {
    let Ok(settings) = settings::load_app_settings(app) else {
        return;
    };

    match (settings.trigger_mode(), state) {
        (TriggerMode::Hold, ShortcutState::Pressed) => {
            start_dictation(app.clone(), None);
        }
        (TriggerMode::Hold, ShortcutState::Released) => {
            stop_dictation(app.clone(), None);
        }
        (TriggerMode::Press, ShortcutState::Pressed) => {
            toggle_dictation(app.clone());
        }
        _ => {}
    }
}

pub fn handle_cancel_shortcut(app: &tauri::AppHandle) {
    if let Err(error) = cancel_dictation_inner(app.clone(), None) {
        emit_dictation_error(app, error.into_message());
    }
}

pub fn handle_pause_shortcut(app: &tauri::AppHandle) {
    toggle_pause(app, None);
}

pub fn handle_paste_latest_shortcut(app: &tauri::AppHandle) {
    let app = app.clone();

    tauri::async_runtime::spawn(async move {
        if let Err(error) = paste_latest_history_text_inner(&app).await {
            emit_dictation_error(&app, error.into_message());
        }
    });
}

pub fn handle_copy_latest_shortcut(app: &tauri::AppHandle) {
    if let Err(error) = copy_latest_history_text_to_clipboard(app) {
        emit_dictation_error(app, error.into_message());
    }
}

pub fn handle_repeat_latest_shortcut(app: &tauri::AppHandle) {
    let app = app.clone();

    tauri::async_runtime::spawn(async move {
        if let Err(error) = repeat_latest_history_record_inner(app.clone()).await {
            emit_dictation_error(&app, error.into_message());
        }
    });
}

// Эти четыре DOM-команды намеренно синхронные и лишь ставят задачу в очередь —
// см. `ensure_dictation_dispatch_thread`, почему фактическая
// работа не должна выполняться в главном потоке.

#[tauri::command]
pub fn cancel_dictation(app: tauri::AppHandle, session_id: Option<u64>) {
    enqueue_dictation_job(&app, DictationJob::Cancel { session_id });
}

#[tauri::command]
pub fn toggle_pause_dictation(app: tauri::AppHandle, session_id: Option<u64>) {
    enqueue_dictation_job(&app, DictationJob::TogglePause { session_id });
}

#[tauri::command]
pub fn dictation_shortcut_pressed(app: tauri::AppHandle, activation_id: u64) {
    enqueue_dictation_job(&app, DictationJob::DomPressed { activation_id });
}

#[tauri::command]
pub fn dictation_shortcut_released(app: tauri::AppHandle, activation_id: u64) {
    enqueue_dictation_job(&app, DictationJob::DomReleased { activation_id });
}

#[tauri::command]
pub async fn paste_latest_history_text(app: tauri::AppHandle) -> Result<(), String> {
    paste_latest_history_text_inner(&app)
        .await
        .map_err(AppError::into_message)
}

#[tauri::command]
pub fn copy_latest_history_text(app: tauri::AppHandle) -> Result<(), String> {
    copy_latest_history_text_to_clipboard(&app).map_err(AppError::into_message)
}

#[tauri::command]
pub async fn repeat_latest_history_record(app: tauri::AppHandle) -> Result<(), String> {
    repeat_latest_history_record_inner(app)
        .await
        .map_err(AppError::into_message)
}

fn toggle_dictation(app: tauri::AppHandle) {
    let is_recording = app
        .state::<DictationRuntime>()
        .session
        .lock()
        .map(|session| matches!(*session, DictationSession::Recording { .. }))
        .unwrap_or(false);

    if is_recording {
        stop_dictation(app, None);
    } else {
        start_dictation(app, None);
    }
}

fn toggle_pause(app: &tauri::AppHandle, expected_session_id: Option<u64>) {
    if let Err(error) = toggle_pause_inner(app, expected_session_id) {
        emit_dictation_error(app, error.into_message());
    }
}

/// Переключает паузу текущей записи. Один и тот же хоткей ставит на паузу и
/// продолжает запись; вне состояния `Recording` вызов ничего не делает, поэтому
/// флаг паузы не может пережить отправку или отмену сессии.
fn toggle_pause_inner(app: &tauri::AppHandle, expected_session_id: Option<u64>) -> AppResult<()> {
    // Настройки читаем один раз: они нужны и для режима запуска, и для режима звука
    // на пути продолжения записи.
    let app_settings = settings::load_app_settings(app)?;

    // В режиме удержания запись живёт ровно столько, сколько зажат хоткей, поэтому
    // пауза там недоступна, а поле хоткея паузы в настройках заблокировано.
    if matches!(app_settings.trigger_mode(), TriggerMode::Hold) {
        return Ok(());
    }

    let runtime = app.state::<DictationRuntime>();
    let mut session = runtime
        .session
        .lock()
        .map_err(|_| AppError::from(i18n::text(app, "dictation-state-lock-failed")))?;

    let DictationSession::Recording {
        id,
        handle,
        is_paused,
    } = &mut *session
    else {
        return Ok(());
    };

    // Запоздавшее нажатие из прошлой сессии не должно ставить на паузу текущую.
    if expected_session_id.is_some_and(|expected_id| expected_id != *id) {
        return Ok(());
    }

    let should_pause = !*is_paused;

    if should_pause {
        pause_recording(app);

        // Звук возвращаем после освобождения микрофона, а не до, — тот же порядок,
        // что и при остановке записи. Если возвращать не просили, guard держит звук
        // приглушённым и на паузе.
        if app_settings.is_restore_audio_while_paused_enabled() {
            drop(handle.audio_guard.take());
        }
    } else if handle.audio_guard.is_some() {
        // Звук на паузе не возвращали: режим уже применён, ждать нечего.
        resume_recording(app)?;
    } else {
        // Продолжение идёт тем же путём, что и старт: сначала останавливаем медиа
        // (с ожиданием), потом возобновляем захват, и только потом заглушаем вывод.
        let media_pause_guard = acquire_media_pause(app_settings.recording_audio_mode());
        resume_recording(app)?;

        handle.audio_guard = match media_pause_guard {
            Some(guard) => Some(RecordingAudioGuard::MediaPause { _guard: guard }),
            None => acquire_output_mute(app_settings.recording_audio_mode())
                .map(|guard| RecordingAudioGuard::Mute { _guard: guard }),
        };
    }

    *is_paused = should_pause;
    drop(session);

    if should_pause {
        overlay::show_paused_overlay(app)
    } else {
        overlay::show_recording_overlay(app)
    }
}

fn handle_dom_shortcut_pressed(app: &tauri::AppHandle, payload: DictationShortcutPayload) {
    let Ok(settings) = settings::load_app_settings(app) else {
        return;
    };

    match settings.trigger_mode() {
        TriggerMode::Hold => start_dictation(app.clone(), Some(payload.activation_id)),
        TriggerMode::Press => toggle_dictation(app.clone()),
    }
}

fn handle_dom_shortcut_released(app: &tauri::AppHandle, payload: DictationShortcutPayload) {
    let Ok(settings) = settings::load_app_settings(app) else {
        return;
    };

    if matches!(settings.trigger_mode(), TriggerMode::Hold) {
        stop_dictation(app.clone(), Some(payload.activation_id));
    }
}

async fn paste_latest_history_text_inner(app: &tauri::AppHandle) -> AppResult<()> {
    if !is_session_idle(app) {
        return Ok(());
    }

    let text = history::latest_history_text(app)?;
    let paste_hotkey = settings::load_app_settings(app)?
        .paste_latest_hotkey()
        .to_string();

    if text.trim().is_empty() {
        return Ok(());
    }

    shortcut_hook::wait_for_hotkey_release(&paste_hotkey).await?;

    if !is_session_idle(app) {
        return Ok(());
    }

    keyboard::paste_text(app, &text).await
}

pub fn copy_latest_history_text_to_clipboard(app: &tauri::AppHandle) -> AppResult<()> {
    let text = history::latest_history_text(app)?;

    if text.trim().is_empty() {
        return Ok(());
    }

    keyboard::copy_text(app, &text)
}

fn start_dictation(app: tauri::AppHandle, activation_id: Option<u64>) {
    if let Err(error) = start_dictation_inner(&app, activation_id) {
        emit_dictation_error(&app, error.into_message());
        let _ = overlay::hide_recording_overlay(&app);
    }
}

fn start_dictation_inner(app: &tauri::AppHandle, activation_id: Option<u64>) -> AppResult<()> {
    let runtime = app.state::<DictationRuntime>();
    let mut session = runtime
        .session
        .lock()
        .map_err(|_| AppError::from(i18n::text(app, "dictation-state-lock-failed")))?;

    if !matches!(*session, DictationSession::Idle) {
        return Ok(());
    }

    // Проверяем готовность конфигурации ДО записи. Если обработать аудио нельзя
    // (не выбран провайдер/модель, модель несовместима с провайдером, нет
    // API-ключа), запись не начинаем: сессия остаётся Idle, оверлей записи не
    // показывается, а пользователь получает системное уведомление. Возвращаем
    // Ok, чтобы не сработала ветка ошибки в start_dictation (она бы показала
    // dictation-error и попыталась скрыть несуществующий оверлей).
    if let Err(config_error) = validate_processing_ready(app) {
        drop(session);
        notification::show_config_error(app, &config_error);
        return Ok(());
    }

    // Читаем настройки приложения один раз здесь и переиспользуем их и для режима
    // звука, и для хоткеев отмены и паузы, вместо многократной загрузки с диска.
    let app_settings = settings::load_app_settings(app).ok();
    let audio_mode = app_settings
        .as_ref()
        .map_or_else(RecordingAudioMode::default, |settings| {
            settings.recording_audio_mode().clone()
        });

    // Медиа ставим на паузу ДО захвата и дожидаемся остановки: иначе музыка попадёт
    // в первые сотни миллисекунд записи. Оверлей на это время не показываем — запись
    // ещё не идёт, и он бы врал.
    let media_pause_guard = acquire_media_pause(&audio_mode);

    overlay::show_recording_overlay(app)?;

    // Запускаем захват на заранее прогретом потоке. Сборка потока — дорогой
    // шаг WASAPI — уже выполнена заранее, так что это всего лишь дешёвый
    // вызов `play()`, и звук начинает поступать почти мгновенно.
    let started_at = begin_recording(app)?;

    // Заглушаем вывод по умолчанию ПОСЛЕ начала захвата, чтобы держать затраты
    // на COM/заглушение вне пути перед захватом. Спецификация допускает продолжение
    // записи, даже если приглушить звук не удалось, поэтому здесь достаточно guard'а
    // по принципу best-effort.
    let audio_guard = match media_pause_guard {
        Some(guard) => Some(RecordingAudioGuard::MediaPause { _guard: guard }),
        None => acquire_output_mute(&audio_mode)
            .map(|guard| RecordingAudioGuard::Mute { _guard: guard }),
    };

    let id = runtime.next_session_id.fetch_add(1, Ordering::Relaxed) + 1;

    *session = DictationSession::Recording {
        id,
        handle: RecordingHandle {
            started_at,
            audio_guard,
        },
        is_paused: false,
    };
    drop(session);

    set_active_hold_activation_id(app, activation_id);

    // Активируем хоткей отмены на время этой сессии. Отсутствующий или
    // пустой хоткей отмены молча деактивирует хук (ни одна клавиша не перехватывается).
    if let Some(app_settings) = app_settings.as_ref() {
        if let Err(error) = shortcut_hook::arm_cancel_hotkey(app_settings.cancel_hotkey()) {
            emit_dictation_error(app, error.into_message());
        }

        // Хоткей паузы активен только на время записи и только в режиме «по нажатию»:
        // в режиме удержания пустая строка деактивирует его так же, как незаданный хоткей.
        let pause_hotkey = match app_settings.trigger_mode() {
            TriggerMode::Press => app_settings.pause_hotkey(),
            TriggerMode::Hold => "",
        };

        if let Err(error) = shortcut_hook::arm_pause_hotkey(pause_hotkey) {
            emit_dictation_error(app, error.into_message());
        }
    }

    // Уведомляем фронтенд, что сессия теперь активна (используется, чтобы разрешить хоткеи
    // отмены и паузы внутри приложения).
    emit_dictation_session(app, true, Some(id), true);

    Ok(())
}

/// Убеждается, что для текущего устройства ввода по умолчанию есть подготовленный поток захвата,
/// и запускает его. Возвращает временную метку начала записи. Дорогая сборка потока
/// выполняется здесь только если нет прогретого потока или устройство ввода
/// по умолчанию изменилось с момента его сборки.
fn begin_recording(app: &tauri::AppHandle) -> AppResult<DateTime<Utc>> {
    let runtime = app.state::<DictationRuntime>();
    let mut prepared = runtime
        .prepared_recorder
        .lock()
        .map_err(|_| AppError::from(i18n::text(app, "dictation-state-lock-failed")))?;

    let needs_build = match prepared.as_ref() {
        Some(recorder) => !recorder.is_for_current_default_device(),
        None => true,
    };

    if needs_build {
        *prepared = Some(recording::prepare_recorder(app)?);
    }

    prepared
        .as_ref()
        .expect("prepared recorder was just ensured")
        .start(app)?;

    Ok(Utc::now())
}

/// Заглушение устройства вывода по умолчанию по принципу best-effort. Возвращает guard, который включает звук обратно
/// при drop, либо `None`, если режим звука другой или заглушить не удалось.
fn acquire_output_mute(mode: &RecordingAudioMode) -> Option<OutputMuteGuard> {
    if !matches!(mode, RecordingAudioMode::Mute) {
        return None;
    }

    match OutputMuteGuard::new() {
        Ok(guard) => Some(guard),
        Err(error) => {
            eprintln!("Failed to mute system audio: {error}");
            None
        }
    }
}

/// Ставит системное медиа на паузу и ждёт остановки, если этого требует режим звука.
/// Возвращает guard, который возобновляет воспроизведение при drop.
///
/// Ожидание проходит без оверлея: пока оно идёт, запись ещё не началась, а на паузе
/// диктовки оверлей остаётся в состоянии `Paused`.
fn acquire_media_pause(mode: &RecordingAudioMode) -> Option<MediaPauseGuard> {
    if !matches!(mode, RecordingAudioMode::Pause) {
        return None;
    }

    media_control::pause_sessions(&media_control::playing_sessions())
}

/// Собирает переиспользуемый поток захвата ещё до первой диктовки, чтобы затраты на сборку
/// WASAPI были оплачены при запуске, а не при первом нажатии хоткея. Выполняется в
/// фоновом потоке; ошибка не является фатальной — поток будет собран по требованию.
pub fn prewarm_recorder(app: &tauri::AppHandle) {
    let app = app.clone();

    std::thread::spawn(move || match recording::prepare_recorder(&app) {
        Ok(recorder) => {
            if let Ok(mut prepared) = app.state::<DictationRuntime>().prepared_recorder.lock() {
                // Не затираем recorder, который уже успела собрать очень ранняя диктовка.
                if prepared.is_none() {
                    *prepared = Some(recorder);
                }
            }
        }
        Err(error) => {
            eprintln!("Recorder prewarm failed: {}", error.into_message());
        }
    });
}

/// Приостанавливает захват на время паузы, сохраняя накопленные сэмплы.
/// Порядок блокировок session -> prepared_recorder такой же, как в
/// `start_dictation_inner` и `cancel_dictation_inner`.
fn pause_recording(app: &tauri::AppHandle) {
    if let Ok(prepared) = app.state::<DictationRuntime>().prepared_recorder.lock() {
        if let Some(recorder) = prepared.as_ref() {
            recorder.pause();
        }
    }
}

/// Продолжает приостановленный захват. Ошибка оставляет сессию на паузе, чтобы
/// состояние UI не разошлось с реальным состоянием потока захвата.
fn resume_recording(app: &tauri::AppHandle) -> AppResult<()> {
    let runtime = app.state::<DictationRuntime>();
    let prepared = runtime
        .prepared_recorder
        .lock()
        .map_err(|_| AppError::from(i18n::text(app, "dictation-state-lock-failed")))?;

    let Some(recorder) = prepared.as_ref() else {
        return Err(AppError::from(i18n::text(
            app,
            "recording-no-audio-captured",
        )));
    };

    recorder.resume(app)
}

/// Приостанавливает и очищает подготовленный recorder без формирования аудио (используется при отмене
/// во время записи). Recorder остаётся готовым к повторному использованию.
fn release_recording(app: &tauri::AppHandle) {
    if let Ok(prepared) = app.state::<DictationRuntime>().prepared_recorder.lock() {
        if let Some(recorder) = prepared.as_ref() {
            recorder.abort();
        }
    }
}

/// Останавливает подготовленный recorder и кодирует захваченное аудио. Drop `handle`
/// возвращает системный звук после того, как микрофон освобождён; если запись
/// останавливают с паузы, звук уже вернула сама пауза, и drop ничего не делает.
fn finish_recording(app: &tauri::AppHandle, handle: RecordingHandle) -> AppResult<RecordedAudio> {
    let runtime = app.state::<DictationRuntime>();
    let audio = {
        let prepared = runtime
            .prepared_recorder
            .lock()
            .map_err(|_| AppError::from(i18n::text(app, "dictation-state-lock-failed")))?;

        let Some(recorder) = prepared.as_ref() else {
            return Err(AppError::from(i18n::text(
                app,
                "recording-no-audio-captured",
            )));
        };

        recorder.stop_to_audio(app, handle.started_at)?
    };

    drop(handle); // возвращаем звук теперь, когда микрофон освобождён
    Ok(audio)
}

/// Проверяет, что аудио можно будет обработать: выбраны провайдер и модель STT,
/// модель есть в каталоге и совместима с провайдером, у провайдера задан
/// API-ключ. Если включена постобработка, те же проверки выполняются для неё.
/// Переиспользует `build_stt_snapshot` / `build_post_process_snapshot` (чистая
/// проверка конфигурации без сетевых запросов) и `resolve_provider_api_key`.
fn validate_processing_ready(app: &tauri::AppHandle) -> Result<(), ConfigError> {
    let stt = runner::build_stt_snapshot(app).map_err(|error| ConfigError {
        section: ConfigErrorSection::SpeechToText,
        message: error.into_message(),
    })?;
    runner::ensure_stt_prompt_within_limit(app, &stt).map_err(|error| ConfigError {
        section: ConfigErrorSection::Dictionary,
        message: error.into_message(),
    })?;
    providers::resolve_provider_api_key(app, &stt.provider.provider_id).map_err(|error| {
        ConfigError {
            section: ConfigErrorSection::SpeechToText,
            message: error.into_message(),
        }
    })?;

    let config = load_processing_config(app).map_err(|error| ConfigError {
        section: ConfigErrorSection::SpeechToText,
        message: error.into_message(),
    })?;

    if config.post_process.enabled {
        let post = runner::build_post_process_snapshot(app).map_err(|error| ConfigError {
            section: ConfigErrorSection::PostProcessing,
            message: error.into_message(),
        })?;
        providers::resolve_provider_api_key(app, &post.provider.provider_id).map_err(|error| {
            ConfigError {
                section: ConfigErrorSection::PostProcessing,
                message: error.into_message(),
            }
        })?;
    }

    Ok(())
}

async fn repeat_latest_history_record_inner(app: tauri::AppHandle) -> AppResult<()> {
    let Some((id, record_id)) = begin_repeat_latest_history_record(&app)? else {
        return Ok(());
    };

    let handle = tauri::async_runtime::spawn(process_repeat_latest_history_record(
        app.clone(),
        id,
        record_id,
    ));
    register_active_task(&app, id, handle);

    Ok(())
}

fn begin_repeat_latest_history_record(app: &tauri::AppHandle) -> AppResult<Option<(u64, String)>> {
    let Some(record_id) = history::latest_history_record_id(app)? else {
        return Ok(None);
    };

    let runtime = app.state::<DictationRuntime>();
    let mut session = runtime
        .session
        .lock()
        .map_err(|_| AppError::from(i18n::text(app, "dictation-state-lock-failed")))?;

    if !matches!(*session, DictationSession::Idle) {
        return Ok(None);
    }

    // Как и при обычной диктовке, проверяем готовность конфигурации до показа
    // оверлея. Если обработать аудио нельзя, повтор не запускаем: оверлей не
    // показывается, а пользователь получает системное уведомление.
    if let Err(config_error) = validate_processing_ready(app) {
        drop(session);
        notification::show_config_error(app, &config_error);
        return Ok(None);
    }

    overlay::show_transcribing_overlay(app)?;

    let id = runtime.next_session_id.fetch_add(1, Ordering::Relaxed) + 1;
    *session = DictationSession::Transcribing { id };

    if let Ok(app_settings) = settings::load_app_settings(app) {
        if let Err(error) = shortcut_hook::arm_cancel_hotkey(app_settings.cancel_hotkey()) {
            emit_dictation_error(app, error.into_message());
        }
    }

    clear_active_hold_activation_id(app);
    emit_dictation_session(app, true, Some(id), false);

    Ok(Some((id, record_id)))
}

fn stop_dictation(app: tauri::AppHandle, activation_id: Option<u64>) {
    let (id, recording_handle) = match take_recording(&app, activation_id) {
        Ok(Some((id, recording_handle))) => (id, recording_handle),
        Ok(None) => return,
        Err(error) => {
            emit_dictation_error(&app, error.into_message());
            return;
        }
    };

    // Запись закончилась: пауза больше не применима, поэтому её хоткей снимается с
    // взвода и снова доходит до других приложений (в отличие от хоткея отмены,
    // который действует до конца постобработки).
    shortcut_hook::disarm_pause_hotkey();
    emit_dictation_session(&app, true, Some(id), false);

    // Останавливаем аудиопоток синхронно, чтобы индикатор микрофона ОС погас,
    // а системный звук включился обратно до начала STT/постобработки.
    let audio = match finish_recording(&app, recording_handle) {
        Ok(audio) => audio,
        Err(error) => {
            let _ = reset_session(&app, id, true);
            emit_dictation_error(&app, error.into_message());
            return;
        }
    };

    let handle = tauri::async_runtime::spawn(process_recording(app.clone(), id, audio));
    register_active_task(&app, id, handle);
}

fn take_recording(
    app: &tauri::AppHandle,
    activation_id: Option<u64>,
) -> AppResult<Option<(u64, RecordingHandle)>> {
    let runtime = app.state::<DictationRuntime>();
    let mut session = runtime
        .session
        .lock()
        .map_err(|_| AppError::from(i18n::text(app, "dictation-state-lock-failed")))?;
    let mut active_hold_activation_id = runtime.active_hold_activation_id.lock().map_err(|_| {
        AppError::from(i18n::text(
            app,
            "dictation-active-hold-shortcut-state-lock-failed",
        ))
    })?;

    if activation_id.is_some() && *active_hold_activation_id != activation_id {
        return Ok(None);
    }

    // Проверка перед replace: std::mem::replace безусловно записывает Idle, поэтому
    // перед вызовом нужно убедиться, что состояние — Recording, иначе случайный
    // stop_dictation (например, отпускание хоткея во время транскрибации) повредил бы
    // состояние и не дал бы reset_session когда-либо скрыть оверлей.
    if !matches!(*session, DictationSession::Recording { .. }) {
        return Ok(None);
    }

    let DictationSession::Recording { id, handle, .. } =
        std::mem::replace(&mut *session, DictationSession::Idle)
    else {
        unreachable!()
    };

    *session = DictationSession::Transcribing { id };
    *active_hold_activation_id = None;

    Ok(Some((id, handle)))
}

async fn process_recording(app: tauri::AppHandle, id: u64, audio: RecordedAudio) {
    let outcome = process_recording_inner(&app, id, audio).await;
    clear_active_task(&app, id);

    // Отменённая или заменённая сессия уже скрыла свой оверлей через
    // cancel_dictation_inner; нужно лишь убедиться, что состояние сессии сброшено.
    if !is_current_session(&app, id) {
        let _ = reset_session(&app, id, true);
        return;
    }

    match outcome {
        Ok(DictationOutcome::Completed) => {
            let _ = reset_session(&app, id, true);
        }
        Ok(DictationOutcome::SttError { record_id }) => {
            if reset_session(&app, id, false) {
                let _ = overlay::show_error_overlay(&app, record_id);
            }
        }
        Ok(DictationOutcome::PostProcessError { record_id }) => {
            if reset_session(&app, id, false) {
                let _ = overlay::show_warning_overlay(&app, Some(record_id));
            }
        }
        Err(error) => {
            emit_dictation_error(&app, error.into_message());
            if reset_session(&app, id, false) {
                let _ = overlay::show_error_overlay(&app, None);
            }
        }
    }
}

async fn process_repeat_latest_history_record(app: tauri::AppHandle, id: u64, record_id: String) {
    let outcome = process_repeat_latest_history_record_inner(&app, id, &record_id).await;
    clear_active_task(&app, id);

    if !is_current_session(&app, id) {
        let _ = reset_session(&app, id, true);
        return;
    }

    match outcome {
        Ok(DictationOutcome::Completed) => {
            let _ = reset_session(&app, id, true);
        }
        Ok(DictationOutcome::SttError { record_id }) => {
            if reset_session(&app, id, false) {
                let _ = overlay::show_error_overlay(&app, record_id);
            }
        }
        Ok(DictationOutcome::PostProcessError { record_id }) => {
            if reset_session(&app, id, false) {
                let _ = overlay::show_warning_overlay(&app, Some(record_id));
            }
        }
        Err(error) => {
            emit_dictation_error(&app, error.into_message());
            if reset_session(&app, id, false) {
                let _ = overlay::show_error_overlay(&app, Some(record_id));
            }
        }
    }
}

async fn process_recording_inner(
    app: &tauri::AppHandle,
    id: u64,
    audio: RecordedAudio,
) -> AppResult<DictationOutcome> {
    overlay::show_transcribing_overlay(app)?;

    let config = load_processing_config(app)?;

    if config.stt.provider_id.is_none() || config.stt.model_key.is_none() {
        return Err(AppError::from(i18n::text(
            app,
            "dictation-stt-provider-and-model-not-selected",
        )));
    }

    let history_record_id = Uuid::new_v4().to_string();

    if !is_current_session(app, id) {
        return Ok(DictationOutcome::Completed);
    }

    let stt_snapshot = runner::build_stt_snapshot(app)?;
    let postprocessing_snapshot = if config.post_process.enabled {
        runner::build_post_process_snapshot(app).ok()
    } else {
        None
    };
    let stt_log_context = ModelRunLogContext {
        source: ModelRunSource::Dictation,
        operation_id: Uuid::new_v4().to_string(),
        history_record_id: Some(history_record_id.clone()),
        recording_started_at: Some(audio.started_at.to_rfc3339()),
        audio_duration_ms: Some(audio.duration_ms),
        audio_file_name: Some(audio.file_name.clone()),
        audio_size_bytes: Some(audio.bytes.len()),
        audio_path: None,
    };
    let transcription = match runner::run_stt_with_snapshot(
        app,
        &stt_snapshot,
        audio.bytes.clone(),
        audio.file_name.clone(),
        Some(audio.duration_ms),
        Some(stt_log_context),
    )
    .await
    {
        Ok(output) => output,
        Err(error) => {
            let record_id = history_record_id.clone();
            let _ = history::save_new_history_record(
                app,
                history::NewHistoryRecord {
                    id: Some(history_record_id),
                    audio,
                    postprocessing: None,
                    postprocessing_snapshot,
                    transcription: Err((stt_snapshot, error)),
                },
            );

            return Ok(DictationOutcome::SttError {
                record_id: Some(record_id),
            });
        }
    };

    if !is_current_session(app, id) {
        return Ok(DictationOutcome::Completed);
    }

    let (final_text, postprocessing) = if config.post_process.enabled {
        if !begin_processing_phase(app, id)? {
            return Ok(DictationOutcome::Completed);
        }
        let postprocessing_snapshot = runner::build_post_process_snapshot(app)?;
        let postprocessing_log_context = ModelRunLogContext {
            source: ModelRunSource::Dictation,
            operation_id: Uuid::new_v4().to_string(),
            history_record_id: Some(history_record_id.clone()),
            recording_started_at: Some(audio.started_at.to_rfc3339()),
            audio_duration_ms: Some(audio.duration_ms),
            audio_file_name: Some(audio.file_name.clone()),
            audio_size_bytes: Some(audio.bytes.len()),
            audio_path: None,
        };
        match runner::run_post_process_with_snapshot(
            app,
            &postprocessing_snapshot,
            transcription.text.clone(),
            Some(postprocessing_log_context),
        )
        .await
        {
            Ok(output) => (output.text.clone(), Some(Ok(output))),
            Err(error) => {
                let record_id = history_record_id.clone();
                let stt_text = transcription.text.clone();
                let _ = history::save_new_history_record(
                    app,
                    history::NewHistoryRecord {
                        id: Some(history_record_id),
                        audio,
                        postprocessing: Some(Err((postprocessing_snapshot, error))),
                        postprocessing_snapshot: None,
                        transcription: Ok(transcription),
                    },
                );

                // Постобработка не удалась, но текст распознавания речи корректен —
                // всё равно вставляем его, чтобы пользователь не потерял свою диктовку.
                if is_current_session(app, id) {
                    keyboard::paste_text(app, &stt_text).await?;
                }

                return Ok(DictationOutcome::PostProcessError { record_id });
            }
        }
    } else {
        (transcription.text.clone(), None)
    };

    if is_current_session(app, id) {
        let _ = history::save_new_history_record(
            app,
            history::NewHistoryRecord {
                id: Some(history_record_id),
                audio,
                postprocessing,
                postprocessing_snapshot,
                transcription: Ok(transcription),
            },
        );
        keyboard::paste_text(app, &final_text).await?;
    }

    Ok(DictationOutcome::Completed)
}

async fn process_repeat_latest_history_record_inner(
    app: &tauri::AppHandle,
    id: u64,
    record_id: &str,
) -> AppResult<DictationOutcome> {
    if !is_current_session(app, id) {
        return Ok(DictationOutcome::Completed);
    }

    let outcome = history::repeat_history_record_for_hotkey(app, record_id, || {
        begin_processing_phase(app, id)
    })
    .await?;

    if !is_current_session(app, id) {
        return Ok(DictationOutcome::Completed);
    }

    match outcome {
        history::RepeatHistoryHotkeyOutcome::Success { final_text } => {
            if final_text.trim().is_empty() {
                return Ok(DictationOutcome::Completed);
            }

            if is_current_session(app, id) {
                keyboard::paste_text(app, &final_text).await?;
            }

            Ok(DictationOutcome::Completed)
        }
        history::RepeatHistoryHotkeyOutcome::SttError { record_id } => {
            Ok(DictationOutcome::SttError {
                record_id: Some(record_id),
            })
        }
        history::RepeatHistoryHotkeyOutcome::PostProcessError {
            record_id,
            final_text,
        } => {
            if !final_text.trim().is_empty() && is_current_session(app, id) {
                keyboard::paste_text(app, &final_text).await?;
            }

            Ok(DictationOutcome::PostProcessError { record_id })
        }
    }
}

fn begin_processing_phase(app: &tauri::AppHandle, id: u64) -> AppResult<bool> {
    let runtime = app.state::<DictationRuntime>();
    let mut session = runtime
        .session
        .lock()
        .map_err(|_| AppError::from(i18n::text(app, "dictation-state-lock-failed")))?;

    if !try_enter_processing(&mut session, id) {
        return Ok(false);
    }

    drop(session);

    overlay::show_processing_overlay(app)?;

    // Отмена может состязаться (race) с переходом STT -> постобработка. Проверяем повторно после
    // показа оверлея, чтобы поздняя отмена не оставила оверлей обработки
    // видимым, когда сессия уже отменена.
    if !is_current_session(app, id) {
        let _ = overlay::hide_recording_overlay(app);
        return Ok(false);
    }

    Ok(true)
}

fn try_enter_processing(session: &mut DictationSession, id: u64) -> bool {
    if matches!(*session, DictationSession::Transcribing { id: current } if current == id) {
        *session = DictationSession::Processing { id };
        true
    } else {
        false
    }
}

fn is_current_session(app: &tauri::AppHandle, id: u64) -> bool {
    app.state::<DictationRuntime>()
        .session
        .lock()
        .map(|session| {
            matches!(
                *session,
                DictationSession::Transcribing { id: current }
                    | DictationSession::Processing { id: current }
                    if current == id
            )
        })
        .unwrap_or(false)
}

fn is_session_idle(app: &tauri::AppHandle) -> bool {
    app.state::<DictationRuntime>()
        .session
        .lock()
        .map(|session| matches!(*session, DictationSession::Idle))
        .unwrap_or(false)
}

/// Сбрасывает завершённую сессию в состояние ожидания (idle). Оверлей скрывается только когда
/// установлен `hide_overlay` — уведомления об ошибке/предупреждении вместо этого оставляют его видимым и
/// сами управляют своим закрытием.
fn reset_session(app: &tauri::AppHandle, id: u64, hide_overlay: bool) -> bool {
    if let Ok(mut session) = app.state::<DictationRuntime>().session.lock() {
        if matches!(
            *session,
            DictationSession::Transcribing { id: current }
                | DictationSession::Processing { id: current }
                if current == id
        ) {
            *session = DictationSession::Idle;
            clear_active_task(app, id);
            clear_active_hold_activation_id(app);
            shortcut_hook::disarm_cancel_hotkey();
            shortcut_hook::disarm_pause_hotkey();
            emit_dictation_session(app, false, None, false);
            if hide_overlay {
                let _ = overlay::hide_recording_overlay(app);
            }

            return true;
        }
    }

    false
}

fn cancel_dictation_inner(
    app: tauri::AppHandle,
    expected_session_id: Option<u64>,
) -> AppResult<()> {
    let runtime = app.state::<DictationRuntime>();
    let mut session = runtime
        .session
        .lock()
        .map_err(|_| AppError::from(i18n::text(&app, "dictation-state-lock-failed")))?;

    if expected_session_id
        .is_some_and(|expected_id| current_session_id(&session) != Some(expected_id))
    {
        return Ok(());
    }

    let cancelled_task_id = match std::mem::replace(&mut *session, DictationSession::Idle) {
        DictationSession::Idle => None,
        DictationSession::Recording { handle, .. } => {
            // Останавливаем захват и отбрасываем аудио; drop `handle` возвращает системный
            // звук после того, как микрофон освобождён. При отмене с паузы звук уже вернулся,
            // и drop ничего не делает.
            release_recording(&app);
            drop(handle);
            None
        }
        DictationSession::Transcribing { id } | DictationSession::Processing { id } => Some(id),
    };
    drop(session);

    if let Some(id) = cancelled_task_id {
        abort_active_task(&app, id);
    }

    clear_active_hold_activation_id(&app);
    shortcut_hook::disarm_cancel_hotkey();
    shortcut_hook::disarm_pause_hotkey();
    emit_dictation_session(&app, false, None, false);
    let _ = overlay::hide_recording_overlay(&app);

    Ok(())
}

fn emit_dictation_error(app: &tauri::AppHandle, message: String) {
    let _ = app.emit("dictation-error", DictationErrorPayload { message });
}

fn emit_dictation_session(
    app: &tauri::AppHandle,
    active: bool,
    session_id: Option<u64>,
    is_recording: bool,
) {
    let _ = app.emit(
        "dictation-session",
        DictationSessionPayload {
            active,
            session_id,
            is_recording,
        },
    );
}

fn current_session_id(session: &DictationSession) -> Option<u64> {
    match session {
        DictationSession::Idle => None,
        DictationSession::Recording { id, .. }
        | DictationSession::Transcribing { id }
        | DictationSession::Processing { id } => Some(*id),
    }
}

fn set_active_hold_activation_id(app: &tauri::AppHandle, activation_id: Option<u64>) {
    if let Ok(mut active_hold_activation_id) = app
        .state::<DictationRuntime>()
        .active_hold_activation_id
        .lock()
    {
        *active_hold_activation_id = activation_id;
    }
}

fn clear_active_hold_activation_id(app: &tauri::AppHandle) {
    set_active_hold_activation_id(app, None);
}

fn register_active_task(
    app: &tauri::AppHandle,
    session_id: u64,
    handle: tauri::async_runtime::JoinHandle<()>,
) {
    if let Ok(mut active_task) = app.state::<DictationRuntime>().active_task.lock() {
        if let Some(previous_task) = active_task.replace(ActiveDictationTask { session_id, handle })
        {
            previous_task.handle.abort();
        }
    }
}

fn clear_active_task(app: &tauri::AppHandle, session_id: u64) {
    if let Ok(mut active_task) = app.state::<DictationRuntime>().active_task.lock() {
        if active_task
            .as_ref()
            .is_some_and(|task| task.session_id == session_id)
        {
            active_task.take();
        }
    }
}

fn abort_active_task(app: &tauri::AppHandle, session_id: u64) {
    if let Ok(mut active_task) = app.state::<DictationRuntime>().active_task.lock() {
        if active_task
            .as_ref()
            .is_some_and(|task| task.session_id == session_id)
        {
            if let Some(task) = active_task.take() {
                task.handle.abort();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{try_enter_processing, DictationSession};

    #[test]
    fn try_enter_processing_transitions_matching_transcribing_session() {
        let mut session = DictationSession::Transcribing { id: 7 };

        assert!(try_enter_processing(&mut session, 7));
        assert!(matches!(session, DictationSession::Processing { id: 7 }));
    }

    #[test]
    fn try_enter_processing_ignores_other_session_id() {
        let mut session = DictationSession::Transcribing { id: 7 };

        assert!(!try_enter_processing(&mut session, 8));
        assert!(matches!(session, DictationSession::Transcribing { id: 7 }));
    }
}
