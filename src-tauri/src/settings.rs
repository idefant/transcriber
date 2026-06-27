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

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OverlayVariant {
    Bottom,
    Center,
}

impl Default for OverlayVariant {
    fn default() -> Self {
        Self::Center
    }
}

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OverlayScreenMode {
    All,
    Cursor,
}

impl Default for OverlayScreenMode {
    fn default() -> Self {
        Self::All
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
    #[serde(default = "default_mute_while_recording_enabled")]
    is_mute_while_recording_enabled: bool,
    #[serde(default)]
    is_debug_logging_enabled: bool,
    #[serde(default = "default_launch_at_login_enabled")]
    is_launch_at_login_enabled: bool,
    #[serde(default = "default_hotkey")]
    hotkey: String,
    #[serde(default = "default_cancel_hotkey")]
    cancel_hotkey: String,
    #[serde(default)]
    copy_latest_hotkey: String,
    #[serde(default)]
    paste_latest_hotkey: String,
    #[serde(default)]
    repeat_latest_hotkey: String,
    #[serde(default)]
    trigger_mode: TriggerMode,
    #[serde(default)]
    overlay_variant: OverlayVariant,
    #[serde(default)]
    overlay_screen_mode: OverlayScreenMode,
    #[serde(default)]
    is_offer_unstable_versions_enabled: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme_preference: ThemePreference::default(),
            ui_language: UiLanguage::default(),
            effective_ui_language: resolve_effective_ui_language(&UiLanguage::default()),
            is_mute_while_recording_enabled: default_mute_while_recording_enabled(),
            is_debug_logging_enabled: false,
            is_launch_at_login_enabled: default_launch_at_login_enabled(),
            hotkey: default_hotkey(),
            cancel_hotkey: default_cancel_hotkey(),
            copy_latest_hotkey: String::new(),
            paste_latest_hotkey: String::new(),
            repeat_latest_hotkey: String::new(),
            trigger_mode: TriggerMode::default(),
            overlay_variant: OverlayVariant::default(),
            overlay_screen_mode: OverlayScreenMode::default(),
            is_offer_unstable_versions_enabled: false,
        }
    }
}

impl AppSettings {
    pub fn is_mute_while_recording_enabled(&self) -> bool {
        self.is_mute_while_recording_enabled
    }

    pub fn hotkey(&self) -> &str {
        &self.hotkey
    }

    pub fn cancel_hotkey(&self) -> &str {
        &self.cancel_hotkey
    }

    pub fn paste_latest_hotkey(&self) -> &str {
        &self.paste_latest_hotkey
    }

    pub fn copy_latest_hotkey(&self) -> &str {
        &self.copy_latest_hotkey
    }

    pub fn repeat_latest_hotkey(&self) -> &str {
        &self.repeat_latest_hotkey
    }

    pub fn trigger_mode(&self) -> &TriggerMode {
        &self.trigger_mode
    }

    pub fn overlay_variant(&self) -> &OverlayVariant {
        &self.overlay_variant
    }

    pub fn overlay_screen_mode(&self) -> &OverlayScreenMode {
        &self.overlay_screen_mode
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettingsInput {
    theme_preference: Option<ThemePreference>,
    ui_language: Option<UiLanguage>,
    is_mute_while_recording_enabled: Option<bool>,
    is_debug_logging_enabled: Option<bool>,
    is_launch_at_login_enabled: Option<bool>,
    hotkey: Option<String>,
    cancel_hotkey: Option<String>,
    copy_latest_hotkey: Option<String>,
    paste_latest_hotkey: Option<String>,
    repeat_latest_hotkey: Option<String>,
    trigger_mode: Option<TriggerMode>,
    overlay_variant: Option<OverlayVariant>,
    overlay_screen_mode: Option<OverlayScreenMode>,
    is_offer_unstable_versions_enabled: Option<bool>,
}

fn default_mute_while_recording_enabled() -> bool {
    true
}

fn default_launch_at_login_enabled() -> bool {
    true
}

fn default_hotkey() -> String {
    "Ctrl+Space".to_string()
}

fn default_cancel_hotkey() -> String {
    "Ctrl+Z".to_string()
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

    if let Some(is_mute_while_recording_enabled) = input.is_mute_while_recording_enabled {
        settings.is_mute_while_recording_enabled = is_mute_while_recording_enabled;
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

    if let Some(cancel_hotkey) = input.cancel_hotkey {
        settings.cancel_hotkey = if cancel_hotkey.trim().is_empty() {
            String::new()
        } else {
            shortcut_hook::normalize_hotkey(&cancel_hotkey)?
        };
    }

    if let Some(paste_latest_hotkey) = input.paste_latest_hotkey {
        settings.paste_latest_hotkey = if paste_latest_hotkey.trim().is_empty() {
            String::new()
        } else {
            shortcut_hook::normalize_hotkey(&paste_latest_hotkey)?
        };
    }

    if let Some(copy_latest_hotkey) = input.copy_latest_hotkey {
        settings.copy_latest_hotkey = if copy_latest_hotkey.trim().is_empty() {
            String::new()
        } else {
            shortcut_hook::normalize_hotkey(&copy_latest_hotkey)?
        };
    }

    if let Some(repeat_latest_hotkey) = input.repeat_latest_hotkey {
        settings.repeat_latest_hotkey = if repeat_latest_hotkey.trim().is_empty() {
            String::new()
        } else {
            shortcut_hook::normalize_hotkey(&repeat_latest_hotkey)?
        };
    }

    if let Some(trigger_mode) = input.trigger_mode {
        settings.trigger_mode = trigger_mode;
    }

    if let Some(overlay_variant) = input.overlay_variant {
        settings.overlay_variant = overlay_variant;
    }

    if let Some(overlay_screen_mode) = input.overlay_screen_mode {
        settings.overlay_screen_mode = overlay_screen_mode;
    }

    if let Some(is_offer_unstable_versions_enabled) = input.is_offer_unstable_versions_enabled {
        settings.is_offer_unstable_versions_enabled = is_offer_unstable_versions_enabled;
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

    if !settings.cancel_hotkey.trim().is_empty() {
        settings.cancel_hotkey = shortcut_hook::normalize_hotkey(&settings.cancel_hotkey)?;
    }

    if !settings.paste_latest_hotkey.trim().is_empty() {
        settings.paste_latest_hotkey =
            shortcut_hook::normalize_hotkey(&settings.paste_latest_hotkey)?;
    }

    if !settings.copy_latest_hotkey.trim().is_empty() {
        settings.copy_latest_hotkey =
            shortcut_hook::normalize_hotkey(&settings.copy_latest_hotkey)?;
    }

    if !settings.repeat_latest_hotkey.trim().is_empty() {
        settings.repeat_latest_hotkey =
            shortcut_hook::normalize_hotkey(&settings.repeat_latest_hotkey)?;
    }

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
