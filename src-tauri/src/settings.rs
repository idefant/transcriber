use serde::{Deserialize, Serialize};

use crate::{
    error::{AppError, AppResult},
    storage,
};

const SETTINGS_FILE_NAME: &str = "settings.json";

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum ThemePreference {
    Auto,
    Dark,
    Light,
}

impl Default for ThemePreference {
    fn default() -> Self {
        Self::Light
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum TriggerMode {
    Hold,
    Press,
}

impl Default for TriggerMode {
    fn default() -> Self {
        Self::Press
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum UiLanguage {
    En,
    Ru,
}

impl Default for UiLanguage {
    fn default() -> Self {
        Self::Ru
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default)]
    theme_preference: ThemePreference,
    #[serde(default)]
    ui_language: UiLanguage,
    #[serde(default = "default_dictation_sounds_enabled")]
    are_dictation_sounds_enabled: bool,
    #[serde(default = "default_hotkey")]
    hotkey: String,
    #[serde(default)]
    trigger_mode: TriggerMode,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme_preference: ThemePreference::default(),
            ui_language: UiLanguage::default(),
            are_dictation_sounds_enabled: default_dictation_sounds_enabled(),
            hotkey: default_hotkey(),
            trigger_mode: TriggerMode::default(),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettingsInput {
    theme_preference: Option<ThemePreference>,
    ui_language: Option<UiLanguage>,
    are_dictation_sounds_enabled: Option<bool>,
    hotkey: Option<String>,
    trigger_mode: Option<TriggerMode>,
}

fn default_dictation_sounds_enabled() -> bool {
    true
}

fn default_hotkey() -> String {
    "Ctrl + Shift + Space".to_string()
}

#[tauri::command]
pub fn get_app_settings(app: tauri::AppHandle) -> Result<AppSettings, String> {
    load_app_settings(&app).map_err(AppError::into_message)
}

#[tauri::command]
pub fn update_app_settings(
    app: tauri::AppHandle,
    input: AppSettingsInput,
) -> Result<AppSettings, String> {
    update_app_settings_inner(&app, input).map_err(AppError::into_message)
}

fn update_app_settings_inner(
    app: &tauri::AppHandle,
    input: AppSettingsInput,
) -> AppResult<AppSettings> {
    let mut settings = load_app_settings(app)?;

    if let Some(theme_preference) = input.theme_preference {
        settings.theme_preference = theme_preference;
    }

    if let Some(ui_language) = input.ui_language {
        settings.ui_language = ui_language;
    }

    if let Some(are_dictation_sounds_enabled) = input.are_dictation_sounds_enabled {
        settings.are_dictation_sounds_enabled = are_dictation_sounds_enabled;
    }

    if let Some(hotkey) = input.hotkey {
        settings.hotkey = hotkey.trim().to_string();
    }

    if let Some(trigger_mode) = input.trigger_mode {
        settings.trigger_mode = trigger_mode;
    }

    save_app_settings(app, &settings)?;

    Ok(settings)
}

fn load_app_settings(app: &tauri::AppHandle) -> AppResult<AppSettings> {
    storage::load_json_or_default(app, SETTINGS_FILE_NAME)
}

fn save_app_settings(app: &tauri::AppHandle, settings: &AppSettings) -> AppResult<()> {
    storage::save_json(app, SETTINGS_FILE_NAME, settings)
}
