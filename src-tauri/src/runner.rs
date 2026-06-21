use std::time::{Duration, Instant};

use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION};
use serde::{Deserialize, Serialize};

use crate::{
    catalog::{model_by_key, ModelParams},
    dictionary,
    error::{AppError, AppResult},
    processing::load_processing_config,
    providers::{resolve_provider_api_key, resolve_provider_credentials, ProviderKind},
    settings::get_effective_ui_language,
};

const AGENT_NAME: &str = "Transcriber";

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeaderSnapshot {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSnapshot {
    pub provider_id: String,
    pub provider_name: String,
    pub provider_kind: ProviderKind,
    pub base_url: String,
    pub headers: Vec<HeaderSnapshot>,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SttSettingsSnapshot {
    pub provider: ProviderSnapshot,
    pub model_key: String,
    pub model_label: String,
    pub api_model_id: String,
    pub language: String,
    pub temperature: f32,
    pub response_format: String,
    pub prompt: String,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostProcessSettingsSnapshot {
    pub provider: ProviderSnapshot,
    pub model_key: String,
    pub model_label: String,
    pub api_model_id: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub disable_thinking_body: bool,
    pub disable_thinking_prompt: bool,
    pub reasoning_effort: Option<String>,
    pub reasoning_format: Option<String>,
    pub include_reasoning: Option<bool>,
    pub reasoning: Option<ReasoningSnapshot>,
    pub system_prompt: String,
    pub user_prompt_template: String,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningSnapshot {
    pub effort: String,
    pub exclude: bool,
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RunUsage {
    pub raw: serde_json::Value,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SttRunOutput {
    pub text: String,
    pub provider: String,
    pub model: String,
    pub duration_ms: u64,
    pub usage: Option<RunUsage>,
    pub cost: Option<f64>,
    pub settings_snapshot: SttSettingsSnapshot,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostProcessRunOutput {
    pub text: String,
    pub provider: String,
    pub model: String,
    pub duration_ms: u64,
    pub usage: Option<RunUsage>,
    pub cost: Option<f64>,
    pub settings_snapshot: PostProcessSettingsSnapshot,
}

#[derive(Deserialize)]
struct SttResponse {
    text: String,
    #[serde(default)]
    usage: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatMessage,
}

#[derive(Deserialize)]
struct ChatMessage {
    content: Option<String>,
}

#[derive(Deserialize)]
struct ChatResponse {
    #[serde(default)]
    id: Option<String>,
    choices: Vec<ChatChoice>,
    #[serde(default)]
    usage: Option<serde_json::Value>,
}

#[tauri::command]
pub async fn run_stt_test(
    app: tauri::AppHandle,
    audio: Vec<u8>,
    file_name: String,
) -> Result<String, String> {
    run_stt(&app, audio, file_name)
        .await
        .map_err(AppError::into_message)
}

#[tauri::command]
pub async fn run_post_process_test(app: tauri::AppHandle, text: String) -> Result<String, String> {
    run_post_process(&app, text)
        .await
        .map_err(AppError::into_message)
}

pub async fn run_stt(
    app: &tauri::AppHandle,
    audio: Vec<u8>,
    file_name: String,
) -> AppResult<String> {
    Ok(run_stt_detailed(app, audio, file_name, None).await?.text)
}

pub async fn run_post_process(app: &tauri::AppHandle, text: String) -> AppResult<String> {
    Ok(run_post_process_detailed(app, text).await?.text)
}

pub async fn run_stt_detailed(
    app: &tauri::AppHandle,
    audio: Vec<u8>,
    file_name: String,
    audio_duration_ms: Option<u64>,
) -> AppResult<SttRunOutput> {
    let snapshot = build_stt_snapshot(app)?;

    run_stt_with_snapshot(app, &snapshot, audio, file_name, audio_duration_ms).await
}

pub async fn run_post_process_detailed(
    app: &tauri::AppHandle,
    text: String,
) -> AppResult<PostProcessRunOutput> {
    let snapshot = build_post_process_snapshot(app)?;

    run_post_process_with_snapshot(app, &snapshot, text).await
}

pub async fn run_stt_with_snapshot(
    app: &tauri::AppHandle,
    snapshot: &SttSettingsSnapshot,
    audio: Vec<u8>,
    file_name: String,
    audio_duration_ms: Option<u64>,
) -> AppResult<SttRunOutput> {
    let api_key = resolve_provider_api_key(app, &snapshot.provider.provider_id)?;
    let mime = mime_from_filename(&file_name);
    let file_part = reqwest::multipart::Part::bytes(audio)
        .file_name(file_name)
        .mime_str(mime)
        .map_err(|e| format!("Invalid MIME type: {e}"))?;

    let form = reqwest::multipart::Form::new()
        .part("file", file_part)
        .text("model", snapshot.api_model_id.clone())
        .text("response_format", snapshot.response_format.clone())
        .text("temperature", snapshot.temperature.to_string());

    let form = if snapshot.language != "auto" && !snapshot.language.trim().is_empty() {
        form.text("language", snapshot.language.clone())
    } else {
        form
    };

    let form = if !snapshot.prompt.trim().is_empty() {
        form.text("prompt", snapshot.prompt.clone())
    } else {
        form
    };

    let url = format!(
        "{}/audio/transcriptions",
        snapshot.provider.base_url.trim_end_matches('/')
    );
    let client = build_client()?;
    let started_at = Instant::now();
    let mut request = client
        .post(url)
        .header(AUTHORIZATION, format!("Bearer {api_key}"))
        .multipart(form);

    let headers = header_map_from_snapshot(&snapshot.provider.headers)?;

    if !headers.is_empty() {
        request = request.headers(headers);
    }

    let response = request.send().await?;
    let status = response.status();

    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("STT request failed with status {status}: {body}").into());
    }

    let stt_response = response.json::<SttResponse>().await?;
    let duration_ms = elapsed_ms(started_at);
    let cost = stt_cost(snapshot, audio_duration_ms, stt_response.usage.as_ref());

    Ok(SttRunOutput {
        text: stt_response.text.trim().to_string(),
        provider: snapshot.provider.provider_name.clone(),
        model: snapshot.model_label.clone(),
        duration_ms,
        usage: stt_response.usage.map(|raw| RunUsage { raw }),
        cost,
        settings_snapshot: snapshot.clone(),
    })
}

pub async fn run_post_process_with_snapshot(
    app: &tauri::AppHandle,
    snapshot: &PostProcessSettingsSnapshot,
    text: String,
) -> AppResult<PostProcessRunOutput> {
    let api_key = resolve_provider_api_key(app, &snapshot.provider.provider_id)?;
    let mut system_prompt = apply_template(
        &snapshot.system_prompt,
        &[("CLEANUP_TOOL_AGENT_NAME", AGENT_NAME)],
    );

    if snapshot.disable_thinking_prompt && !system_prompt.contains("/no_think") {
        system_prompt.push_str("\n\n/no_think");
    }

    let user_content = apply_template(
        &snapshot.user_prompt_template,
        &[("TRANSCRIBED_TEXT", text.as_str())],
    );

    let mut body = serde_json::json!({
        "model": snapshot.api_model_id,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_content }
        ],
        "temperature": snapshot.temperature,
        "max_completion_tokens": snapshot.max_tokens,
    });

    if snapshot.disable_thinking_body {
        body["thinking"] = serde_json::json!({ "type": "disabled" });
    }

    if let Some(reasoning) = &snapshot.reasoning {
        body["reasoning"] = serde_json::json!({
            "effort": reasoning.effort,
            "exclude": reasoning.exclude,
        });
    }

    if let Some(reasoning_effort) = &snapshot.reasoning_effort {
        body["reasoning_effort"] = serde_json::json!(reasoning_effort);
    }

    if let Some(reasoning_format) = &snapshot.reasoning_format {
        body["reasoning_format"] = serde_json::json!(reasoning_format);
    }

    if let Some(include_reasoning) = snapshot.include_reasoning {
        body["include_reasoning"] = serde_json::json!(include_reasoning);
    }

    let url = format!(
        "{}/chat/completions",
        snapshot.provider.base_url.trim_end_matches('/')
    );
    let client = build_client()?;
    let started_at = Instant::now();
    let mut request = client
        .post(url)
        .header(AUTHORIZATION, format!("Bearer {api_key}"))
        .json(&body);

    let headers = header_map_from_snapshot(&snapshot.provider.headers)?;

    if !headers.is_empty() {
        request = request.headers(headers);
    }

    let response = request.send().await?;
    let status = response.status();

    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Post-process request failed with status {status}: {body}").into());
    }

    let chat_response = response.json::<ChatResponse>().await?;
    let duration_ms = elapsed_ms(started_at);
    let content = chat_response
        .choices
        .into_iter()
        .next()
        .and_then(|c| c.message.content)
        .ok_or("Provider returned an empty response")?;
    let cost = post_process_cost(
        app,
        snapshot,
        chat_response.id.as_deref(),
        chat_response.usage.as_ref(),
    )
    .await;

    Ok(PostProcessRunOutput {
        text: content,
        provider: snapshot.provider.provider_name.clone(),
        model: snapshot.model_label.clone(),
        duration_ms,
        usage: chat_response.usage.map(|raw| RunUsage { raw }),
        cost,
        settings_snapshot: snapshot.clone(),
    })
}

pub fn build_stt_snapshot(app: &tauri::AppHandle) -> AppResult<SttSettingsSnapshot> {
    let config = load_processing_config(app)?;
    let stt = &config.stt;
    let provider_id = stt.provider_id.clone().ok_or("Provider is not selected")?;
    let model_key = stt.model_key.clone().ok_or("Model is not selected")?;
    let credentials = resolve_provider_credentials(app, &provider_id)?;
    let model = model_by_key(&model_key).ok_or("Model not found in catalog")?;
    let provider_entry = model
        .entry_for(credentials.kind)
        .ok_or("Model is not available for this provider")?;
    let ModelParams::Stt(params) = &model.params else {
        return Err("Expected STT model params".into());
    };
    let dictionary = dictionary::load_dictionary_words(app)?.join(", ");
    let prompt = apply_template(
        stt.effective_system_prompt(),
        &[
            ("STT_DICTIONARY", dictionary.as_str()),
            ("CLEANUP_TOOL_AGENT_NAME", AGENT_NAME),
        ],
    );

    Ok(SttSettingsSnapshot {
        provider: provider_snapshot(&credentials),
        model_key,
        model_label: model.label.to_string(),
        api_model_id: provider_entry.api_id.to_string(),
        language: stt.language.trim().to_string(),
        temperature: params.temperature,
        response_format: params.response_format.to_string(),
        prompt,
    })
}

pub fn build_post_process_snapshot(
    app: &tauri::AppHandle,
) -> AppResult<PostProcessSettingsSnapshot> {
    let config = load_processing_config(app)?;
    let ui_language = get_effective_ui_language(app)?;
    let post_process = &config.post_process;
    let provider_id = post_process
        .provider_id
        .clone()
        .ok_or("Provider is not selected")?;
    let model_key = post_process
        .model_key
        .clone()
        .ok_or("Model is not selected")?;
    let credentials = resolve_provider_credentials(app, &provider_id)?;
    let model = model_by_key(&model_key).ok_or("Model not found in catalog")?;
    let provider_entry = model
        .entry_for(credentials.kind)
        .ok_or("Model is not available for this provider")?;
    let ModelParams::PostProcess(params) = &model.params else {
        return Err("Expected PostProcess model params".into());
    };

    Ok(PostProcessSettingsSnapshot {
        provider: provider_snapshot(&credentials),
        model_key,
        model_label: model.label.to_string(),
        api_model_id: provider_entry.api_id.to_string(),
        temperature: params.temperature,
        max_tokens: params.max_tokens,
        disable_thinking_body: params.disable_thinking_body,
        disable_thinking_prompt: params.disable_thinking_prompt,
        reasoning_effort: provider_entry.reasoning_effort.map(ToString::to_string),
        reasoning_format: provider_entry.reasoning_format.map(ToString::to_string),
        include_reasoning: provider_entry.include_reasoning,
        reasoning: provider_entry
            .reasoning
            .as_ref()
            .map(|reasoning| ReasoningSnapshot {
                effort: reasoning.effort.to_string(),
                exclude: reasoning.exclude,
            }),
        system_prompt: post_process.effective_system_prompt(&ui_language)?,
        user_prompt_template: post_process.effective_user_template()?,
    })
}

fn provider_snapshot(credentials: &crate::providers::ProviderCredentials) -> ProviderSnapshot {
    ProviderSnapshot {
        provider_id: credentials.id.clone(),
        provider_name: credentials.name.clone(),
        provider_kind: credentials.kind,
        base_url: credentials.base_url.clone(),
        headers: credentials
            .headers
            .iter()
            .filter_map(|(name, value)| {
                Some(HeaderSnapshot {
                    name: name.as_str().to_string(),
                    value: value.to_str().ok()?.to_string(),
                })
            })
            .collect(),
    }
}

fn header_map_from_snapshot(headers: &[HeaderSnapshot]) -> AppResult<HeaderMap> {
    let mut header_map = HeaderMap::new();

    for header in headers {
        let name = HeaderName::from_bytes(header.name.as_bytes())
            .map_err(|error| format!("Invalid header name `{}`: {}", header.name, error))?;
        let value = HeaderValue::from_str(&header.value)
            .map_err(|error| format!("Invalid header value for `{}`: {}", header.name, error))?;

        header_map.insert(name, value);
    }

    Ok(header_map)
}

async fn post_process_cost(
    app: &tauri::AppHandle,
    snapshot: &PostProcessSettingsSnapshot,
    generation_id: Option<&str>,
    usage: Option<&serde_json::Value>,
) -> Option<f64> {
    if let Some(cost) = usage.and_then(find_cost) {
        return Some(cost);
    }

    if matches!(snapshot.provider.provider_kind, ProviderKind::Openrouter) {
        if let Some(cost) = openrouter_generation_cost(app, snapshot, generation_id).await {
            return Some(cost);
        }
    }

    None
}

async fn openrouter_generation_cost(
    app: &tauri::AppHandle,
    snapshot: &PostProcessSettingsSnapshot,
    generation_id: Option<&str>,
) -> Option<f64> {
    let generation_id = generation_id?;
    let api_key = resolve_provider_api_key(app, &snapshot.provider.provider_id).ok()?;
    let url = format!(
        "{}/generation?id={}",
        snapshot.provider.base_url.trim_end_matches('/'),
        generation_id
    );
    let client = build_client().ok()?;
    let response = client
        .get(url)
        .header(AUTHORIZATION, format!("Bearer {api_key}"))
        .send()
        .await
        .ok()?;
    let value = response.json::<serde_json::Value>().await.ok()?;

    find_cost(&value)
}

fn stt_cost(
    snapshot: &SttSettingsSnapshot,
    audio_duration_ms: Option<u64>,
    usage: Option<&serde_json::Value>,
) -> Option<f64> {
    if let Some(cost) = usage.and_then(find_cost) {
        return Some(cost);
    }

    let _ = (snapshot, audio_duration_ms);

    None
}

fn find_cost(value: &serde_json::Value) -> Option<f64> {
    match value {
        serde_json::Value::Object(map) => {
            for key in ["cost", "total_cost", "totalCost"] {
                if let Some(cost) = map.get(key).and_then(serde_json::Value::as_f64) {
                    return Some(cost);
                }
            }

            map.values().find_map(find_cost)
        }
        serde_json::Value::Array(values) => values.iter().find_map(find_cost),
        _ => None,
    }
}

fn elapsed_ms(started_at: Instant) -> u64 {
    started_at
        .elapsed()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn apply_template(template: &str, vars: &[(&str, &str)]) -> String {
    let mut result = template.to_string();

    for (key, value) in vars {
        let placeholder = ["{{", key, "}}"].concat();
        result = result.replace(&placeholder, value);
    }

    result
}

fn build_client() -> AppResult<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .timeout(Duration::from_secs(60))
        .build()?)
}

fn mime_from_filename(name: &str) -> &'static str {
    let ext = name.rsplit('.').next().unwrap_or("").to_lowercase();

    match ext.as_str() {
        "mp3" | "mpeg" | "mpga" => "audio/mpeg",
        "mp4" | "m4a" => "audio/mp4",
        "wav" => "audio/wav",
        "ogg" | "oga" => "audio/ogg",
        "webm" => "audio/webm",
        "flac" => "audio/flac",
        _ => "application/octet-stream",
    }
}
