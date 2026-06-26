use std::sync::{
    atomic::{AtomicU64, Ordering},
    Mutex,
};

use serde::Serialize;
use tauri::{Emitter, Manager};
use uuid::Uuid;

use crate::{
    debug_log::{ModelRunLogContext, ModelRunSource},
    error::{AppError, AppResult},
    history, keyboard, overlay,
    processing::load_processing_config,
    recording::{self, AudioRecording, RecordedAudio},
    runner,
    settings::{self, TriggerMode},
    shortcut_hook::{self, ShortcutState},
};

#[derive(Default)]
pub struct DictationRuntime {
    session: Mutex<DictationSession>,
    next_session_id: AtomicU64,
}

enum DictationSession {
    Idle,
    Recording { id: u64, recording: AudioRecording },
    Transcribing { id: u64 },
    Processing { id: u64 },
    Cancelled { id: u64 },
}

impl Default for DictationSession {
    fn default() -> Self {
        Self::Idle
    }
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
}

/// Result of processing a recording, used to decide how the overlay ends.
enum DictationOutcome {
    /// Success (text inserted) or a cancelled/superseded session — hide overlay.
    Completed,
    /// Speech-to-text failed; no text was inserted. Show the red error overlay.
    /// `record_id` is `Some` when a history record was saved.
    SttError { record_id: Option<String> },
    /// Post-processing failed but the speech-to-text text was inserted. Show the
    /// amber warning overlay linked to the saved record.
    PostProcessError { record_id: String },
}

pub fn register_dictation_shortcut(app: &tauri::AppHandle) -> AppResult<()> {
    let settings = settings::load_app_settings(app)?;

    shortcut_hook::install_dictation_shortcut(app.clone(), settings.hotkey())?;

    Ok(())
}

pub fn update_dictation_shortcut(app: &tauri::AppHandle) -> AppResult<()> {
    let settings = settings::load_app_settings(app)?;

    shortcut_hook::set_dictation_hotkey(settings.hotkey())
}

pub fn handle_shortcut_event(app: &tauri::AppHandle, state: ShortcutState) {
    let Ok(settings) = settings::load_app_settings(app) else {
        return;
    };

    match (settings.trigger_mode(), state) {
        (TriggerMode::Hold, ShortcutState::Pressed) => {
            start_dictation(app.clone());
        }
        (TriggerMode::Hold, ShortcutState::Released) => {
            stop_dictation(app.clone());
        }
        (TriggerMode::Press, ShortcutState::Pressed) => {
            toggle_dictation(app.clone());
        }
        _ => {}
    }
}

pub fn handle_cancel_shortcut(app: &tauri::AppHandle) {
    if let Err(error) = cancel_dictation_inner(app.clone()) {
        emit_dictation_error(app, error.into_message());
    }
}

#[tauri::command]
pub fn cancel_dictation(app: tauri::AppHandle) -> Result<(), String> {
    cancel_dictation_inner(app).map_err(AppError::into_message)
}

#[tauri::command]
pub fn dictation_shortcut_pressed(app: tauri::AppHandle) {
    handle_shortcut_event(&app, ShortcutState::Pressed);
}

#[tauri::command]
pub fn dictation_shortcut_released(app: tauri::AppHandle) {
    handle_shortcut_event(&app, ShortcutState::Released);
}

fn toggle_dictation(app: tauri::AppHandle) {
    let is_recording = app
        .state::<DictationRuntime>()
        .session
        .lock()
        .map(|session| matches!(*session, DictationSession::Recording { .. }))
        .unwrap_or(false);

    if is_recording {
        stop_dictation(app);
    } else {
        start_dictation(app);
    }
}

fn start_dictation(app: tauri::AppHandle) {
    if let Err(error) = start_dictation_inner(&app) {
        emit_dictation_error(&app, error.into_message());
        let _ = overlay::hide_recording_overlay(&app);
    }
}

fn start_dictation_inner(app: &tauri::AppHandle) -> AppResult<()> {
    let runtime = app.state::<DictationRuntime>();
    let mut session = runtime
        .session
        .lock()
        .map_err(|_| AppError::from("Could not lock dictation state"))?;

    if !matches!(
        *session,
        DictationSession::Idle | DictationSession::Cancelled { .. }
    ) {
        return Ok(());
    }

    overlay::show_recording_overlay(app)?;

    let recording = recording::start_recording(app.clone())?;
    let id = runtime.next_session_id.fetch_add(1, Ordering::Relaxed) + 1;

    *session = DictationSession::Recording { id, recording };

    // Arm the cancel hotkey for the duration of this session. A missing or
    // empty cancel hotkey silently disarms the hook (no key is consumed).
    if let Ok(app_settings) = settings::load_app_settings(app) {
        if let Err(error) = shortcut_hook::arm_cancel_hotkey(app_settings.cancel_hotkey()) {
            emit_dictation_error(app, error.into_message());
        }
    }

    // Notify the frontend that a session is now active (used to gate in-app cancel hotkey).
    emit_dictation_session(app, true);

    Ok(())
}

fn stop_dictation(app: tauri::AppHandle) {
    let (id, recording) = match take_recording(&app) {
        Ok(Some((id, recording))) => (id, recording),
        Ok(None) => return,
        Err(error) => {
            emit_dictation_error(&app, error.into_message());
            return;
        }
    };

    // Stop the audio stream synchronously so the OS microphone indicator turns
    // off and system audio un-mutes before STT/post-processing begins.
    let audio = match recording.stop() {
        Ok(audio) => audio,
        Err(error) => {
            reset_session(&app, id, true);
            emit_dictation_error(&app, error.into_message());
            return;
        }
    };

    tauri::async_runtime::spawn(process_recording(app, id, audio));
}

fn take_recording(app: &tauri::AppHandle) -> AppResult<Option<(u64, AudioRecording)>> {
    let runtime = app.state::<DictationRuntime>();
    let mut session = runtime
        .session
        .lock()
        .map_err(|_| AppError::from("Could not lock dictation state"))?;

    // Guard before replace: std::mem::replace writes Idle unconditionally, so we
    // must confirm the state is Recording before calling it, otherwise a spurious
    // stop_dictation (e.g. hotkey release while transcribing) would corrupt the
    // state and prevent reset_session from ever hiding the overlay.
    if !matches!(*session, DictationSession::Recording { .. }) {
        return Ok(None);
    }

    let DictationSession::Recording { id, recording } =
        std::mem::replace(&mut *session, DictationSession::Idle)
    else {
        unreachable!()
    };

    *session = DictationSession::Transcribing { id };

    Ok(Some((id, recording)))
}

async fn process_recording(app: tauri::AppHandle, id: u64, audio: RecordedAudio) {
    let outcome = process_recording_inner(&app, id, audio).await;

    // A cancelled or superseded session has already hidden its overlay via
    // cancel_dictation_inner; just make sure the session state is reset.
    if !is_current_session(&app, id) {
        reset_session(&app, id, true);
        return;
    }

    match outcome {
        Ok(DictationOutcome::Completed) => {
            reset_session(&app, id, true);
        }
        Ok(DictationOutcome::SttError { record_id }) => {
            reset_session(&app, id, false);
            let _ = overlay::show_error_overlay(&app, record_id);
        }
        Ok(DictationOutcome::PostProcessError { record_id }) => {
            reset_session(&app, id, false);
            let _ = overlay::show_warning_overlay(&app, Some(record_id));
        }
        Err(error) => {
            emit_dictation_error(&app, error.into_message());
            reset_session(&app, id, false);
            let _ = overlay::show_error_overlay(&app, None);
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
        return Err(AppError::from(
            "Speech-to-text provider and model are not selected",
        ));
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
        set_processing(app, id)?;
        overlay::show_processing_overlay(app)?;
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

                // Post-processing failed, but the speech-to-text text is valid —
                // still insert it so the user does not lose their dictation.
                if is_current_session(app, id) {
                    keyboard::paste_text(&stt_text).await?;
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
        keyboard::paste_text(&final_text).await?;
    }

    Ok(DictationOutcome::Completed)
}

fn set_processing(app: &tauri::AppHandle, id: u64) -> AppResult<()> {
    let runtime = app.state::<DictationRuntime>();
    let mut session = runtime
        .session
        .lock()
        .map_err(|_| AppError::from("Could not lock dictation state"))?;

    if matches!(*session, DictationSession::Transcribing { id: current } if current == id) {
        *session = DictationSession::Processing { id };
    }

    Ok(())
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

/// Reset a finished session to idle. The overlay is hidden only when
/// `hide_overlay` is set — the error/warning notifications keep it visible and
/// manage their own dismissal instead.
fn reset_session(app: &tauri::AppHandle, id: u64, hide_overlay: bool) {
    if let Ok(mut session) = app.state::<DictationRuntime>().session.lock() {
        if matches!(
            *session,
            DictationSession::Transcribing { id: current }
                | DictationSession::Processing { id: current }
                | DictationSession::Cancelled { id: current }
                if current == id
        ) {
            *session = DictationSession::Idle;
            shortcut_hook::disarm_cancel_hotkey();
            emit_dictation_session(app, false);
            if hide_overlay {
                let _ = overlay::hide_recording_overlay(app);
            }
        }
    }
}

fn cancel_dictation_inner(app: tauri::AppHandle) -> AppResult<()> {
    let runtime = app.state::<DictationRuntime>();
    let mut session = runtime
        .session
        .lock()
        .map_err(|_| AppError::from("Could not lock dictation state"))?;

    match std::mem::replace(&mut *session, DictationSession::Idle) {
        DictationSession::Idle | DictationSession::Cancelled { .. } => {}
        DictationSession::Recording { .. } => {
            shortcut_hook::disarm_cancel_hotkey();
            emit_dictation_session(&app, false);
            let _ = overlay::hide_recording_overlay(&app);
        }
        DictationSession::Transcribing { id } | DictationSession::Processing { id } => {
            *session = DictationSession::Cancelled { id };
            shortcut_hook::disarm_cancel_hotkey();
            emit_dictation_session(&app, false);
            let _ = overlay::hide_recording_overlay(&app);
        }
    }

    Ok(())
}

fn emit_dictation_error(app: &tauri::AppHandle, message: String) {
    let _ = app.emit("dictation-error", DictationErrorPayload { message });
}

fn emit_dictation_session(app: &tauri::AppHandle, active: bool) {
    let _ = app.emit("dictation-session", DictationSessionPayload { active });
}
