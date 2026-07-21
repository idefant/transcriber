use std::{fs, path::PathBuf, process::Command};

use chrono::{DateTime, Datelike, Local, SecondsFormat, TimeZone, Timelike, Utc};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};
use uuid::Uuid;

use crate::{
    db,
    debug_log::{self, ModelRunLogContext, ModelRunSource},
    error::{AppError, AppResult},
    i18n,
    metrics::{RunOutcome, RunTimer},
    processing::load_processing_config,
    recording::RecordedAudio,
    runner::{
        self, PostProcessRunOutput, PostProcessSettingsSnapshot, SttRunOutput, SttSettingsSnapshot,
    },
};

const HISTORY_FILE_NAME: &str = "history.json";
const RECORDINGS_DIR_NAME: &str = "recordings";
const HISTORY_UPDATED_EVENT: &str = "history-updated";
/// Сколько записей возвращает одна страница поиска.
const SEARCH_PAGE_SIZE: u32 = 100;
/// Trigram-токенайзер FTS5 не индексирует последовательности короче трёх
/// символов, поэтому запросы короче не ищутся вовсе.
const SEARCH_MIN_QUERY_CHARS: usize = 3;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryAudio {
    duration: String,
    duration_ms: u64,
    path: String,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingDetails {
    cost: Option<String>,
    duration: String,
    duration_ms: Option<u64>,
    error_message: Option<String>,
    #[serde(default)]
    error_details: Option<serde_json::Value>,
    is_processing: bool,
    model: String,
    provider: String,
    /// Фактический апстрим-провайдер, обработавший запрос (только для
    /// постобработки через OpenRouter).
    #[serde(default)]
    resolved_provider: Option<String>,
    status: HistoryResultStatus,
    text: String,
    usage: Option<serde_json::Value>,
    settings_snapshot: Option<serde_json::Value>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum HistoryResultStatus {
    Error,
    Processing,
    Skipped,
    Success,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryRecord {
    audio: HistoryAudio,
    created_at: String,
    final_text: String,
    id: String,
    postprocessing: ProcessingDetails,
    status: HistoryRecordStatus,
    time: String,
    transcription: ProcessingDetails,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum HistoryRecordStatus {
    Error,
    Processing,
    Success,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryGroup {
    date: String,
    label: String,
    month: String,
    records: Vec<HistoryRecord>,
}

/// Страница результатов поиска. `page_size` возвращается вместе с данными,
/// чтобы фронтенду не приходилось дублировать размер страницы у себя.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistorySearchResult {
    groups: Vec<HistoryGroup>,
    page: u32,
    page_size: u32,
    total: u32,
}

#[derive(Default, Deserialize, Serialize)]
struct HistoryStore {
    #[serde(default)]
    records: Vec<HistoryRecord>,
}

pub struct NewHistoryRecord {
    pub id: Option<String>,
    pub audio: RecordedAudio,
    pub postprocessing:
        Option<Result<PostProcessRunOutput, (PostProcessSettingsSnapshot, AppError)>>,
    pub postprocessing_snapshot: Option<PostProcessSettingsSnapshot>,
    pub transcription: Result<SttRunOutput, (SttSettingsSnapshot, AppError)>,
}

pub enum RepeatHistoryHotkeyOutcome {
    Success {
        final_text: String,
    },
    SttError {
        record_id: String,
    },
    PostProcessError {
        record_id: String,
        final_text: String,
    },
}

/// Асинхронная команда намеренно: синхронные команды Tauri выполняются в
/// главном потоке, а выборка и десериализация целого месяца истории заморозила
/// бы его. Фронтенд перезагружает месяц по каждому событию history-updated
/// (в т.ч. во время диктовки), поэтому эта работа должна идти вне главного
/// потока, иначе синтетический Ctrl+V вставки не успеет обработаться вовремя.
#[tauri::command]
pub async fn get_history_groups(
    app: tauri::AppHandle,
    month: Option<String>,
) -> Result<Vec<HistoryGroup>, String> {
    get_history_groups_inner(&app, month.as_deref()).map_err(AppError::into_message)
}

/// Локальный месяц самой старой записи истории в формате `YYYY-MM` или `None`,
/// если история пуста. Фронтенд использует его как нижнюю границу выбора
/// месяца. Асинхронная по той же причине, что и [`get_history_groups`]: команда
/// вызывается на каждое обновление истории, в том числе во время диктовки, и не
/// должна занимать главный поток.
#[tauri::command]
pub async fn get_history_oldest_month(app: tauri::AppHandle) -> Result<Option<String>, String> {
    get_history_oldest_month_inner(&app).map_err(AppError::into_message)
}

/// Асинхронная по той же причине, что и [`get_history_groups`]: страница поиска
/// содержит до `SEARCH_PAGE_SIZE` записей, и их десериализация не должна
/// выполняться в главном потоке.
#[tauri::command]
pub async fn search_history_records(
    app: tauri::AppHandle,
    query: String,
    page: u32,
) -> Result<HistorySearchResult, String> {
    search_history_records_inner(&app, &query, page).map_err(AppError::into_message)
}

#[tauri::command]
pub fn delete_history_record(app: tauri::AppHandle, record_id: String) -> Result<(), String> {
    delete_history_record_inner(&app, &record_id).map_err(AppError::into_message)
}

#[tauri::command]
pub fn open_history_audio(app: tauri::AppHandle, record_id: String) -> Result<(), String> {
    open_history_audio_inner(&app, &record_id).map_err(AppError::into_message)
}

#[tauri::command]
pub async fn repeat_history_transcription(
    app: tauri::AppHandle,
    record_id: String,
) -> Result<HistoryRecord, String> {
    repeat_history_transcription_inner(&app, &record_id)
        .await
        .map_err(AppError::into_message)
}

#[tauri::command]
pub async fn repeat_history_record(
    app: tauri::AppHandle,
    record_id: String,
) -> Result<HistoryRecord, String> {
    repeat_history_record_inner(&app, &record_id)
        .await
        .map_err(AppError::into_message)
}

#[tauri::command]
pub async fn repeat_history_post_processing(
    app: tauri::AppHandle,
    record_id: String,
) -> Result<HistoryRecord, String> {
    repeat_history_post_processing_inner(&app, &record_id)
        .await
        .map_err(AppError::into_message)
}

/// Показывает запись истории в главном окне. Используется уведомлениями об
/// ошибках/предупреждениях оверлея: скрывает оверлей, выводит главное окно
/// на передний план и сообщает странице истории, какую запись открыть
/// (с месяцем/датой, нужными для навигации).
#[tauri::command]
pub fn open_history_record(app: tauri::AppHandle, record_id: String) -> Result<(), String> {
    open_history_record_inner(&app, &record_id).map_err(AppError::into_message)
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct OpenHistoryRecordPayload {
    record_id: String,
    month: String,
    date: String,
}

fn open_history_record_inner(app: &tauri::AppHandle, record_id: &str) -> AppResult<()> {
    let _ = crate::overlay::hide_recording_overlay(app);

    let record = find_history_record(app, record_id)?;
    let local = parse_record_time(&record.created_at);
    let month = format!("{:04}-{:02}", local.year(), local.month());
    let date = format!(
        "{:04}-{:02}-{:02}",
        local.year(),
        local.month(),
        local.day()
    );

    crate::background::show_main_window(app)?;

    app.emit(
        "open-history-record",
        OpenHistoryRecordPayload {
            record_id: record_id.to_string(),
            month,
            date,
        },
    )?;

    Ok(())
}

pub fn save_new_history_record(
    app: &tauri::AppHandle,
    input: NewHistoryRecord,
) -> AppResult<HistoryRecord> {
    let id = input.id.unwrap_or_else(|| Uuid::new_v4().to_string());
    let created_at = input.audio.started_at;
    let audio_path = save_audio_file(app, created_at, &id, &input.audio.bytes)?;
    let transcription = match input.transcription {
        Ok(output) => result_from_stt_output(output),
        Err((snapshot, error)) => result_from_stt_error(snapshot, error),
    };
    let postprocessing = match input.postprocessing {
        Some(Ok(output)) => result_from_post_process_output(output),
        Some(Err((snapshot, error))) => result_from_post_process_error(Some(snapshot), error),
        None => skipped_result(input.postprocessing_snapshot),
    };
    let status = record_status(&transcription, &postprocessing);
    let final_text = final_text(&transcription, &postprocessing);
    let record = HistoryRecord {
        audio: HistoryAudio {
            duration: format_duration(input.audio.duration_ms),
            duration_ms: input.audio.duration_ms,
            path: audio_path.to_string_lossy().to_string(),
        },
        created_at: created_at.to_rfc3339(),
        final_text,
        id,
        postprocessing,
        status,
        time: format_time(created_at),
        transcription,
    };

    upsert_record(app, &record)?;
    emit_history_updated(app, Some(&record));
    debug_log::log_event(
        app,
        "history.recordSaved",
        Some(&ModelRunLogContext {
            source: ModelRunSource::Dictation,
            operation_id: Uuid::new_v4().to_string(),
            history_record_id: Some(record.id.clone()),
            recording_started_at: Some(record.created_at.clone()),
            audio_duration_ms: Some(record.audio.duration_ms),
            audio_file_name: Some(input.audio.file_name.clone()),
            audio_size_bytes: Some(input.audio.bytes.len()),
            audio_path: Some(record.audio.path.clone()),
        }),
        serde_json::json!({
            "record": {
                "id": record.id.clone(),
                "createdAt": record.created_at.clone(),
                "audio": record.audio.clone(),
                "status": record.status.clone(),
            },
        }),
    );

    Ok(record)
}

pub fn latest_history_text(app: &tauri::AppHandle) -> AppResult<String> {
    match db::latest_data(app)? {
        Some(data) => Ok(parse_record(&data)?.final_text),
        None => Ok(String::new()),
    }
}

pub fn latest_history_record_id(app: &tauri::AppHandle) -> AppResult<Option<String>> {
    match db::latest_data(app)? {
        Some(data) => Ok(Some(parse_record(&data)?.id)),
        None => Ok(None),
    }
}

pub async fn repeat_history_record_for_hotkey(
    app: &tauri::AppHandle,
    record_id: &str,
    before_post_process: impl FnOnce() -> AppResult<bool>,
) -> AppResult<RepeatHistoryHotkeyOutcome> {
    prepare_history_record_repeat(app, record_id)?;

    let record = repeat_history_transcription_inner(app, record_id).await?;
    let config = load_processing_config(app)?;
    let record = if should_run_repeat_hotkey_post_process(
        config.post_process.enabled,
        &record.transcription.status,
    ) {
        if !before_post_process()? {
            return repeat_history_hotkey_outcome(record);
        }

        repeat_history_post_processing_inner(app, record_id).await?
    } else {
        record
    };

    repeat_history_hotkey_outcome(record)
}

fn repeat_history_hotkey_outcome(record: HistoryRecord) -> AppResult<RepeatHistoryHotkeyOutcome> {
    if matches!(record.transcription.status, HistoryResultStatus::Error) {
        return Ok(RepeatHistoryHotkeyOutcome::SttError {
            record_id: record.id,
        });
    }

    if matches!(record.postprocessing.status, HistoryResultStatus::Error) {
        return Ok(RepeatHistoryHotkeyOutcome::PostProcessError {
            record_id: record.id,
            final_text: record.final_text,
        });
    }

    Ok(RepeatHistoryHotkeyOutcome::Success {
        final_text: record.final_text,
    })
}

fn should_run_repeat_hotkey_post_process(
    post_process_enabled: bool,
    transcription_status: &HistoryResultStatus,
) -> bool {
    post_process_enabled && matches!(transcription_status, HistoryResultStatus::Success)
}

fn get_history_groups_inner(
    app: &tauri::AppHandle,
    month: Option<&str>,
) -> AppResult<Vec<HistoryGroup>> {
    // Границы выбранного локального месяца переводятся в UTC до запроса.
    // Так `created_at` остаётся единственным источником истины для времени,
    // а база может использовать его индекс и для фильтрации, и для сортировки.
    let created_at_range = month.map(local_month_bounds).transpose()?;
    let records = db::list_data(
        app,
        created_at_range
            .as_ref()
            .map(|(from, to)| (from.as_str(), to.as_str())),
    )?;

    group_records(records)
}

fn get_history_oldest_month_inner(app: &tauri::AppHandle) -> AppResult<Option<String>> {
    let Some(created_at) = db::oldest_created_at(app)? else {
        return Ok(None);
    };

    // Записи хранятся в UTC, а месяц истории всегда локальный, поэтому границу
    // нужно считать по локальному времени самой старой записи.
    let local = parse_record_time(&created_at);

    Ok(Some(format!("{:04}-{:02}", local.year(), local.month())))
}

fn search_history_records_inner(
    app: &tauri::AppHandle,
    query: &str,
    page: u32,
) -> AppResult<HistorySearchResult> {
    let query = query.trim();
    let page = page.max(1);

    if query.chars().count() < SEARCH_MIN_QUERY_CHARS {
        return Ok(HistorySearchResult {
            groups: Vec::new(),
            page,
            page_size: SEARCH_PAGE_SIZE,
            total: 0,
        });
    }

    let offset = (page - 1) * SEARCH_PAGE_SIZE;
    let (records, total) = db::search_data(
        app,
        &build_fts_phrase_query(query),
        SEARCH_PAGE_SIZE,
        offset,
    )?;

    Ok(HistorySearchResult {
        groups: group_records(records)?,
        page,
        page_size: SEARCH_PAGE_SIZE,
        total,
    })
}

/// Оборачивает пользовательский ввод в фразу FTS5, чтобы он искался как
/// подстрока-литерал: внутри кавычек спецсинтаксис FTS (`AND`, `OR`, `*`, `^`,
/// `:`) теряет силу, а сами кавычки экранируются удвоением.
fn build_fts_phrase_query(query: &str) -> String {
    format!("\"{}\"", query.replace('"', "\"\""))
}

/// Группирует JSON-строки записей по локальной календарной дате, сохраняя
/// порядок, в котором их вернула база (от новых к старым).
fn group_records(records: Vec<String>) -> AppResult<Vec<HistoryGroup>> {
    let mut groups: Vec<HistoryGroup> = Vec::new();

    for data in records {
        let record = parse_record(&data)?;
        let local = parse_record_time(&record.created_at);
        let record_month = format!("{:04}-{:02}", local.year(), local.month());

        let date = format!(
            "{:04}-{:02}-{:02}",
            local.year(),
            local.month(),
            local.day()
        );

        if let Some(group) = groups.iter_mut().find(|group| group.date == date) {
            group.records.push(record);
        } else {
            groups.push(HistoryGroup {
                date,
                label: format!(
                    "{:02}.{:02}.{:04}",
                    local.day(),
                    local.month(),
                    local.year()
                ),
                month: record_month,
                records: vec![record],
            });
        }
    }

    Ok(groups)
}

fn delete_history_record_inner(app: &tauri::AppHandle, record_id: &str) -> AppResult<()> {
    let record = find_history_record(app, record_id)?;

    db::delete(app, record_id)?;

    let path = PathBuf::from(record.audio.path);

    if path.exists() {
        fs::remove_file(path)?;
    }

    emit_history_updated(app, None);

    Ok(())
}

fn open_history_audio_inner(app: &tauri::AppHandle, record_id: &str) -> AppResult<()> {
    let record = find_history_record(app, record_id)?;
    let path = PathBuf::from(record.audio.path);

    if !path.exists() {
        return Err(i18n::text(app, "history-audio-file-not-found").into());
    }

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;

        let absolute_path = fs::canonicalize(&path)?;

        Command::new("explorer.exe")
            .raw_arg(format!("/select,\"{}\"", absolute_path.to_string_lossy()))
            .spawn()
            .map_err(|error| {
                AppError::from(i18n::text_with(
                    app,
                    "history-open-file-explorer-failed",
                    &[("error", error.to_string())],
                ))
            })?;
    }

    #[cfg(not(target_os = "windows"))]
    {
        Command::new("xdg-open")
            .arg(path.parent().unwrap_or(&path))
            .spawn()
            .map_err(|error| {
                AppError::from(i18n::text_with(
                    app,
                    "history-open-audio-location-failed",
                    &[("error", error.to_string())],
                ))
            })?;
    }

    Ok(())
}

async fn repeat_history_transcription_inner(
    app: &tauri::AppHandle,
    record_id: &str,
) -> AppResult<HistoryRecord> {
    let mut record = find_history_record(app, record_id)?;
    let audio_path = record.audio.path.clone();
    let audio_duration_ms = record.audio.duration_ms;
    let created_at = record.created_at.clone();

    record.transcription = processing_result();
    record.final_text = final_text(&record.transcription, &record.postprocessing);
    record.status = HistoryRecordStatus::Processing;
    upsert_record(app, &record)?;
    emit_history_updated(app, Some(&record));

    let audio_bytes = fs::read(&audio_path)?;
    let log_context = ModelRunLogContext {
        source: ModelRunSource::HistoryRepeat,
        operation_id: Uuid::new_v4().to_string(),
        history_record_id: Some(record_id.to_string()),
        recording_started_at: Some(created_at),
        audio_duration_ms: Some(audio_duration_ms),
        audio_file_name: Some("dictation.wav".to_string()),
        audio_size_bytes: Some(audio_bytes.len()),
        audio_path: Some(audio_path.clone()),
    };
    let stt_snapshot = match runner::build_stt_snapshot(app) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            return save_repeated_stt_error(app, record_id, None, error);
        }
    };
    let timer = RunTimer::new(&ModelRunSource::HistoryRepeat);
    timer.set_history_record_id(record_id);
    timer.set_audio(audio_duration_ms, audio_bytes.len());

    let result = runner::run_stt_with_snapshot(
        app,
        &stt_snapshot,
        audio_bytes,
        "dictation.wav".to_string(),
        Some(audio_duration_ms),
        Some(log_context),
        Some(&timer),
    )
    .await;

    timer.finish(
        app,
        if result.is_ok() {
            RunOutcome::Completed
        } else {
            RunOutcome::SttError
        },
    );

    match result {
        Ok(output) => {
            // Перечитываем запись: за время STT её могли изменить.
            let mut record = find_history_record(app, record_id)?;
            record.transcription = result_from_stt_output(output);
            record.final_text = final_text(&record.transcription, &record.postprocessing);
            record.status = record_status(&record.transcription, &record.postprocessing);
            upsert_record(app, &record)?;
            emit_history_updated(app, Some(&record));

            Ok(record)
        }
        Err(error) => save_repeated_stt_error(app, record_id, Some(stt_snapshot), error),
    }
}

async fn repeat_history_record_inner(
    app: &tauri::AppHandle,
    record_id: &str,
) -> AppResult<HistoryRecord> {
    prepare_history_record_repeat(app, record_id)?;

    let record = repeat_history_transcription_inner(app, record_id).await?;
    let config = load_processing_config(app)?;

    if config.post_process.enabled
        && matches!(record.transcription.status, HistoryResultStatus::Success)
    {
        repeat_history_post_processing_inner(app, record_id).await
    } else {
        Ok(record)
    }
}

fn prepare_history_record_repeat(app: &tauri::AppHandle, record_id: &str) -> AppResult<()> {
    let mut record = find_history_record(app, record_id)?;

    record.postprocessing = skipped_result(None);
    record.final_text = String::new();
    upsert_record(app, &record)?;
    emit_history_updated(app, Some(&record));

    Ok(())
}

async fn repeat_history_post_processing_inner(
    app: &tauri::AppHandle,
    record_id: &str,
) -> AppResult<HistoryRecord> {
    let mut record = find_history_record(app, record_id)?;

    if !matches!(record.transcription.status, HistoryResultStatus::Success) {
        return Err(
            i18n::text(app, "history-transcription-required-before-post-processing").into(),
        );
    }

    if !load_processing_config(app)?.post_process.enabled {
        return Err(i18n::text(app, "history-post-processing-disabled").into());
    }

    let input_text = record.transcription.text.clone();
    let created_at = record.created_at.clone();
    let audio_duration_ms = record.audio.duration_ms;
    let audio_path = record.audio.path.clone();
    let snapshot = match runner::build_post_process_snapshot(app) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            record.postprocessing = result_from_post_process_error(None, error);
            record.final_text = final_text(&record.transcription, &record.postprocessing);
            record.status = record_status(&record.transcription, &record.postprocessing);
            upsert_record(app, &record)?;
            emit_history_updated(app, Some(&record));

            return Ok(record);
        }
    };

    record.postprocessing = processing_result();
    record.postprocessing.settings_snapshot = serde_json::to_value(snapshot.clone()).ok();
    record.postprocessing.model = snapshot.model_label.clone();
    record.postprocessing.provider = snapshot.provider.provider_name.clone();
    record.status = HistoryRecordStatus::Processing;
    upsert_record(app, &record)?;
    emit_history_updated(app, Some(&record));

    let log_context = ModelRunLogContext {
        source: ModelRunSource::HistoryRepeat,
        operation_id: Uuid::new_v4().to_string(),
        history_record_id: Some(record_id.to_string()),
        recording_started_at: Some(created_at),
        audio_duration_ms: Some(audio_duration_ms),
        audio_file_name: Some("dictation.wav".to_string()),
        audio_size_bytes: fs::metadata(&audio_path)
            .ok()
            .and_then(|metadata| metadata.len().try_into().ok()),
        audio_path: Some(audio_path),
    };
    let timer = RunTimer::new(&ModelRunSource::HistoryRepeat);
    timer.set_history_record_id(record_id);

    let result = runner::run_post_process_with_snapshot(
        app,
        &snapshot,
        input_text,
        Some(log_context),
        Some(&timer),
    )
    .await;

    timer.finish(
        app,
        if result.is_ok() {
            RunOutcome::Completed
        } else {
            RunOutcome::PostProcessError
        },
    );

    // Перечитываем запись: за время постобработки её могли изменить.
    let mut record = find_history_record(app, record_id)?;

    match result {
        Ok(output) => {
            record.postprocessing = result_from_post_process_output(output);
        }
        Err(error) => {
            record.postprocessing = result_from_post_process_error(Some(snapshot), error);
        }
    }

    record.final_text = final_text(&record.transcription, &record.postprocessing);
    record.status = record_status(&record.transcription, &record.postprocessing);
    upsert_record(app, &record)?;
    emit_history_updated(app, Some(&record));

    Ok(record)
}

fn result_from_stt_output(output: SttRunOutput) -> ProcessingDetails {
    ProcessingDetails {
        cost: format_cost(output.cost),
        duration: format_duration(output.duration_ms),
        duration_ms: Some(output.duration_ms),
        error_message: None,
        error_details: None,
        is_processing: false,
        model: output.model,
        provider: output.provider,
        resolved_provider: None,
        status: HistoryResultStatus::Success,
        text: output.text,
        usage: output.usage.map(|usage| usage.raw),
        settings_snapshot: serde_json::to_value(output.settings_snapshot).ok(),
    }
}

fn result_from_stt_error(snapshot: SttSettingsSnapshot, error: AppError) -> ProcessingDetails {
    let (message, details) = error.into_message_and_details();

    ProcessingDetails {
        cost: format_cost(None),
        duration: format_duration(0),
        duration_ms: None,
        error_message: Some(message),
        error_details: details,
        is_processing: false,
        model: snapshot.model_label.clone(),
        provider: snapshot.provider.provider_name.clone(),
        resolved_provider: None,
        status: HistoryResultStatus::Error,
        text: String::new(),
        usage: None,
        settings_snapshot: serde_json::to_value(snapshot).ok(),
    }
}

fn result_from_generic_stt_error(error: AppError) -> ProcessingDetails {
    let (message, details) = error.into_message_and_details();

    ProcessingDetails {
        cost: format_cost(None),
        duration: format_duration(0),
        duration_ms: None,
        error_message: Some(message),
        error_details: details,
        is_processing: false,
        model: String::new(),
        provider: String::new(),
        resolved_provider: None,
        status: HistoryResultStatus::Error,
        text: String::new(),
        usage: None,
        settings_snapshot: None,
    }
}

fn result_from_post_process_output(output: PostProcessRunOutput) -> ProcessingDetails {
    ProcessingDetails {
        cost: format_cost(output.cost),
        duration: format_duration(output.duration_ms),
        duration_ms: Some(output.duration_ms),
        error_message: None,
        error_details: None,
        is_processing: false,
        model: output.model,
        provider: output.provider,
        resolved_provider: output.resolved_provider,
        status: HistoryResultStatus::Success,
        text: output.text,
        usage: output.usage.map(|usage| usage.raw),
        settings_snapshot: serde_json::to_value(output.settings_snapshot).ok(),
    }
}

fn result_from_post_process_error(
    snapshot: Option<PostProcessSettingsSnapshot>,
    error: AppError,
) -> ProcessingDetails {
    let (message, details) = error.into_message_and_details();

    ProcessingDetails {
        cost: format_cost(None),
        duration: format_duration(0),
        duration_ms: None,
        error_message: Some(message),
        error_details: details,
        is_processing: false,
        model: snapshot
            .as_ref()
            .map(|snapshot| snapshot.model_label.clone())
            .unwrap_or_default(),
        provider: snapshot
            .as_ref()
            .map(|snapshot| snapshot.provider.provider_name.clone())
            .unwrap_or_default(),
        resolved_provider: None,
        status: HistoryResultStatus::Error,
        text: String::new(),
        usage: None,
        settings_snapshot: snapshot.and_then(|snapshot| serde_json::to_value(snapshot).ok()),
    }
}

fn processing_result() -> ProcessingDetails {
    ProcessingDetails {
        cost: format_cost(None),
        duration: format_duration(0),
        duration_ms: None,
        error_message: None,
        error_details: None,
        is_processing: true,
        model: String::new(),
        provider: String::new(),
        resolved_provider: None,
        status: HistoryResultStatus::Processing,
        text: String::new(),
        usage: None,
        settings_snapshot: None,
    }
}

fn skipped_result(snapshot: Option<PostProcessSettingsSnapshot>) -> ProcessingDetails {
    ProcessingDetails {
        cost: format_cost(None),
        duration: format_duration(0),
        duration_ms: None,
        error_message: None,
        error_details: None,
        is_processing: false,
        model: snapshot
            .as_ref()
            .map(|snapshot| snapshot.model_label.clone())
            .unwrap_or_default(),
        provider: snapshot
            .as_ref()
            .map(|snapshot| snapshot.provider.provider_name.clone())
            .unwrap_or_default(),
        resolved_provider: None,
        status: HistoryResultStatus::Skipped,
        text: String::new(),
        usage: None,
        settings_snapshot: snapshot.and_then(|snapshot| serde_json::to_value(snapshot).ok()),
    }
}

fn save_repeated_stt_error(
    app: &tauri::AppHandle,
    record_id: &str,
    snapshot: Option<SttSettingsSnapshot>,
    error: AppError,
) -> AppResult<HistoryRecord> {
    let mut record = find_history_record(app, record_id)?;

    record.transcription = match snapshot {
        Some(snapshot) => result_from_stt_error(snapshot, error),
        None => result_from_generic_stt_error(error),
    };
    record.final_text = final_text(&record.transcription, &record.postprocessing);
    record.status = record_status(&record.transcription, &record.postprocessing);
    upsert_record(app, &record)?;
    emit_history_updated(app, Some(&record));

    Ok(record)
}

fn final_text(transcription: &ProcessingDetails, postprocessing: &ProcessingDetails) -> String {
    if matches!(postprocessing.status, HistoryResultStatus::Success) {
        postprocessing.text.clone()
    } else if matches!(transcription.status, HistoryResultStatus::Success) {
        transcription.text.clone()
    } else {
        String::new()
    }
}

fn record_status(
    transcription: &ProcessingDetails,
    postprocessing: &ProcessingDetails,
) -> HistoryRecordStatus {
    if matches!(transcription.status, HistoryResultStatus::Processing)
        || matches!(postprocessing.status, HistoryResultStatus::Processing)
    {
        HistoryRecordStatus::Processing
    } else if matches!(transcription.status, HistoryResultStatus::Success)
        && !matches!(postprocessing.status, HistoryResultStatus::Error)
    {
        HistoryRecordStatus::Success
    } else {
        HistoryRecordStatus::Error
    }
}

fn find_history_record(app: &tauri::AppHandle, record_id: &str) -> AppResult<HistoryRecord> {
    match db::get_data(app, record_id)? {
        Some(data) => parse_record(&data),
        None => Err(i18n::text(app, "history-record-not-found").into()),
    }
}

/// Разбирает JSON-строку колонки `data` в запись истории.
fn parse_record(data: &str) -> AppResult<HistoryRecord> {
    Ok(serde_json::from_str(data)?)
}

/// Строит строку таблицы истории из записи. `created_at` и тексты вынесены в
/// колонки для запросов, а вся запись дополнительно кладётся в `data` как
/// JSON, чтобы не менять формат данных истории.
fn record_row(record: &HistoryRecord) -> AppResult<db::RecordRow> {
    Ok(db::RecordRow {
        id: record.id.clone(),
        created_at: record.created_at.clone(),
        transcription_text: record.transcription.text.clone(),
        postprocessing_text: record.postprocessing.text.clone(),
        data: serde_json::to_string(record)?,
    })
}

/// Возвращает UTC-границы локального календарного месяца в формате RFC3339.
/// Верхняя граница не входит в интервал.
fn local_month_bounds(month: &str) -> AppResult<(String, String)> {
    let Some((year, month)) = month.split_once('-') else {
        return Err(AppError::from("history month must use YYYY-MM format"));
    };
    let year = year
        .parse::<i32>()
        .map_err(|_| AppError::from("history month contains an invalid year"))?;
    let month = month
        .parse::<u32>()
        .map_err(|_| AppError::from("history month contains an invalid month"))?;

    let (next_year, next_month) = if month == 12 {
        (year + 1, 1)
    } else {
        (year, month + 1)
    };
    let start = Local
        .with_ymd_and_hms(year, month, 1, 0, 0, 0)
        .single()
        .ok_or_else(|| AppError::from("history month is outside the supported range"))?;
    let end = Local
        .with_ymd_and_hms(next_year, next_month, 1, 0, 0, 0)
        .single()
        .ok_or_else(|| AppError::from("history month is outside the supported range"))?;

    Ok((
        start
            .with_timezone(&Utc)
            .to_rfc3339_opts(SecondsFormat::Millis, false),
        end.with_timezone(&Utc)
            .to_rfc3339_opts(SecondsFormat::Millis, false),
    ))
}

fn upsert_record(app: &tauri::AppHandle, record: &HistoryRecord) -> AppResult<()> {
    db::upsert(app, &record_row(record)?)
}

/// Дозаписывает стоимость постобработки, полученную уже после вставки текста.
///
/// OpenRouter сообщает стоимость отдельным запросом, и раньше его ждали до
/// вставки. Теперь запрос уходит в фон, а результат догоняет запись здесь.
/// Запись перечитывается: за это время её могли изменить или удалить, и второе
/// не считается ошибкой — догонять больше нечего.
pub fn set_post_processing_cost(
    app: &tauri::AppHandle,
    record_id: &str,
    cost: f64,
) -> AppResult<()> {
    let Ok(mut record) = find_history_record(app, record_id) else {
        return Ok(());
    };

    record.postprocessing.cost = format_cost(Some(cost));
    upsert_record(app, &record)?;
    emit_history_updated(app, Some(&record));

    Ok(())
}

/// Переносит историю из `history.json` в базу SQLite. Вызывается один раз
/// миграцией схемы v3.
///
/// Исходный `history.json` не удаляется, а переименовывается в резервную
/// копию: так пользователь ничего не теряет, даже если позже запустит более
/// старую версию приложения. Повреждённый JSON тоже сохраняется в бэкап, а
/// импорт при этом продолжается с пустой базой, чтобы не блокировать запуск.
pub fn migrate_history_json_to_db(app: &tauri::AppHandle) -> AppResult<()> {
    let dir = app.path().app_data_dir()?;
    let json_path = dir.join(HISTORY_FILE_NAME);

    if !json_path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&json_path)?;

    if content.trim().is_empty() {
        let _ = fs::remove_file(&json_path);
        return Ok(());
    }

    let store: HistoryStore = match serde_json::from_str(&content) {
        Ok(store) => store,
        Err(_) => {
            backup_history_json(&dir, &json_path, "corrupt");
            return Ok(());
        }
    };

    let rows = store
        .records
        .iter()
        .map(record_row)
        .collect::<AppResult<Vec<_>>>()?;

    db::import(app, &rows)?;
    backup_history_json(&dir, &json_path, "pre-sqlite");

    Ok(())
}

/// Переименовывает `history.json` в резервную копию рядом с ним. Если файл
/// с таким именем уже есть, добавляет к нему временную метку.
fn backup_history_json(dir: &std::path::Path, json_path: &std::path::Path, suffix: &str) {
    let mut backup = dir.join(format!("history.{suffix}.bak"));

    if backup.exists() {
        let timestamp = Local::now().format("%Y%m%d-%H%M%S");
        backup = dir.join(format!("history.{suffix}-{timestamp}.bak"));
    }

    let _ = fs::rename(json_path, backup);
}

fn save_audio_file(
    app: &tauri::AppHandle,
    started_at: DateTime<Utc>,
    record_id: &str,
    audio: &[u8],
) -> AppResult<PathBuf> {
    let recordings_dir = app.path().app_data_dir()?.join(RECORDINGS_DIR_NAME);

    fs::create_dir_all(&recordings_dir)?;

    let file_name = sanitize_file_name(&started_at.to_rfc3339_opts(SecondsFormat::Millis, true));
    let mut path = recordings_dir.join(format!("{file_name}.wav"));

    if path.exists() {
        path = recordings_dir.join(format!("{file_name}-{record_id}.wav"));
    }

    fs::write(&path, audio)?;

    Ok(path)
}

fn sanitize_file_name(value: &str) -> String {
    value
        .chars()
        .map(|character| match character {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '-',
            character if character.is_control() => '-',
            character => character,
        })
        .collect()
}

fn parse_record_time(value: &str) -> DateTime<Local> {
    DateTime::parse_from_rfc3339(value)
        .map(|date| date.with_timezone(&Local))
        .unwrap_or_else(|_| Local::now())
}

fn format_time(created_at: DateTime<Utc>) -> String {
    let local = created_at.with_timezone(&Local);

    format!("{:02}:{:02}", local.hour(), local.minute())
}

fn format_duration(duration_ms: u64) -> String {
    if duration_ms == 0 {
        return "-".to_string();
    }

    let total_seconds = duration_ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    let millis = duration_ms % 1000;

    format!("{minutes:02}:{seconds:02}.{millis:03}")
}

fn format_cost(cost: Option<f64>) -> Option<String> {
    cost.map(|value| format!("${value:.6}"))
}

fn emit_history_updated(app: &tauri::AppHandle, record: Option<&HistoryRecord>) {
    let _ = app.emit(HISTORY_UPDATED_EVENT, record);
    crate::background::refresh_tray_history_state(app);
}

#[cfg(test)]
mod tests {
    use super::{should_run_repeat_hotkey_post_process, HistoryResultStatus};

    #[test]
    fn repeat_hotkey_post_process_requires_feature_enabled() {
        assert!(!should_run_repeat_hotkey_post_process(
            false,
            &HistoryResultStatus::Success,
        ));
    }

    #[test]
    fn repeat_hotkey_post_process_requires_successful_transcription() {
        assert!(!should_run_repeat_hotkey_post_process(
            true,
            &HistoryResultStatus::Processing,
        ));
        assert!(!should_run_repeat_hotkey_post_process(
            true,
            &HistoryResultStatus::Error,
        ));
    }

    #[test]
    fn repeat_hotkey_post_process_runs_only_after_successful_stt() {
        assert!(should_run_repeat_hotkey_post_process(
            true,
            &HistoryResultStatus::Success,
        ));
    }
}
