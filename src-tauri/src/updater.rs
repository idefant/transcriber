use serde::Serialize;
use std::sync::Mutex;
use tauri_plugin_updater::UpdaterExt;
use url::Url;

use crate::error::AppError;

const STABLE_ENDPOINT: &str = "https://idefant.github.io/transcriber/stable.json";
const UNSTABLE_ENDPOINT: &str = "https://idefant.github.io/transcriber/unstable.json";

/// A pending update that has been discovered but not yet installed.
/// Stored in managed state so `download_and_install_update` can retrieve
/// and consume it.
pub struct PendingUpdate(pub Mutex<Option<tauri_plugin_updater::Update>>);

impl Default for PendingUpdate {
    fn default() -> Self {
        Self(Mutex::new(None))
    }
}

/// Information about an available update returned to the frontend.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub version: String,
    pub notes: Option<String>,
}

/// Progress payload emitted to the frontend during download.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProgress {
    pub downloaded: u64,
    pub total: Option<u64>,
}

/// Checks whether an update is available.
///
/// If `offer_unstable` is `true`, queries the `unstable` channel
/// (includes alpha/beta/rc releases). Otherwise queries the `stable`
/// channel.
///
/// On success, stores the pending `Update` in managed state and returns
/// metadata about it. Returns `None` if already on the latest version.
#[tauri::command]
pub async fn check_for_update(
    app: tauri::AppHandle,
    offer_unstable: bool,
) -> Result<Option<UpdateInfo>, String> {
    let endpoint = if offer_unstable {
        UNSTABLE_ENDPOINT
    } else {
        STABLE_ENDPOINT
    };

    let url = Url::parse(endpoint).map_err(|e| AppError::Message(e.to_string()).into_message())?;

    let updater = app
        .updater_builder()
        .endpoints(vec![url])
        .map_err(|e| AppError::Message(e.to_string()).into_message())?
        .build()
        .map_err(|e| AppError::Message(e.to_string()).into_message())?;

    let update = updater
        .check()
        .await
        .map_err(|e| AppError::Message(e.to_string()).into_message())?;

    match update {
        Some(update) => {
            let info = UpdateInfo {
                version: update.version.clone(),
                notes: update.body.clone(),
            };
            *app.state::<PendingUpdate>().0.lock().unwrap() = Some(update);
            Ok(Some(info))
        }
        None => {
            *app.state::<PendingUpdate>().0.lock().unwrap() = None;
            Ok(None)
        }
    }
}

/// Downloads and installs the pending update, then restarts the app.
///
/// Must be called after a successful `check_for_update` that returned
/// `Some(...)`. Emits `updater://progress` events during download.
#[tauri::command]
pub async fn download_and_install_update(app: tauri::AppHandle) -> Result<(), String> {
    let update = app
        .state::<PendingUpdate>()
        .0
        .lock()
        .unwrap()
        .take()
        .ok_or_else(|| "No pending update found. Call check_for_update first.".to_string())?;

    let app_for_progress = app.clone();
    let mut downloaded: u64 = 0;

    update
        .download_and_install(
            |chunk_length, content_length| {
                downloaded += chunk_length as u64;
                let progress = UpdateProgress {
                    downloaded,
                    total: content_length,
                };
                let _ = app_for_progress.emit("updater://progress", progress);
            },
            || {},
        )
        .await
        .map_err(|e| e.to_string())?;

    app.restart();
}
