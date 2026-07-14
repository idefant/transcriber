use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    catalog::{model_by_key, ModelTask},
    error::{AppError, AppResult},
    i18n,
    providers::{find_provider_kind, ProviderKind},
    settings::{get_effective_ui_language, EffectiveUiLanguage},
    storage,
};

const PROCESSING_FILE_NAME: &str = "processing.json";
const PROMPTS_JSON: &str = include_str!("../../resources/promps.json");

// ── Хранимые / публичные структуры ────────────────────────────────────────────

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
    /// Переопределение системного промпта. `None` использует промпт по умолчанию.
    #[serde(default, alias = "prompt")]
    pub system_prompt: Option<String>,
}

impl Default for SttConfig {
    fn default() -> Self {
        Self {
            provider_id: None,
            model_key: None,
            language: default_language(),
            use_custom_prompt: false,
            system_prompt: None,
        }
    }
}

impl SttConfig {
    /// Действующий шаблон системного промпта (всё ещё содержит плейсхолдеры `{{...}}`).
    pub fn effective_system_prompt(&self) -> AppResult<String> {
        if self.use_custom_prompt {
            match &self.system_prompt {
                Some(system_prompt) => Ok(system_prompt.clone()),
                None => default_stt_system_prompt(&self.language),
            }
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
    /// Переопределение системного промпта. `None` использует промпт по умолчанию.
    #[serde(default, alias = "prompt")]
    pub system_prompt: Option<String>,
    /// Переопределение шаблона пользовательского промпта. `None` использует шаблон по умолчанию.
    #[serde(default)]
    pub user_prompt_template: Option<String>,
    /// Предпочтительный апстрим-провайдер OpenRouter (slug). `None` означает «Авто».
    #[serde(default)]
    pub openrouter_provider: Option<String>,
}

impl PostProcessConfig {
    /// Действующий шаблон системного промпта (всё ещё содержит плейсхолдеры `{{...}}`).
    pub fn effective_system_prompt(&self, language: &EffectiveUiLanguage) -> AppResult<String> {
        if self.use_custom_prompts {
            match &self.system_prompt {
                Some(system_prompt) => Ok(system_prompt.clone()),
                None => default_post_process_system_prompt(language),
            }
        } else {
            default_post_process_system_prompt(language)
        }
    }

    /// Действующий шаблон пользовательского промпта (всё ещё содержит плейсхолдеры `{{...}}`).
    pub fn effective_user_template(&self) -> AppResult<String> {
        if self.use_custom_prompts {
            match &self.user_prompt_template {
                Some(user_prompt_template) => Ok(user_prompt_template.clone()),
                None => default_post_process_user_template(),
            }
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

#[derive(Default)]
enum NullableInput<T> {
    #[default]
    Missing,
    Null,
    Value(T),
}

impl<'de, T> Deserialize<'de> for NullableInput<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        match Option::<T>::deserialize(deserializer)? {
            Some(value) => Ok(Self::Value(value)),
            None => Ok(Self::Null),
        }
    }
}

// ── Входные структуры (частичные обновления) ──────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SttConfigInput {
    #[serde(default)]
    provider_id: NullableInput<String>,
    #[serde(default)]
    model_key: NullableInput<String>,
    language: Option<String>,
    use_custom_prompt: Option<bool>,
    #[serde(default)]
    system_prompt: NullableInput<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostProcessConfigInput {
    enabled: Option<bool>,
    #[serde(default)]
    provider_id: NullableInput<String>,
    #[serde(default)]
    model_key: NullableInput<String>,
    use_custom_prompts: Option<bool>,
    #[serde(default)]
    system_prompt: NullableInput<String>,
    #[serde(default)]
    user_prompt_template: NullableInput<String>,
    #[serde(default)]
    openrouter_provider: NullableInput<String>,
}

// ── Команды Tauri ──────────────────────────────────────────────────────────────

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

// ── Внутренние функции ─────────────────────────────────────────────────────────

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

    apply_nullable_input_patch(&mut config.stt.system_prompt, input.system_prompt);

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

    apply_nullable_input_patch(&mut config.post_process.system_prompt, input.system_prompt);

    apply_nullable_input_patch(
        &mut config.post_process.user_prompt_template,
        input.user_prompt_template,
    );

    apply_nullable_input_patch(
        &mut config.post_process.openrouter_provider,
        input.openrouter_provider,
    );

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

fn apply_optional_string_patch(target: &mut Option<String>, patch: NullableInput<String>) {
    match patch {
        NullableInput::Missing => {}
        NullableInput::Null => *target = None,
        NullableInput::Value(value) => {
            *target = normalize_optional_string(value);
        }
    }
}

fn apply_nullable_input_patch(target: &mut Option<String>, patch: NullableInput<String>) {
    match patch {
        NullableInput::Missing => {}
        NullableInput::Null => *target = None,
        NullableInput::Value(value) => *target = Some(value),
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
    changed |= normalize_post_process_openrouter_provider(app, &mut config.post_process)?;

    Ok(changed)
}

/// Сбрасывает выбранный апстрим-провайдер OpenRouter, если провайдер
/// постобработки больше не OpenRouter или модель не выбрана: набор
/// доступных апстрим-провайдеров привязан к конкретной модели, поэтому
/// значение, выбранное для другой модели, может быть недействительным.
fn normalize_post_process_openrouter_provider(
    app: &tauri::AppHandle,
    post_process: &mut PostProcessConfig,
) -> AppResult<bool> {
    if post_process.openrouter_provider.is_none() {
        return Ok(false);
    }

    let is_openrouter = post_process
        .provider_id
        .as_deref()
        .map(|provider_id| find_provider_kind(app, provider_id))
        .transpose()?
        .flatten()
        .is_some_and(|kind| matches!(kind, ProviderKind::Openrouter));

    if post_process.model_key.is_some() && is_openrouter {
        return Ok(false);
    }

    post_process.openrouter_provider = None;

    Ok(true)
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
    serde_json::from_str(PROMPTS_JSON).map_err(|error| {
        i18n::text_for_language(
            EffectiveUiLanguage::En,
            "prompt-defaults-invalid",
            &[("error", error.to_string())],
        )
        .into()
    })
}
