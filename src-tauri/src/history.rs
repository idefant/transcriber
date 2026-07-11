use std::{fs, path::PathBuf, process::Command};

use chrono::{DateTime, Datelike, Local, SecondsFormat, Timelike, Utc};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager};
use uuid::Uuid;

use crate::{
    debug_log::{self, ModelRunLogContext, ModelRunSource},
    error::{AppError, AppResult},
    i18n,
    processing::load_processing_config,
    recording::RecordedAudio,
    runner::{
        self, PostProcessRunOutput, PostProcessSettingsSnapshot, SttRunOutput, SttSettingsSnapshot,
    },
    storage,
};

const HISTORY_FILE_NAME: &str = "history.json";
const RECORDINGS_DIR_NAME: &str = "recordings";
const HISTORY_UPDATED_EVENT: &str = "history-updated";

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

#[tauri::command]
pub fn get_history_groups(
    app: tauri::AppHandle,
    month: Option<String>,
) -> Result<Vec<HistoryGroup>, String> {
    get_history_groups_inner(&app, month.as_deref()).map_err(AppError::into_message)
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
    let mut store = load_history_store(app)?;
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

    store.records.push(record.clone());
    sort_records(&mut store.records);
    save_history_store(app, &store)?;
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
    let mut records = load_history_store(app)?.records;

    sort_records(&mut records);

    Ok(records
        .first()
        .map(|record| record.final_text.clone())
        .unwrap_or_default())
}

pub fn latest_history_record_id(app: &tauri::AppHandle) -> AppResult<Option<String>> {
    let mut records = load_history_store(app)?.records;

    sort_records(&mut records);

    Ok(records.first().map(|record| record.id.clone()))
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
    let mut records = load_history_store(app)?.records;
    sort_records(&mut records);

    let mut groups: Vec<HistoryGroup> = Vec::new();

    for record in records {
        let local = parse_record_time(&record.created_at);
        let record_month = format!("{:04}-{:02}", local.year(), local.month());

        if month.is_some_and(|month| month != record_month) {
            continue;
        }

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
    let mut store = load_history_store(app)?;
    let record = store
        .records
        .iter()
        .find(|record| record.id == record_id)
        .cloned()
        .ok_or_else(|| i18n::text(app, "history-record-not-found"))?;

    store.records.retain(|record| record.id != record_id);
    save_history_store(app, &store)?;

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
    let mut store = load_history_store(app)?;
    let index = find_record_index(app, &store.records, record_id)?;
    let audio_path = store.records[index].audio.path.clone();
    let audio_duration_ms = store.records[index].audio.duration_ms;
    let created_at = store.records[index].created_at.clone();

    store.records[index].transcription = processing_result();
    store.records[index].final_text = final_text(
        &store.records[index].transcription,
        &store.records[index].postprocessing,
    );
    store.records[index].status = HistoryRecordStatus::Processing;
    save_history_store(app, &store)?;
    emit_history_updated(app, Some(&store.records[index]));

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
    let result = runner::run_stt_with_snapshot(
        app,
        &stt_snapshot,
        audio_bytes,
        "dictation.wav".to_string(),
        Some(audio_duration_ms),
        Some(log_context),
    )
    .await;

    let mut store = load_history_store(app)?;
    let index = find_record_index(app, &store.records, record_id)?;

    match result {
        Ok(output) => {
            store.records[index].transcription = result_from_stt_output(output);
            store.records[index].final_text = final_text(
                &store.records[index].transcription,
                &store.records[index].postprocessing,
            );
            store.records[index].status = record_status(
                &store.records[index].transcription,
                &store.records[index].postprocessing,
            );
            save_history_store(app, &store)?;
            emit_history_updated(app, Some(&store.records[index]));

            Ok(store.records[index].clone())
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
    let mut store = load_history_store(app)?;
    let index = find_record_index(app, &store.records, record_id)?;

    store.records[index].postprocessing = skipped_result(None);
    store.records[index].final_text = String::new();
    save_history_store(app, &store)?;
    emit_history_updated(app, Some(&store.records[index]));

    Ok(())
}

async fn repeat_history_post_processing_inner(
    app: &tauri::AppHandle,
    record_id: &str,
) -> AppResult<HistoryRecord> {
    let mut store = load_history_store(app)?;
    let index = find_record_index(app, &store.records, record_id)?;

    if !matches!(
        store.records[index].transcription.status,
        HistoryResultStatus::Success
    ) {
        return Err(
            i18n::text(app, "history-transcription-required-before-post-processing").into(),
        );
    }

    if !load_processing_config(app)?.post_process.enabled {
        return Err(i18n::text(app, "history-post-processing-disabled").into());
    }

    let input_text = store.records[index].transcription.text.clone();
    let created_at = store.records[index].created_at.clone();
    let audio_duration_ms = store.records[index].audio.duration_ms;
    let audio_path = store.records[index].audio.path.clone();
    let snapshot = match runner::build_post_process_snapshot(app) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            store.records[index].postprocessing = result_from_post_process_error(None, error);
            store.records[index].final_text = final_text(
                &store.records[index].transcription,
                &store.records[index].postprocessing,
            );
            store.records[index].status = record_status(
                &store.records[index].transcription,
                &store.records[index].postprocessing,
            );
            save_history_store(app, &store)?;
            emit_history_updated(app, Some(&store.records[index]));

            return Ok(store.records[index].clone());
        }
    };

    store.records[index].postprocessing = processing_result();
    store.records[index].postprocessing.settings_snapshot =
        serde_json::to_value(snapshot.clone()).ok();
    store.records[index].postprocessing.model = snapshot.model_label.clone();
    store.records[index].postprocessing.provider = snapshot.provider.provider_name.clone();
    store.records[index].status = HistoryRecordStatus::Processing;
    save_history_store(app, &store)?;
    emit_history_updated(app, Some(&store.records[index]));

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
    let result =
        runner::run_post_process_with_snapshot(app, &snapshot, input_text, Some(log_context)).await;
    let mut store = load_history_store(app)?;
    let index = find_record_index(app, &store.records, record_id)?;

    match result {
        Ok(output) => {
            store.records[index].postprocessing = result_from_post_process_output(output);
            store.records[index].final_text = final_text(
                &store.records[index].transcription,
                &store.records[index].postprocessing,
            );
            store.records[index].status = record_status(
                &store.records[index].transcription,
                &store.records[index].postprocessing,
            );
            save_history_store(app, &store)?;
            emit_history_updated(app, Some(&store.records[index]));
            Ok(store.records[index].clone())
        }
        Err(error) => {
            store.records[index].postprocessing =
                result_from_post_process_error(Some(snapshot), error);
            store.records[index].final_text = final_text(
                &store.records[index].transcription,
                &store.records[index].postprocessing,
            );
            store.records[index].status = record_status(
                &store.records[index].transcription,
                &store.records[index].postprocessing,
            );
            save_history_store(app, &store)?;
            emit_history_updated(app, Some(&store.records[index]));
            Ok(store.records[index].clone())
        }
    }
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
    let mut store = load_history_store(app)?;
    let index = find_record_index(app, &store.records, record_id)?;

    store.records[index].transcription = match snapshot {
        Some(snapshot) => result_from_stt_error(snapshot, error),
        None => result_from_generic_stt_error(error),
    };
    store.records[index].final_text = final_text(
        &store.records[index].transcription,
        &store.records[index].postprocessing,
    );
    store.records[index].status = record_status(
        &store.records[index].transcription,
        &store.records[index].postprocessing,
    );
    save_history_store(app, &store)?;
    emit_history_updated(app, Some(&store.records[index]));

    Ok(store.records[index].clone())
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
    load_history_store(app)?
        .records
        .into_iter()
        .find(|record| record.id == record_id)
        .ok_or_else(|| i18n::text(app, "history-record-not-found").into())
}

fn find_record_index(
    app: &tauri::AppHandle,
    records: &[HistoryRecord],
    record_id: &str,
) -> AppResult<usize> {
    records
        .iter()
        .position(|record| record.id == record_id)
        .ok_or_else(|| i18n::text(app, "history-record-not-found").into())
}

fn load_history_store(app: &tauri::AppHandle) -> AppResult<HistoryStore> {
    storage::load_json_strict(app, HISTORY_FILE_NAME)
}

fn save_history_store(app: &tauri::AppHandle, store: &HistoryStore) -> AppResult<()> {
    storage::save_json(app, HISTORY_FILE_NAME, store)
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

fn sort_records(records: &mut [HistoryRecord]) {
    records.sort_by(|first, second| second.created_at.cmp(&first.created_at));
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
