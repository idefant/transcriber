use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
    time::{Duration, SystemTime},
};

use chrono::{SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use tauri::Manager;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    settings,
};

const LOGS_DIR_NAME: &str = "debug-logs";
const LOG_FILE_PREFIX: &str = "transcriber-debug";
const MAX_LOGS_AGE_DAYS: u64 = 30;
const MAX_LOGS_TOTAL_SIZE_BYTES: u64 = 100 * 1024 * 1024;

#[derive(Default)]
pub struct DebugLogRuntime {
    file_path: Mutex<Option<PathBuf>>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelRunLogContext {
    pub source: ModelRunSource,
    pub operation_id: String,
    pub history_record_id: Option<String>,
    pub recording_started_at: Option<String>,
    pub audio_duration_ms: Option<u64>,
    pub audio_file_name: Option<String>,
    pub audio_size_bytes: Option<usize>,
    pub audio_path: Option<String>,
}

impl ModelRunLogContext {
    pub fn settings_test() -> Self {
        Self {
            source: ModelRunSource::SettingsTest,
            operation_id: Uuid::new_v4().to_string(),
            history_record_id: None,
            recording_started_at: None,
            audio_duration_ms: None,
            audio_file_name: None,
            audio_size_bytes: None,
            audio_path: None,
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelRunSource {
    Dictation,
    HistoryRepeat,
    SettingsTest,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelRunStage {
    PostProcessing,
    SpeechToText,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DebugLogEvent<'a> {
    timestamp: String,
    event: &'a str,
    stage: Option<ModelRunStage>,
    #[serde(flatten)]
    context: Option<&'a ModelRunLogContext>,
    payload: serde_json::Value,
}

#[tauri::command]
pub fn open_debug_logs_folder(app: tauri::AppHandle) -> Result<(), String> {
    open_debug_logs_folder_inner(&app).map_err(AppError::into_message)
}

pub fn handle_logging_setting_changed(app: &tauri::AppHandle, is_enabled: bool) {
    reset_current_log_file(app);

    if is_enabled {
        let _ = write_event(
            app,
            "debugLogging.enabled",
            None,
            None,
            serde_json::json!({
                "isDebugLoggingEnabled": true,
            }),
        );
    }
}

pub fn log_model_event(
    app: &tauri::AppHandle,
    event: &'static str,
    stage: ModelRunStage,
    context: Option<&ModelRunLogContext>,
    payload: serde_json::Value,
) {
    let _ = write_event(app, event, Some(stage), context, payload);
}

pub fn log_event(
    app: &tauri::AppHandle,
    event: &'static str,
    context: Option<&ModelRunLogContext>,
    payload: serde_json::Value,
) {
    let _ = write_event(app, event, None, context, payload);
}

/// Записывает важное локальное диагностическое событие независимо от настройки
/// расширенного отладочного логирования. Используется только для безопасных
/// метаданных о сбоях, которые иначе невозможно расследовать после релиза.
pub fn log_critical_event(
    app: &tauri::AppHandle,
    event: &'static str,
    context: Option<&ModelRunLogContext>,
    payload: serde_json::Value,
) {
    let _ = write_event_forcefully(app, event, None, context, payload);
}

pub fn sanitized_headers(headers: &[crate::runner::HeaderSnapshot]) -> Vec<String> {
    headers.iter().map(|header| header.name.clone()).collect()
}

fn write_event(
    app: &tauri::AppHandle,
    event: &'static str,
    stage: Option<ModelRunStage>,
    context: Option<&ModelRunLogContext>,
    payload: serde_json::Value,
) -> AppResult<()> {
    if !settings::is_debug_logging_enabled(app)? {
        return Ok(());
    }

    write_event_forcefully(app, event, stage, context, payload)
}

fn write_event_forcefully(
    app: &tauri::AppHandle,
    event: &'static str,
    stage: Option<ModelRunStage>,
    context: Option<&ModelRunLogContext>,
    payload: serde_json::Value,
) -> AppResult<()> {
    cleanup_logs(app)?;

    let path = current_log_file_path(app)?;
    let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let entry = DebugLogEvent {
        timestamp: timestamp.clone(),
        event,
        stage,
        context,
        payload,
    };
    let json = serde_json::to_string_pretty(&entry)?;
    let header = event_header(&timestamp, event, context);
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;

    writeln!(file, "{header}")?;
    writeln!(file, "{json}")?;
    writeln!(file)?;

    Ok(())
}

fn current_log_file_path(app: &tauri::AppHandle) -> AppResult<PathBuf> {
    let runtime = app.state::<DebugLogRuntime>();
    let mut file_path = runtime
        .file_path
        .lock()
        .map_err(|_| AppError::from("Could not lock debug log state"))?;

    if let Some(path) = file_path.as_ref() {
        return Ok(path.clone());
    }

    let logs_dir = logs_dir(app)?;
    fs::create_dir_all(&logs_dir)?;

    let timestamp = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let file_name = sanitize_file_name(&format!("{LOG_FILE_PREFIX}-{timestamp}.log"));
    let path = logs_dir.join(file_name);

    *file_path = Some(path.clone());

    Ok(path)
}

fn reset_current_log_file(app: &tauri::AppHandle) {
    if let Ok(mut file_path) = app.state::<DebugLogRuntime>().file_path.lock() {
        *file_path = None;
    }
}

fn open_debug_logs_folder_inner(app: &tauri::AppHandle) -> AppResult<()> {
    let logs_dir = logs_dir(app)?;
    fs::create_dir_all(&logs_dir)?;

    let selected_path = app
        .state::<DebugLogRuntime>()
        .file_path
        .lock()
        .map_err(|_| AppError::from("Could not lock debug log state"))?
        .clone();

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;

        if let Some(path) = selected_path.filter(|path| path.exists()) {
            let absolute_path = fs::canonicalize(path)?;

            Command::new("explorer.exe")
                .raw_arg(format!("/select,\"{}\"", absolute_path.to_string_lossy()))
                .spawn()
                .map_err(|error| {
                    AppError::from(format!("Could not open debug log location: {error}"))
                })?;
        } else {
            Command::new("explorer.exe")
                .arg(logs_dir)
                .spawn()
                .map_err(|error| {
                    AppError::from(format!("Could not open debug logs folder: {error}"))
                })?;
        }
    }

    #[cfg(not(target_os = "windows"))]
    {
        let target = selected_path
            .filter(|path| path.exists())
            .and_then(|path| path.parent().map(Path::to_path_buf))
            .unwrap_or(logs_dir);

        Command::new("xdg-open")
            .arg(target)
            .spawn()
            .map_err(|error| {
                AppError::from(format!("Could not open debug logs folder: {error}"))
            })?;
    }

    Ok(())
}

fn cleanup_logs(app: &tauri::AppHandle) -> AppResult<()> {
    let logs_dir = logs_dir(app)?;

    if !logs_dir.exists() {
        return Ok(());
    }

    delete_expired_logs(&logs_dir)?;
    trim_logs_total_size(&logs_dir)
}

fn delete_expired_logs(logs_dir: &Path) -> AppResult<()> {
    let max_age = Duration::from_secs(MAX_LOGS_AGE_DAYS * 24 * 60 * 60);
    let now = SystemTime::now();

    for entry in log_entries(logs_dir)? {
        let metadata = entry.metadata()?;
        let Ok(modified_at) = metadata.modified() else {
            continue;
        };
        let Ok(age) = now.duration_since(modified_at) else {
            continue;
        };

        if age > max_age {
            let _ = fs::remove_file(entry.path());
        }
    }

    Ok(())
}

fn trim_logs_total_size(logs_dir: &Path) -> AppResult<()> {
    let mut entries = log_entries(logs_dir)?
        .into_iter()
        .filter_map(|entry| {
            let metadata = entry.metadata().ok()?;
            let modified_at = metadata.modified().ok()?;

            Some((entry.path(), metadata.len(), modified_at))
        })
        .collect::<Vec<_>>();
    let mut total_size = entries.iter().map(|(_, size, _)| *size).sum::<u64>();

    entries.sort_by_key(|(_, _, modified_at)| *modified_at);

    for (path, size, _) in entries {
        if total_size <= MAX_LOGS_TOTAL_SIZE_BYTES {
            break;
        }

        if fs::remove_file(path).is_ok() {
            total_size = total_size.saturating_sub(size);
        }
    }

    Ok(())
}

fn log_entries(logs_dir: &Path) -> AppResult<Vec<fs::DirEntry>> {
    let mut entries = Vec::new();

    for entry in fs::read_dir(logs_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|extension| extension.to_str()) == Some("log")
            && path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with(LOG_FILE_PREFIX))
        {
            entries.push(entry);
        }
    }

    Ok(entries)
}

fn logs_dir(app: &tauri::AppHandle) -> AppResult<PathBuf> {
    Ok(app.path().app_data_dir()?.join(LOGS_DIR_NAME))
}

fn event_header(timestamp: &str, event: &str, context: Option<&ModelRunLogContext>) -> String {
    let mut header = format!("{}\n{timestamp} {event}", "=".repeat(80),);

    if let Some(context) = context {
        header.push_str(&format!(" operationId={}", context.operation_id));

        if let Some(history_record_id) = &context.history_record_id {
            header.push_str(&format!(" historyRecordId={history_record_id}"));
        }
    }

    header.push('\n');
    header.push_str(&"-".repeat(80));

    header
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_event_without_secret_values() {
        let headers = vec![crate::runner::HeaderSnapshot {
            name: "X-Secret".to_string(),
            value: "secret-value".to_string(),
        }];
        let payload = serde_json::json!({
            "headers": sanitized_headers(&headers),
        });
        let content = serde_json::to_string(&payload).expect("payload should serialize");

        assert!(content.contains("X-Secret"));
        assert!(!content.contains("secret-value"));
    }

    #[test]
    fn event_header_includes_correlation_ids() {
        let context = ModelRunLogContext {
            source: ModelRunSource::Dictation,
            operation_id: "operation-id".to_string(),
            history_record_id: Some("history-id".to_string()),
            recording_started_at: None,
            audio_duration_ms: None,
            audio_file_name: None,
            audio_size_bytes: None,
            audio_path: None,
        };
        let header = event_header(
            "2026-06-21T18:42:12.123Z",
            "speechToText.response",
            Some(&context),
        );

        assert!(header.contains("operationId=operation-id"));
        assert!(header.contains("historyRecordId=history-id"));
    }

    #[test]
    fn sanitizes_log_file_name_timestamp() {
        let file_name = sanitize_file_name("transcriber-debug-2026-06-21T18:42:12.123Z.log");

        assert_eq!(file_name, "transcriber-debug-2026-06-21T18-42-12.123Z.log");
    }
}
