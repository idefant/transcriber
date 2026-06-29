use serde::{Deserialize, Serialize};

use crate::{
    catalog::{model_by_key, ModelTask},
    error::{AppError, AppResult},
    providers::find_provider_kind,
    settings::{get_effective_ui_language, EffectiveUiLanguage},
    storage,
};

const PROCESSING_FILE_NAME: &str = "processing.json";
const PROMPTS_JSON: &str = include_str!("../../resources/promps.json");

// ── Stored / public structs ───────────────────────────────────────────────────

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SttConfig {
    #[serde(default)]
    pub provider_id: Option<String>,
    #[serde(default)]
    pub model_key: Option<String>,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub use_custom_prompt: bool,
    /// Custom system prompt override. Empty string sends no prompt to the API.
    #[serde(default, alias = "prompt")]
    pub system_prompt: String,
}

impl Default for SttConfig {
    fn default() -> Self {
        Self {
            provider_id: None,
            model_key: None,
            language: default_language(),
            use_custom_prompt: false,
            system_prompt: String::new(),
        }
    }
}

impl SttConfig {
    /// Effective system prompt template (still contains `{{...}}` placeholders).
    pub fn effective_system_prompt(&self) -> AppResult<String> {
        if self.use_custom_prompt {
            Ok(self.system_prompt.clone())
        } else {
            default_stt_system_prompt(&self.language)
        }
    }
}

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostProcessConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub provider_id: Option<String>,
    #[serde(default)]
    pub model_key: Option<String>,
    #[serde(default)]
    pub use_custom_prompts: bool,
    /// Custom system prompt override. Empty string sends no system message.
    #[serde(default, alias = "prompt")]
    pub system_prompt: String,
    /// Custom user prompt template override. Empty means "use the default".
    #[serde(default)]
    pub user_prompt_template: String,
}

impl PostProcessConfig {
    /// Effective system prompt template (still contains `{{...}}` placeholders).
    pub fn effective_system_prompt(&self, language: &EffectiveUiLanguage) -> AppResult<String> {
        if self.use_custom_prompts {
            Ok(self.system_prompt.clone())
        } else {
            default_post_process_system_prompt(language)
        }
    }

    /// Effective user prompt template (still contains `{{...}}` placeholders).
    pub fn effective_user_template(&self) -> AppResult<String> {
        if self.use_custom_prompts && !self.user_prompt_template.trim().is_empty() {
            Ok(self.user_prompt_template.clone())
        } else {
            default_post_process_user_template()
        }
    }
}

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingConfig {
    #[serde(default)]
    pub stt: SttConfig,
    #[serde(default)]
    pub post_process: PostProcessConfig,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DefaultPrompts {
    stt_system: String,
    post_process_system: String,
    post_process_user_template: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PromptDefaults {
    stt_system: LocalizedPrompts,
    post_process: PostProcessPromptDefaults,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PostProcessPromptDefaults {
    system: LocalizedPrompts,
    user_template: String,
}

#[derive(Deserialize)]
struct LocalizedPrompts {
    en: String,
    ru: String,
}

// ── Input structs (partial updates) ──────────────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SttConfigInput {
    provider_id: Option<Option<String>>,
    model_key: Option<Option<String>>,
    language: Option<String>,
    use_custom_prompt: Option<bool>,
    system_prompt: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostProcessConfigInput {
    enabled: Option<bool>,
    provider_id: Option<Option<String>>,
    model_key: Option<Option<String>>,
    use_custom_prompts: Option<bool>,
    system_prompt: Option<String>,
    user_prompt_template: Option<String>,
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_processing_config(app: tauri::AppHandle) -> Result<ProcessingConfig, String> {
    load_processing_config(&app).map_err(AppError::into_message)
}

#[tauri::command]
pub fn get_default_prompts(app: tauri::AppHandle) -> Result<DefaultPrompts, String> {
    let config = load_processing_config(&app).map_err(AppError::into_message)?;
    let ui_language = get_effective_ui_language(&app).map_err(AppError::into_message)?;

    default_prompts_for_config(&config, &ui_language).map_err(AppError::into_message)
}

pub fn default_prompts_for_config(
    config: &ProcessingConfig,
    ui_language: &EffectiveUiLanguage,
) -> AppResult<DefaultPrompts> {
    Ok(DefaultPrompts {
        stt_system: default_stt_system_prompt(&config.stt.language)?,
        post_process_system: default_post_process_system_prompt(ui_language)?,
        post_process_user_template: default_post_process_user_template()?,
    })
}

#[tauri::command]
pub fn update_stt_config(
    app: tauri::AppHandle,
    input: SttConfigInput,
) -> Result<ProcessingConfig, String> {
    update_stt_config_inner(&app, input).map_err(AppError::into_message)
}

#[tauri::command]
pub fn update_post_process_config(
    app: tauri::AppHandle,
    input: PostProcessConfigInput,
) -> Result<ProcessingConfig, String> {
    update_post_process_config_inner(&app, input).map_err(AppError::into_message)
}

// ── Inner functions ───────────────────────────────────────────────────────────

fn update_stt_config_inner(
    app: &tauri::AppHandle,
    input: SttConfigInput,
) -> AppResult<ProcessingConfig> {
    let mut config = load_processing_config(app)?;

    apply_optional_string_patch(&mut config.stt.provider_id, input.provider_id);
    apply_optional_string_patch(&mut config.stt.model_key, input.model_key);

    if let Some(language) = input.language {
        config.stt.language = language.trim().to_string();
    }

    if let Some(use_custom_prompt) = input.use_custom_prompt {
        config.stt.use_custom_prompt = use_custom_prompt;
    }

    if let Some(system_prompt) = input.system_prompt {
        config.stt.system_prompt = system_prompt;
    }

    normalize_processing_config(app, &mut config)?;
    save_processing_config(app, &config)?;

    Ok(config)
}

fn update_post_process_config_inner(
    app: &tauri::AppHandle,
    input: PostProcessConfigInput,
) -> AppResult<ProcessingConfig> {
    let mut config = load_processing_config(app)?;

    if let Some(enabled) = input.enabled {
        config.post_process.enabled = enabled;
    }

    apply_optional_string_patch(&mut config.post_process.provider_id, input.provider_id);
    apply_optional_string_patch(&mut config.post_process.model_key, input.model_key);

    if let Some(use_custom_prompts) = input.use_custom_prompts {
        config.post_process.use_custom_prompts = use_custom_prompts;
    }

    if let Some(system_prompt) = input.system_prompt {
        config.post_process.system_prompt = system_prompt;
    }

    if let Some(user_prompt_template) = input.user_prompt_template {
        config.post_process.user_prompt_template = user_prompt_template;
    }

    normalize_processing_config(app, &mut config)?;
    save_processing_config(app, &config)?;

    Ok(config)
}

pub fn load_processing_config(app: &tauri::AppHandle) -> AppResult<ProcessingConfig> {
    let mut config = storage::load_json_or_default(app, PROCESSING_FILE_NAME)?;

    if normalize_processing_config(app, &mut config)? {
        save_processing_config(app, &config)?;
    }

    Ok(config)
}

fn save_processing_config(app: &tauri::AppHandle, config: &ProcessingConfig) -> AppResult<()> {
    storage::save_json(app, PROCESSING_FILE_NAME, config)
}

fn normalize_optional_string(value: String) -> Option<String> {
    let trimmed = value.trim().to_string();

    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn apply_optional_string_patch(target: &mut Option<String>, patch: Option<Option<String>>) {
    if let Some(value) = patch {
        *target = value.and_then(normalize_optional_string);
    }
}

fn normalize_processing_config(
    app: &tauri::AppHandle,
    config: &mut ProcessingConfig,
) -> AppResult<bool> {
    let mut changed = false;

    changed |= normalize_model_selection(
        app,
        &mut config.stt.provider_id,
        &mut config.stt.model_key,
        ModelTask::Stt,
    )?;
    changed |= normalize_model_selection(
        app,
        &mut config.post_process.provider_id,
        &mut config.post_process.model_key,
        ModelTask::PostProcess,
    )?;

    Ok(changed)
}

fn normalize_model_selection(
    app: &tauri::AppHandle,
    provider_id: &mut Option<String>,
    model_key: &mut Option<String>,
    task: ModelTask,
) -> AppResult<bool> {
    let Some(provider_id_value) = provider_id.as_deref() else {
        return Ok(model_key.take().is_some());
    };

    let Some(provider_kind) = find_provider_kind(app, provider_id_value)? else {
        let provider_cleared = provider_id.take().is_some();
        let model_cleared = model_key.take().is_some();

        return Ok(provider_cleared || model_cleared);
    };

    let is_valid = model_key
        .as_deref()
        .and_then(model_by_key)
        .is_some_and(|model| model.task == task && model.entry_for(provider_kind).is_some());

    if is_valid {
        return Ok(false);
    }

    Ok(model_key.take().is_some())
}

fn default_language() -> String {
    "auto".to_string()
}

fn default_stt_system_prompt(language: &str) -> AppResult<String> {
    let prompt_language = match language.trim().to_ascii_lowercase().as_str() {
        "ru" => EffectiveUiLanguage::Ru,
        "en" => EffectiveUiLanguage::En,
        _ => EffectiveUiLanguage::En,
    };

    localized_prompt(load_prompt_defaults()?.stt_system, &prompt_language)
}

fn default_post_process_system_prompt(language: &EffectiveUiLanguage) -> AppResult<String> {
    localized_prompt(load_prompt_defaults()?.post_process.system, language)
}

fn default_post_process_user_template() -> AppResult<String> {
    Ok(load_prompt_defaults()?.post_process.user_template)
}

fn localized_prompt(
    prompts: LocalizedPrompts,
    language: &EffectiveUiLanguage,
) -> AppResult<String> {
    Ok(match language {
        EffectiveUiLanguage::En => prompts.en,
        EffectiveUiLanguage::Ru => prompts.ru,
    })
}

fn load_prompt_defaults() -> AppResult<PromptDefaults> {
    serde_json::from_str(PROMPTS_JSON)
        .map_err(|error| format!("Invalid prompt defaults: {error}").into())
}
