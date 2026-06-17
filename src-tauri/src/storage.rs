use std::fs;

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

    let content = fs::read_to_string(path)?;

    if content.trim().is_empty() {
        return Ok(T::default());
    }

    Ok(serde_json::from_str(&content)?)
}

pub fn save_json<T>(app: &tauri::AppHandle, file_name: &str, value: &T) -> AppResult<()>
where
    T: Serialize + ?Sized,
{
    let path = app_data_file_path(app, file_name)?;
    let content = serde_json::to_string_pretty(value)?;

    fs::write(path, content)?;

    Ok(())
}

fn app_data_file_path(app: &tauri::AppHandle, file_name: &str) -> AppResult<std::path::PathBuf> {
    let app_data_dir = app.path().app_data_dir()?;

    fs::create_dir_all(&app_data_dir)?;

    Ok(app_data_dir.join(file_name))
}
