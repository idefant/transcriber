use std::time::Duration;

use chrono::Utc;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    StatusCode,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    storage,
};

const PROVIDERS_FILE_NAME: &str = "providers.json";

#[derive(Clone, Copy, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
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
pub struct ProviderConfig {
    id: String,
    name: String,
    provider: ProviderKind,
    key_preview: String,
    has_api_key: bool,
    base_url: Option<String>,
    headers: Option<String>,
    use_advanced_settings: bool,
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
            created_at: provider.created_at,
            updated_at: provider.updated_at,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInput {
    name: Option<String>,
    provider: ProviderKind,
    api_key: Option<String>,
    base_url: Option<String>,
    headers: Option<String>,
    use_advanced_settings: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderConnectionInput {
    provider_id: Option<String>,
    provider: ProviderKind,
    api_key: Option<String>,
    base_url: Option<String>,
    headers: Option<String>,
    use_advanced_settings: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderValidationResult {
    ok: bool,
    message: String,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    name: String,
    description: String,
}

#[tauri::command]
pub fn get_providers(app: tauri::AppHandle) -> Result<Vec<ProviderConfig>, String> {
    get_providers_inner(&app).map_err(AppError::into_message)
}

#[tauri::command]
pub fn create_provider(
    app: tauri::AppHandle,
    input: ProviderInput,
) -> Result<ProviderConfig, String> {
    create_provider_inner(&app, input).map_err(AppError::into_message)
}

#[tauri::command]
pub fn update_provider(
    app: tauri::AppHandle,
    provider_id: String,
    input: ProviderInput,
) -> Result<ProviderConfig, String> {
    update_provider_inner(&app, provider_id, input).map_err(AppError::into_message)
}

#[tauri::command]
pub fn delete_provider(app: tauri::AppHandle, provider_id: String) -> Result<(), String> {
    delete_provider_inner(&app, provider_id).map_err(AppError::into_message)
}

#[tauri::command]
pub async fn validate_provider_config(
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
            message: error.into_message(),
        }),
    }
}

#[tauri::command]
pub async fn list_provider_models(
    app: tauri::AppHandle,
    input: ProviderConnectionInput,
) -> Result<Vec<ModelInfo>, String> {
    request_provider_models(&app, input)
        .await
        .map_err(AppError::into_message)
}

fn get_providers_inner(app: &tauri::AppHandle) -> AppResult<Vec<ProviderConfig>> {
    Ok(load_providers(app)?
        .into_iter()
        .map(ProviderConfig::from)
        .collect())
}

fn create_provider_inner(
    app: &tauri::AppHandle,
    input: ProviderInput,
) -> AppResult<ProviderConfig> {
    let mut providers = load_providers(app)?;
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
        favorite_models: Vec::new(),
        created_at: now.clone(),
        updated_at: now,
    };

    providers.push(provider.clone());
    save_providers(app, &providers)?;

    Ok(provider.into())
}

fn update_provider_inner(
    app: &tauri::AppHandle,
    provider_id: String,
    input: ProviderInput,
) -> AppResult<ProviderConfig> {
    let mut providers = load_providers(app)?;
    let provider = providers
        .iter_mut()
        .find(|provider| provider.id == provider_id)
        .ok_or("Provider was not found")?;

    provider.name = normalize_optional_string(input.name)
        .unwrap_or_else(|| input.provider.default_name().to_string());
    provider.provider = input.provider;

    if let Some(api_key) = normalize_optional_string(input.api_key) {
        provider.api_key = api_key;
    }

    provider.base_url = normalize_optional_string(input.base_url);
    provider.headers = normalize_optional_string(input.headers);
    provider.use_advanced_settings = input.use_advanced_settings;
    provider.updated_at = Utc::now().to_rfc3339();

    let updated_provider = provider.clone();
    save_providers(app, &providers)?;

    Ok(updated_provider.into())
}

fn delete_provider_inner(app: &tauri::AppHandle, provider_id: String) -> AppResult<()> {
    let mut providers = load_providers(app)?;
    providers.retain(|provider| provider.id != provider_id);
    save_providers(app, &providers)
}

async fn request_provider_models(
    app: &tauri::AppHandle,
    input: ProviderConnectionInput,
) -> AppResult<Vec<ModelInfo>> {
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
        .ok_or("API key is required")?;
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
        .build()?;
    let url = format!("{}/models", base_url.trim_end_matches('/'));
    let mut request = client.get(url).bearer_auth(api_key);

    if !headers.is_empty() {
        request = request.headers(headers);
    }

    let response = request.send().await?;
    let status = response.status();

    if !status.is_success() {
        return Err(format_request_error(status, response).await.into());
    }

    let value = response.json::<serde_json::Value>().await?;
    let data = value
        .get("data")
        .and_then(serde_json::Value::as_array)
        .ok_or("Provider returned an unsupported models response.")?;
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
) -> AppResult<String> {
    if matches!(input.provider, ProviderKind::Custom) {
        return normalize_optional_string(input.base_url.clone())
            .or_else(|| stored_provider.and_then(|provider| provider.base_url.clone()))
            .ok_or_else(|| "URL is required for custom provider".into());
    }

    if should_use_advanced_settings {
        return normalize_optional_string(input.base_url.clone())
            .or_else(|| stored_provider.and_then(|provider| provider.base_url.clone()))
            .or_else(|| input.provider.default_base_url().map(ToString::to_string))
            .ok_or_else(|| "Provider URL was not found".into());
    }

    input
        .provider
        .default_base_url()
        .map(ToString::to_string)
        .ok_or_else(|| "Provider URL was not found".into())
}

fn resolve_headers(
    input: &ProviderConnectionInput,
    stored_provider: Option<&StoredProvider>,
    should_use_advanced_settings: bool,
) -> AppResult<HeaderMap> {
    if !should_use_advanced_settings {
        return Ok(HeaderMap::new());
    }

    let headers = normalize_optional_string(input.headers.clone())
        .or_else(|| stored_provider.and_then(|provider| provider.headers.clone()));

    parse_headers(&headers)
}

fn parse_headers(headers: &Option<String>) -> AppResult<HeaderMap> {
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

pub struct ProviderCredentials {
    pub kind: ProviderKind,
    pub api_key: String,
    pub base_url: String,
    pub headers: reqwest::header::HeaderMap,
}

pub fn resolve_provider_credentials(
    app: &tauri::AppHandle,
    provider_id: &str,
) -> AppResult<ProviderCredentials> {
    let providers = load_providers(app)?;
    let provider = providers
        .into_iter()
        .find(|p| p.id == provider_id)
        .ok_or("Provider was not found")?;

    let use_advanced = provider.effective_use_advanced_settings();

    let base_url = if matches!(provider.provider, ProviderKind::Custom) {
        normalize_optional_string(provider.base_url.clone())
            .ok_or("URL is required for custom provider")?
    } else if use_advanced {
        normalize_optional_string(provider.base_url.clone())
            .or_else(|| {
                provider
                    .provider
                    .default_base_url()
                    .map(ToString::to_string)
            })
            .ok_or("Provider URL was not found")?
    } else {
        provider
            .provider
            .default_base_url()
            .map(ToString::to_string)
            .ok_or("Provider URL was not found")?
    };

    let headers = if use_advanced {
        parse_headers(&provider.headers)?
    } else {
        reqwest::header::HeaderMap::new()
    };

    Ok(ProviderCredentials {
        kind: provider.provider,
        api_key: provider.api_key,
        base_url,
        headers,
    })
}

fn load_providers(app: &tauri::AppHandle) -> AppResult<Vec<StoredProvider>> {
    storage::load_json_or_default(app, PROVIDERS_FILE_NAME)
}

fn save_providers(app: &tauri::AppHandle, providers: &[StoredProvider]) -> AppResult<()> {
    storage::save_json(app, PROVIDERS_FILE_NAME, providers)
}

fn normalize_required_string(value: Option<String>, field_name: &str) -> AppResult<String> {
    normalize_optional_string(value).ok_or_else(|| format!("{field_name} is required").into())
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
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

#[cfg(test)]
mod tests {
    use reqwest::header::HeaderValue;

    use super::*;

    #[test]
    fn masks_short_api_keys() {
        assert_eq!(mask_api_key("12345678"), "****");
    }

    #[test]
    fn masks_long_api_keys() {
        assert_eq!(mask_api_key("sk-1234567890"), "sk-...7890");
    }

    #[test]
    fn parses_headers() {
        let headers = parse_headers(&Some(
            "X-Title: Transcriber\n\nHTTP-Referer: https://example.com".to_string(),
        ))
        .expect("headers should parse");

        assert_eq!(
            headers.get("X-Title"),
            Some(&HeaderValue::from_static("Transcriber"))
        );
        assert_eq!(
            headers.get("HTTP-Referer"),
            Some(&HeaderValue::from_static("https://example.com"))
        );
    }

    #[test]
    fn rejects_headers_without_colon() {
        let error = parse_headers(&Some("Broken".to_string()))
            .expect_err("header without colon should fail");

        assert_eq!(
            error.into_message(),
            "Header must use `Name: value` format: Broken"
        );
    }

    #[test]
    fn resolves_default_provider_base_url() {
        let input = provider_connection_input(ProviderKind::Openai);

        assert_eq!(
            resolve_base_url(&input, None, false).expect("base url should resolve"),
            "https://api.openai.com/v1"
        );
    }

    #[test]
    fn resolves_custom_provider_base_url_from_input() {
        let mut input = provider_connection_input(ProviderKind::Custom);
        input.base_url = Some(" https://example.com/v1 ".to_string());

        assert_eq!(
            resolve_base_url(&input, None, true).expect("base url should resolve"),
            "https://example.com/v1"
        );
    }

    #[test]
    fn resolves_advanced_provider_base_url_from_stored_provider() {
        let input = provider_connection_input(ProviderKind::Groq);
        let stored_provider = stored_provider_with_base_url("https://proxy.example.com/v1");

        assert_eq!(
            resolve_base_url(&input, Some(&stored_provider), true)
                .expect("base url should resolve"),
            "https://proxy.example.com/v1"
        );
    }

    fn provider_connection_input(provider: ProviderKind) -> ProviderConnectionInput {
        ProviderConnectionInput {
            provider_id: None,
            provider,
            api_key: None,
            base_url: None,
            headers: None,
            use_advanced_settings: None,
        }
    }

    fn stored_provider_with_base_url(base_url: &str) -> StoredProvider {
        StoredProvider {
            id: "provider-id".to_string(),
            name: "Provider".to_string(),
            provider: ProviderKind::Groq,
            api_key: "api-key".to_string(),
            base_url: Some(base_url.to_string()),
            headers: None,
            use_advanced_settings: Some(true),
            favorite_models: Vec::new(),
            created_at: "2026-06-16T00:00:00Z".to_string(),
            updated_at: "2026-06-16T00:00:00Z".to_string(),
        }
    }
}
