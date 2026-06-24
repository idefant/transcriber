use std::fs;

use chrono::Local;
use serde::{de::DeserializeOwned, Serialize};
use tauri::Manager;

use crate::error::AppResult;

pub fn load_json_or_default<T>(app: &tauri::AppHandle, file_name: &str) -> AppResult<T>
where
    T: Default + DeserializeOwned,
{
    let path = app_data_file_path(app, file_name)?;

    if !path.exists() {
        return Ok(T::default());
    }

    let content = fs::read_to_string(&path)?;

    if content.trim().is_empty() {
        return Ok(T::default());
    }

    match serde_json::from_str(&content) {
        Ok(value) => Ok(value),
        Err(_) => {
            // Back up the corrupt/incompatible file rather than failing hard.
            // The caller gets T::default() so the domain stays functional.
            backup_corrupt_file(app, file_name);
            Ok(T::default())
        }
    }
}

/// Like `load_json_or_default`, but distinguishes between an absent file
/// (returns `T::default()`, e.g. fresh install) and a present-but-corrupt file
/// (backs it up and returns an `Err`).
///
/// Use this for domains where silent fallback to an empty default would cause
/// cascading failures or unacceptable data loss (providers, history).
pub fn load_json_strict<T>(app: &tauri::AppHandle, file_name: &str) -> AppResult<T>
where
    T: Default + DeserializeOwned,
{
    let path = app_data_file_path(app, file_name)?;

    if !path.exists() {
        return Ok(T::default());
    }

    let content = fs::read_to_string(&path)?;

    if content.trim().is_empty() {
        return Ok(T::default());
    }

    match serde_json::from_str(&content) {
        Ok(value) => Ok(value),
        Err(err) => {
            backup_corrupt_file(app, file_name);
            Err(format!("{file_name} is corrupted and has been backed up: {err}").into())
        }
    }
}

fn backup_corrupt_file(app: &tauri::AppHandle, file_name: &str) {
    let timestamp = Local::now().format("%Y%m%d-%H%M%S");
    let backup_name = format!("{}.corrupt-{}", file_name, timestamp);
    if let (Ok(src), Ok(dst)) = (
        app_data_file_path(app, file_name),
        app_data_file_path(app, &backup_name),
    ) {
        let _ = fs::rename(&src, &dst);
    }
}

pub fn save_json<T>(app: &tauri::AppHandle, file_name: &str, value: &T) -> AppResult<()>
where
    T: Serialize + ?Sized,
{
    let path = app_data_file_path(app, file_name)?;
    let content = serde_json::to_string_pretty(value)?;

    // Write to a temp file then rename for atomicity — avoids partial writes on crash.
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, content)?;
    fs::rename(&tmp_path, &path)?;

    Ok(())
}

fn app_data_file_path(app: &tauri::AppHandle, file_name: &str) -> AppResult<std::path::PathBuf> {
    let app_data_dir = app.path().app_data_dir()?;

    fs::create_dir_all(&app_data_dir)?;

    Ok(app_data_dir.join(file_name))
}
