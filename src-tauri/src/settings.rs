use serde::{Deserialize, Serialize};

use crate::{
    autostart, debug_log, dictation,
    error::{AppError, AppResult},
    shortcut_hook, storage,
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

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TriggerMode {
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
    System,
}

impl Default for UiLanguage {
    fn default() -> Self {
        Self::System
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum EffectiveUiLanguage {
    En,
    Ru,
}

impl Default for EffectiveUiLanguage {
    fn default() -> Self {
        Self::En
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default)]
    theme_preference: ThemePreference,
    #[serde(default)]
    ui_language: UiLanguage,
    #[serde(default, skip_deserializing)]
    effective_ui_language: EffectiveUiLanguage,
    #[serde(default = "default_dictation_sounds_enabled")]
    are_dictation_sounds_enabled: bool,
    #[serde(default)]
    is_debug_logging_enabled: bool,
    #[serde(default = "default_launch_at_login_enabled")]
    is_launch_at_login_enabled: bool,
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
            effective_ui_language: resolve_effective_ui_language(&UiLanguage::default()),
            are_dictation_sounds_enabled: default_dictation_sounds_enabled(),
            is_debug_logging_enabled: false,
            is_launch_at_login_enabled: default_launch_at_login_enabled(),
            hotkey: default_hotkey(),
            trigger_mode: TriggerMode::default(),
        }
    }
}

impl AppSettings {
    pub fn hotkey(&self) -> &str {
        &self.hotkey
    }

    pub fn trigger_mode(&self) -> &TriggerMode {
        &self.trigger_mode
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettingsInput {
    theme_preference: Option<ThemePreference>,
    ui_language: Option<UiLanguage>,
    are_dictation_sounds_enabled: Option<bool>,
    is_debug_logging_enabled: Option<bool>,
    is_launch_at_login_enabled: Option<bool>,
    hotkey: Option<String>,
    trigger_mode: Option<TriggerMode>,
}

fn default_dictation_sounds_enabled() -> bool {
    true
}

fn default_launch_at_login_enabled() -> bool {
    true
}

fn default_hotkey() -> String {
    "Ctrl+Space".to_string()
}

#[tauri::command]
pub fn get_app_settings(app: tauri::AppHandle) -> Result<AppSettings, String> {
    get_app_settings_inner(&app).map_err(AppError::into_message)
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
    let previous_debug_logging_enabled = settings.is_debug_logging_enabled;

    if let Some(theme_preference) = input.theme_preference {
        settings.theme_preference = theme_preference;
    }

    if let Some(ui_language) = input.ui_language {
        settings.ui_language = ui_language;
    }

    if let Some(are_dictation_sounds_enabled) = input.are_dictation_sounds_enabled {
        settings.are_dictation_sounds_enabled = are_dictation_sounds_enabled;
    }

    if let Some(is_debug_logging_enabled) = input.is_debug_logging_enabled {
        settings.is_debug_logging_enabled = is_debug_logging_enabled;
    }

    if let Some(is_launch_at_login_enabled) = input.is_launch_at_login_enabled {
        settings.is_launch_at_login_enabled = is_launch_at_login_enabled;
    }

    if let Some(hotkey) = input.hotkey {
        settings.hotkey = shortcut_hook::normalize_hotkey(&hotkey)?;
    }

    if let Some(trigger_mode) = input.trigger_mode {
        settings.trigger_mode = trigger_mode;
    }

    settings.effective_ui_language = resolve_effective_ui_language(&settings.ui_language);

    save_app_settings(app, &settings)?;
    autostart::sync_launch_at_login(settings.is_launch_at_login_enabled)?;
    dictation::update_dictation_shortcut(app)?;

    if settings.is_debug_logging_enabled != previous_debug_logging_enabled {
        debug_log::handle_logging_setting_changed(app, settings.is_debug_logging_enabled);
    }

    Ok(settings)
}

fn get_app_settings_inner(app: &tauri::AppHandle) -> AppResult<AppSettings> {
    let settings = with_effective_ui_language(load_app_settings(app)?);

    autostart::sync_launch_at_login(settings.is_launch_at_login_enabled)?;

    Ok(settings)
}

pub fn load_app_settings(app: &tauri::AppHandle) -> AppResult<AppSettings> {
    let mut settings: AppSettings = storage::load_json_or_default(app, SETTINGS_FILE_NAME)?;

    settings.hotkey = shortcut_hook::normalize_hotkey(&settings.hotkey)?;

    Ok(settings)
}

fn save_app_settings(app: &tauri::AppHandle, settings: &AppSettings) -> AppResult<()> {
    storage::save_json(app, SETTINGS_FILE_NAME, settings)
}

pub fn get_effective_ui_language(app: &tauri::AppHandle) -> AppResult<EffectiveUiLanguage> {
    let settings = load_app_settings(app)?;

    Ok(resolve_effective_ui_language(&settings.ui_language))
}

pub fn is_debug_logging_enabled(app: &tauri::AppHandle) -> AppResult<bool> {
    Ok(load_app_settings(app)?.is_debug_logging_enabled)
}

fn with_effective_ui_language(mut settings: AppSettings) -> AppSettings {
    settings.effective_ui_language = resolve_effective_ui_language(&settings.ui_language);
    settings
}

fn resolve_effective_ui_language(ui_language: &UiLanguage) -> EffectiveUiLanguage {
    match ui_language {
        UiLanguage::En => EffectiveUiLanguage::En,
        UiLanguage::Ru => EffectiveUiLanguage::Ru,
        UiLanguage::System => get_system_ui_language(),
    }
}

fn get_system_ui_language() -> EffectiveUiLanguage {
    let Some(locale) = sys_locale::get_locale() else {
        return EffectiveUiLanguage::En;
    };

    if locale.to_lowercase().starts_with("ru") {
        EffectiveUiLanguage::Ru
    } else {
        EffectiveUiLanguage::En
    }
}
