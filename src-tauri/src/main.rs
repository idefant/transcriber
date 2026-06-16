#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{fs, path::PathBuf, time::Duration};

use chrono::Utc;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    StatusCode,
};
use serde::{Deserialize, Serialize};
use tauri::Manager;
use uuid::Uuid;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
enum ProviderKind {
    Custom,
    Groq,
    Openai,
    Openrouter,
}

impl ProviderKind {
    fn default_base_url(&self) -> Option<&'static str> {
        match self {
            Self::Custom => None,
            Self::Groq => Some("https://api.groq.com/openai/v1"),
            Self::Openai => Some("https://api.openai.com/v1"),
            Self::Openrouter => Some("https://openrouter.ai/api/v1"),
        }
    }

    fn default_name(&self) -> &'static str {
        match self {
            Self::Custom => "Custom",
            Self::Groq => "Groq",
            Self::Openai => "OpenAI",
            Self::Openrouter => "OpenRouter",
        }
    }
}

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
struct StoredProvider {
    id: String,
    name: String,
    provider: ProviderKind,
    api_key: String,
    base_url: Option<String>,
    headers: Option<String>,
    #[serde(default)]
    use_advanced_settings: Option<bool>,
    favorite_models: Vec<String>,
    created_at: String,
    updated_at: String,
}

impl StoredProvider {
    fn effective_use_advanced_settings(&self) -> bool {
        self.use_advanced_settings
            .unwrap_or_else(|| self.base_url.is_some() || self.headers.is_some())
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProviderConfig {
    id: String,
    name: String,
    provider: ProviderKind,
    key_preview: String,
    has_api_key: bool,
    base_url: Option<String>,
    headers: Option<String>,
    use_advanced_settings: bool,
    favorite_models: Vec<String>,
    created_at: String,
    updated_at: String,
}

impl From<StoredProvider> for ProviderConfig {
    fn from(provider: StoredProvider) -> Self {
        let use_advanced_settings = provider.effective_use_advanced_settings();

        Self {
            id: provider.id,
            name: provider.name,
            key_preview: mask_api_key(&provider.api_key),
            has_api_key: !provider.api_key.trim().is_empty(),
            provider: provider.provider,
            base_url: provider.base_url,
            headers: provider.headers,
            use_advanced_settings,
            favorite_models: provider.favorite_models,
            created_at: provider.created_at,
            updated_at: provider.updated_at,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderInput {
    name: Option<String>,
    provider: ProviderKind,
    api_key: Option<String>,
    base_url: Option<String>,
    headers: Option<String>,
    use_advanced_settings: Option<bool>,
    favorite_models: Option<Vec<String>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProviderConnectionInput {
    provider_id: Option<String>,
    provider: ProviderKind,
    api_key: Option<String>,
    base_url: Option<String>,
    headers: Option<String>,
    use_advanced_settings: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProviderValidationResult {
    ok: bool,
    message: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ModelInfo {
    name: String,
    description: String,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppSettings {
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
struct AppSettingsInput {
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
fn get_app_settings(app: tauri::AppHandle) -> Result<AppSettings, String> {
    load_app_settings(&app)
}

#[tauri::command]
fn update_app_settings(
    app: tauri::AppHandle,
    input: AppSettingsInput,
) -> Result<AppSettings, String> {
    let mut settings = load_app_settings(&app)?;

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

    save_app_settings(&app, &settings)?;

    Ok(settings)
}

#[tauri::command]
fn get_dictionary_words(app: tauri::AppHandle) -> Result<Vec<String>, String> {
    load_dictionary_words(&app)
}

#[tauri::command]
fn add_dictionary_word(app: tauri::AppHandle, word: String) -> Result<Vec<String>, String> {
    let normalized_word = word.trim();

    if normalized_word.is_empty() {
        return load_dictionary_words(&app);
    }

    let mut words = load_dictionary_words(&app)?;
    let normalized_key = dictionary_word_key(normalized_word);

    if !words
        .iter()
        .any(|word| dictionary_word_key(word) == normalized_key)
    {
        words.push(normalized_word.to_string());
    }

    words = normalize_dictionary_words(words);
    save_dictionary_words(&app, &words)?;

    Ok(words)
}

#[tauri::command]
fn delete_dictionary_word(app: tauri::AppHandle, word: String) -> Result<Vec<String>, String> {
    let normalized_key = dictionary_word_key(&word);
    let mut words = load_dictionary_words(&app)?;

    words.retain(|stored_word| dictionary_word_key(stored_word) != normalized_key);
    words = normalize_dictionary_words(words);
    save_dictionary_words(&app, &words)?;

    Ok(words)
}

#[tauri::command]
fn get_providers(app: tauri::AppHandle) -> Result<Vec<ProviderConfig>, String> {
    Ok(load_providers(&app)?
        .into_iter()
        .map(ProviderConfig::from)
        .collect())
}

#[tauri::command]
fn create_provider(app: tauri::AppHandle, input: ProviderInput) -> Result<ProviderConfig, String> {
    let mut providers = load_providers(&app)?;
    let now = Utc::now().to_rfc3339();
    let api_key = normalize_required_string(input.api_key, "API key")?;
    let provider = StoredProvider {
        id: Uuid::new_v4().to_string(),
        name: normalize_optional_string(input.name)
            .unwrap_or_else(|| input.provider.default_name().to_string()),
        provider: input.provider,
        api_key,
        base_url: normalize_optional_string(input.base_url),
        headers: normalize_optional_string(input.headers),
        use_advanced_settings: input.use_advanced_settings,
        favorite_models: normalize_favorite_models(input.favorite_models.unwrap_or_default()),
        created_at: now.clone(),
        updated_at: now,
    };

    providers.push(provider.clone());
    save_providers(&app, &providers)?;

    Ok(provider.into())
}

#[tauri::command]
fn update_provider(
    app: tauri::AppHandle,
    provider_id: String,
    input: ProviderInput,
) -> Result<ProviderConfig, String> {
    let mut providers = load_providers(&app)?;
    let provider = providers
        .iter_mut()
        .find(|provider| provider.id == provider_id)
        .ok_or_else(|| "Provider was not found".to_string())?;

    provider.name = normalize_optional_string(input.name)
        .unwrap_or_else(|| input.provider.default_name().to_string());
    provider.provider = input.provider;

    if let Some(api_key) = normalize_optional_string(input.api_key) {
        provider.api_key = api_key;
    }

    provider.base_url = normalize_optional_string(input.base_url);
    provider.headers = normalize_optional_string(input.headers);
    provider.use_advanced_settings = input.use_advanced_settings;
    provider.favorite_models = normalize_favorite_models(input.favorite_models.unwrap_or_default());
    provider.updated_at = Utc::now().to_rfc3339();

    let updated_provider = provider.clone();
    save_providers(&app, &providers)?;

    Ok(updated_provider.into())
}

#[tauri::command]
fn delete_provider(app: tauri::AppHandle, provider_id: String) -> Result<(), String> {
    let mut providers = load_providers(&app)?;
    providers.retain(|provider| provider.id != provider_id);
    save_providers(&app, &providers)
}

#[tauri::command]
async fn validate_provider_config(
    app: tauri::AppHandle,
    input: ProviderConnectionInput,
) -> Result<ProviderValidationResult, String> {
    match request_provider_models(&app, input).await {
        Ok(models) => Ok(ProviderValidationResult {
            ok: true,
            message: format!("Configuration is valid. Models found: {}.", models.len()),
        }),
        Err(error) => Ok(ProviderValidationResult {
            ok: false,
            message: error,
        }),
    }
}

#[tauri::command]
async fn list_provider_models(
    app: tauri::AppHandle,
    input: ProviderConnectionInput,
) -> Result<Vec<ModelInfo>, String> {
    request_provider_models(&app, input).await
}

#[tauri::command]
fn toggle_favorite_model(
    app: tauri::AppHandle,
    provider_id: String,
    model_name: String,
) -> Result<ProviderConfig, String> {
    let mut providers = load_providers(&app)?;
    let provider = providers
        .iter_mut()
        .find(|provider| provider.id == provider_id)
        .ok_or_else(|| "Provider was not found".to_string())?;

    if provider
        .favorite_models
        .iter()
        .any(|model| model == &model_name)
    {
        provider
            .favorite_models
            .retain(|model| model != &model_name);
    } else {
        provider.favorite_models.push(model_name);
    }

    provider.favorite_models = normalize_favorite_models(provider.favorite_models.clone());
    provider.updated_at = Utc::now().to_rfc3339();

    let updated_provider = provider.clone();
    save_providers(&app, &providers)?;

    Ok(updated_provider.into())
}

async fn request_provider_models(
    app: &tauri::AppHandle,
    input: ProviderConnectionInput,
) -> Result<Vec<ModelInfo>, String> {
    let stored_provider = match &input.provider_id {
        Some(provider_id) => load_providers(app)?
            .into_iter()
            .find(|provider| provider.id == *provider_id),
        None => None,
    };
    let api_key = normalize_optional_string(input.api_key.clone())
        .or_else(|| {
            stored_provider
                .as_ref()
                .map(|provider| provider.api_key.clone())
        })
        .ok_or_else(|| "API key is required".to_string())?;
    let should_use_advanced_settings = matches!(input.provider, ProviderKind::Custom)
        || input
            .use_advanced_settings
            .or_else(|| {
                stored_provider
                    .as_ref()
                    .map(StoredProvider::effective_use_advanced_settings)
            })
            .unwrap_or(false);
    let base_url = resolve_base_url(
        &input,
        stored_provider.as_ref(),
        should_use_advanced_settings,
    )?;
    let headers = resolve_headers(
        &input,
        stored_provider.as_ref(),
        should_use_advanced_settings,
    )?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|error| error.to_string())?;
    let url = format!("{}/models", base_url.trim_end_matches('/'));
    let mut request = client.get(url).bearer_auth(api_key);

    if !headers.is_empty() {
        request = request.headers(headers);
    }

    let response = request.send().await.map_err(|error| error.to_string())?;
    let status = response.status();

    if !status.is_success() {
        return Err(format_request_error(status, response).await);
    }

    let value = response
        .json::<serde_json::Value>()
        .await
        .map_err(|error| error.to_string())?;
    let data = value
        .get("data")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "Provider returned an unsupported models response.".to_string())?;
    let models = data
        .iter()
        .filter_map(|item| {
            let name = item
                .get("id")
                .or_else(|| item.get("name"))
                .and_then(serde_json::Value::as_str)?;
            let description = item
                .get("description")
                .or_else(|| item.get("owned_by"))
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");

            Some(ModelInfo {
                name: name.to_string(),
                description: description.to_string(),
            })
        })
        .collect::<Vec<_>>();

    Ok(models)
}

async fn format_request_error(status: StatusCode, response: reqwest::Response) -> String {
    let body = response.text().await.unwrap_or_default();

    if body.trim().is_empty() {
        return format!("Provider request failed with status {status}.");
    }

    format!("Provider request failed with status {status}: {body}")
}

fn resolve_base_url(
    input: &ProviderConnectionInput,
    stored_provider: Option<&StoredProvider>,
    should_use_advanced_settings: bool,
) -> Result<String, String> {
    if matches!(input.provider, ProviderKind::Custom) {
        return normalize_optional_string(input.base_url.clone())
            .or_else(|| stored_provider.and_then(|provider| provider.base_url.clone()))
            .ok_or_else(|| "URL is required for custom provider".to_string());
    }

    if should_use_advanced_settings {
        return normalize_optional_string(input.base_url.clone())
            .or_else(|| stored_provider.and_then(|provider| provider.base_url.clone()))
            .or_else(|| input.provider.default_base_url().map(ToString::to_string))
            .ok_or_else(|| "Provider URL was not found".to_string());
    }

    input
        .provider
        .default_base_url()
        .map(ToString::to_string)
        .ok_or_else(|| "Provider URL was not found".to_string())
}

fn resolve_headers(
    input: &ProviderConnectionInput,
    stored_provider: Option<&StoredProvider>,
    should_use_advanced_settings: bool,
) -> Result<HeaderMap, String> {
    if !should_use_advanced_settings {
        return Ok(HeaderMap::new());
    }

    let headers = normalize_optional_string(input.headers.clone())
        .or_else(|| stored_provider.and_then(|provider| provider.headers.clone()));

    parse_headers(&headers)
}

fn parse_headers(headers: &Option<String>) -> Result<HeaderMap, String> {
    let mut header_map = HeaderMap::new();

    for line in headers.as_deref().unwrap_or("").lines() {
        let line = line.trim();

        if line.is_empty() {
            continue;
        }

        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| format!("Header must use `Name: value` format: {line}"))?;
        let header_name = HeaderName::from_bytes(name.trim().as_bytes())
            .map_err(|error| format!("Invalid header name `{}`: {}", name.trim(), error))?;
        let header_value = HeaderValue::from_str(value.trim())
            .map_err(|error| format!("Invalid header value for `{}`: {}", name.trim(), error))?;

        header_map.insert(header_name, header_value);
    }

    Ok(header_map)
}

fn load_app_settings(app: &tauri::AppHandle) -> Result<AppSettings, String> {
    let path = settings_path(app)?;

    if !path.exists() {
        return Ok(AppSettings::default());
    }

    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;

    if content.trim().is_empty() {
        return Ok(AppSettings::default());
    }

    serde_json::from_str(&content).map_err(|error| error.to_string())
}

fn save_app_settings(app: &tauri::AppHandle, settings: &AppSettings) -> Result<(), String> {
    let path = settings_path(app)?;
    let content = serde_json::to_string_pretty(settings).map_err(|error| error.to_string())?;

    fs::write(path, content).map_err(|error| error.to_string())
}

fn load_dictionary_words(app: &tauri::AppHandle) -> Result<Vec<String>, String> {
    let path = dictionary_path(app)?;

    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;

    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    let words = serde_json::from_str::<Vec<String>>(&content).map_err(|error| error.to_string())?;

    Ok(normalize_dictionary_words(words))
}

fn save_dictionary_words(app: &tauri::AppHandle, words: &[String]) -> Result<(), String> {
    let path = dictionary_path(app)?;
    let content = serde_json::to_string_pretty(words).map_err(|error| error.to_string())?;

    fs::write(path, content).map_err(|error| error.to_string())
}

fn normalize_dictionary_words(words: Vec<String>) -> Vec<String> {
    let mut normalized_words = Vec::<String>::new();

    for word in words {
        let normalized_word = word.trim();

        if normalized_word.is_empty() {
            continue;
        }

        let normalized_key = dictionary_word_key(normalized_word);

        if normalized_words
            .iter()
            .any(|stored_word| dictionary_word_key(stored_word) == normalized_key)
        {
            continue;
        }

        normalized_words.push(normalized_word.to_string());
    }

    normalized_words.sort_by_key(|word| dictionary_word_key(word));
    normalized_words
}

fn dictionary_word_key(word: &str) -> String {
    word.trim().to_lowercase()
}

fn load_providers(app: &tauri::AppHandle) -> Result<Vec<StoredProvider>, String> {
    let path = providers_path(app)?;

    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(path).map_err(|error| error.to_string())?;

    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    serde_json::from_str(&content).map_err(|error| error.to_string())
}

fn save_providers(app: &tauri::AppHandle, providers: &[StoredProvider]) -> Result<(), String> {
    let path = providers_path(app)?;
    let content = serde_json::to_string_pretty(providers).map_err(|error| error.to_string())?;

    fs::write(path, content).map_err(|error| error.to_string())
}

fn providers_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app_data_file_path(app, "providers.json")
}

fn settings_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app_data_file_path(app, "settings.json")
}

fn dictionary_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app_data_file_path(app, "dictionary.json")
}

fn app_data_file_path(app: &tauri::AppHandle, file_name: &str) -> Result<PathBuf, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?;

    fs::create_dir_all(&app_data_dir).map_err(|error| error.to_string())?;

    Ok(app_data_dir.join(file_name))
}

fn normalize_required_string(value: Option<String>, field_name: &str) -> Result<String, String> {
    normalize_optional_string(value).ok_or_else(|| format!("{field_name} is required"))
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn normalize_favorite_models(mut favorite_models: Vec<String>) -> Vec<String> {
    favorite_models = favorite_models
        .into_iter()
        .map(|model| model.trim().to_string())
        .filter(|model| !model.is_empty())
        .collect();
    favorite_models.sort();
    favorite_models.dedup();
    favorite_models
}

fn mask_api_key(api_key: &str) -> String {
    let chars = api_key.chars().collect::<Vec<_>>();

    if chars.len() <= 8 {
        return "****".to_string();
    }

    let prefix = chars.iter().take(3).collect::<String>();
    let suffix = chars
        .iter()
        .skip(chars.len().saturating_sub(4))
        .collect::<String>();

    format!("{prefix}...{suffix}")
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_app_settings,
            update_app_settings,
            get_dictionary_words,
            add_dictionary_word,
            delete_dictionary_word,
            get_providers,
            create_provider,
            update_provider,
            delete_provider,
            validate_provider_config,
            list_provider_models,
            toggle_favorite_model,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Transcriber");
}
