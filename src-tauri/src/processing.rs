use serde::{Deserialize, Serialize};

use crate::{
    error::{AppError, AppResult},
    settings::{get_effective_ui_language, EffectiveUiLanguage},
    storage,
};

const PROCESSING_FILE_NAME: &str = "processing.json";
const POST_PROCESS_PROMPTS_JSON: &str =
    include_str!("../../scripts/model-testing/post-process-prompts.json");

// Default prompts. These are templates: `{{...}}` placeholders are substituted
// at execution time (see runner.rs). They are the single source of truth and are
// exposed to the frontend through `get_default_prompts` for display.
const DEFAULT_STT_SYSTEM_PROMPT_EN: &str =
    "Custom Dictionary (use these exact spellings when they appear in the text): {{STT_DICTIONARY}}";
const DEFAULT_STT_SYSTEM_PROMPT_RU: &str =
    "Пользовательский словарь (используй эти точные написания, когда они встречаются в тексте): {{STT_DICTIONARY}}";

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
    /// Custom system prompt override. Empty means "use the default".
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
    pub fn effective_system_prompt(&self, language: &EffectiveUiLanguage) -> &str {
        if self.use_custom_prompt && !self.system_prompt.trim().is_empty() {
            &self.system_prompt
        } else {
            default_stt_system_prompt(language)
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
    /// Custom system prompt override. Empty means "use the default".
    #[serde(default, alias = "prompt")]
    pub system_prompt: String,
    /// Custom user prompt template override. Empty means "use the default".
    #[serde(default)]
    pub user_prompt_template: String,
}

impl PostProcessConfig {
    /// Effective system prompt template (still contains `{{...}}` placeholders).
    pub fn effective_system_prompt(&self, language: &EffectiveUiLanguage) -> AppResult<String> {
        if self.use_custom_prompts && !self.system_prompt.trim().is_empty() {
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
struct PostProcessPromptDefaults {
    system: PostProcessSystemPrompts,
    user_template: String,
}

#[derive(Deserialize)]
struct PostProcessSystemPrompts {
    en: String,
    ru: String,
}

// ── Input structs (partial updates) ──────────────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SttConfigInput {
    provider_id: Option<String>,
    model_key: Option<String>,
    language: Option<String>,
    use_custom_prompt: Option<bool>,
    system_prompt: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostProcessConfigInput {
    enabled: Option<bool>,
    provider_id: Option<String>,
    model_key: Option<String>,
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
    let language = get_effective_ui_language(&app).map_err(AppError::into_message)?;

    default_prompts_for_language(&language).map_err(AppError::into_message)
}

pub fn default_prompts_for_language(language: &EffectiveUiLanguage) -> AppResult<DefaultPrompts> {
    Ok(DefaultPrompts {
        stt_system: default_stt_system_prompt(language).to_string(),
        post_process_system: default_post_process_system_prompt(language)?,
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

    if let Some(provider_id) = input.provider_id {
        config.stt.provider_id = normalize_optional_string(provider_id);
    }

    if let Some(model_key) = input.model_key {
        config.stt.model_key = normalize_optional_string(model_key);
    }

    if let Some(language) = input.language {
        config.stt.language = language.trim().to_string();
    }

    if let Some(use_custom_prompt) = input.use_custom_prompt {
        config.stt.use_custom_prompt = use_custom_prompt;
    }

    if let Some(system_prompt) = input.system_prompt {
        config.stt.system_prompt = system_prompt;
    }

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

    if let Some(provider_id) = input.provider_id {
        config.post_process.provider_id = normalize_optional_string(provider_id);
    }

    if let Some(model_key) = input.model_key {
        config.post_process.model_key = normalize_optional_string(model_key);
    }

    if let Some(use_custom_prompts) = input.use_custom_prompts {
        config.post_process.use_custom_prompts = use_custom_prompts;
    }

    if let Some(system_prompt) = input.system_prompt {
        config.post_process.system_prompt = system_prompt;
    }

    if let Some(user_prompt_template) = input.user_prompt_template {
        config.post_process.user_prompt_template = user_prompt_template;
    }

    save_processing_config(app, &config)?;

    Ok(config)
}

pub fn load_processing_config(app: &tauri::AppHandle) -> AppResult<ProcessingConfig> {
    storage::load_json_or_default(app, PROCESSING_FILE_NAME)
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

fn default_language() -> String {
    "auto".to_string()
}

fn default_stt_system_prompt(language: &EffectiveUiLanguage) -> &'static str {
    match language {
        EffectiveUiLanguage::En => DEFAULT_STT_SYSTEM_PROMPT_EN,
        EffectiveUiLanguage::Ru => DEFAULT_STT_SYSTEM_PROMPT_RU,
    }
}

fn default_post_process_system_prompt(language: &EffectiveUiLanguage) -> AppResult<String> {
    let prompts = load_post_process_prompt_defaults()?;

    Ok(match language {
        EffectiveUiLanguage::En => prompts.system.en,
        EffectiveUiLanguage::Ru => prompts.system.ru,
    })
}

fn default_post_process_user_template() -> AppResult<String> {
    Ok(load_post_process_prompt_defaults()?.user_template)
}

fn load_post_process_prompt_defaults() -> AppResult<PostProcessPromptDefaults> {
    serde_json::from_str(POST_PROCESS_PROMPTS_JSON)
        .map_err(|error| format!("Invalid post-process prompt defaults: {error}").into())
}
