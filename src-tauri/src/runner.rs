use std::time::{Duration, Instant};

use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION};
use serde::{Deserialize, Serialize};

use crate::{
    catalog::{model_by_key, ModelParams},
    debug_log::{self, ModelRunLogContext, ModelRunStage},
    dictionary,
    error::{AppError, AppResult},
    i18n,
    processing::load_processing_config,
    providers::{resolve_provider_api_key, resolve_provider_credentials, ProviderKind},
    settings::{get_effective_ui_language, EffectiveUiLanguage},
    stt_prompt,
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
    pub prompt_token_limit: Option<usize>,
}

/// Результат проверки итогового prompt для модели с документированным лимитом.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SttPromptAnalysis {
    pub limit: usize,
    pub token_count: usize,
    pub usage_percent: f64,
    pub fitting_token_count: usize,
    pub excluded_token_count: usize,
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
    pub openrouter_provider: Option<String>,
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
    pub resolved_provider: Option<String>,
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
    /// Имя апстрим-провайдера, обработавшего запрос. Заполняется только
    /// OpenRouter; у OpenAI и Groq в ответе такого поля нет.
    #[serde(default)]
    provider: Option<String>,
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
    Ok(run_stt_detailed(
        app,
        audio,
        file_name,
        None,
        Some(ModelRunLogContext::settings_test()),
    )
    .await?
    .text)
}

pub async fn run_post_process(app: &tauri::AppHandle, text: String) -> AppResult<String> {
    Ok(
        run_post_process_detailed(app, text, Some(ModelRunLogContext::settings_test()))
            .await?
            .text,
    )
}

pub async fn run_stt_detailed(
    app: &tauri::AppHandle,
    audio: Vec<u8>,
    file_name: String,
    audio_duration_ms: Option<u64>,
    log_context: Option<ModelRunLogContext>,
) -> AppResult<SttRunOutput> {
    let snapshot = build_stt_snapshot(app)?;

    run_stt_with_snapshot(
        app,
        &snapshot,
        audio,
        file_name,
        audio_duration_ms,
        log_context,
    )
    .await
}

pub async fn run_post_process_detailed(
    app: &tauri::AppHandle,
    text: String,
    log_context: Option<ModelRunLogContext>,
) -> AppResult<PostProcessRunOutput> {
    let snapshot = build_post_process_snapshot(app)?;

    run_post_process_with_snapshot(app, &snapshot, text, log_context).await
}

pub async fn run_stt_with_snapshot(
    app: &tauri::AppHandle,
    snapshot: &SttSettingsSnapshot,
    audio: Vec<u8>,
    file_name: String,
    audio_duration_ms: Option<u64>,
    log_context: Option<ModelRunLogContext>,
) -> AppResult<SttRunOutput> {
    ensure_stt_prompt_within_limit(app, snapshot)?;
    let api_key = resolve_provider_api_key(app, &snapshot.provider.provider_id)?;
    let mime = mime_from_filename(&file_name);
    let audio_size_bytes = audio.len();
    let file_part = reqwest::multipart::Part::bytes(audio)
        .file_name(file_name.clone())
        .mime_str(mime)
        .map_err(|error| {
            i18n::text_with(app, "mime-type-invalid", &[("error", error.to_string())])
        })?;

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
    debug_log::log_model_event(
        app,
        "speechToText.request",
        ModelRunStage::SpeechToText,
        log_context.as_ref(),
        serde_json::json!({
            "provider": provider_payload(&snapshot.provider),
            "request": {
                "method": "POST",
                "url": url,
                "endpoint": "/audio/transcriptions",
                "headers": debug_log::sanitized_headers(&snapshot.provider.headers),
                "multipart": {
                    "file": {
                        "fileName": file_name,
                        "mime": mime,
                        "sizeBytes": audio_size_bytes,
                    },
                    "model": snapshot.api_model_id,
                    "responseFormat": snapshot.response_format,
                    "temperature": snapshot.temperature,
                    "language": optional_stt_language(snapshot),
                    "prompt": optional_prompt(&snapshot.prompt),
                },
            },
        }),
    );
    let client = build_client()?;
    let started_at = Instant::now();
    let mut request = client
        .post(url)
        .header(AUTHORIZATION, format!("Bearer {api_key}"))
        .multipart(form);

    let headers = header_map_from_snapshot(
        get_effective_ui_language(app).unwrap_or_default(),
        &snapshot.provider.headers,
    )?;

    if !headers.is_empty() {
        request = request.headers(headers);
    }

    let response = match request.send().await {
        Ok(response) => response,
        Err(error) => {
            debug_log::log_model_event(
                app,
                "speechToText.error",
                ModelRunStage::SpeechToText,
                log_context.as_ref(),
                serde_json::json!({
                    "error": error.to_string(),
                }),
            );

            return Err(error.into());
        }
    };
    let status = response.status();

    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        debug_log::log_model_event(
            app,
            "speechToText.error",
            ModelRunStage::SpeechToText,
            log_context.as_ref(),
            serde_json::json!({
                "response": {
                    "status": status.as_u16(),
                    "body": body,
                },
            }),
        );
        return Err(AppError::api(
            i18n::text_with(app, "stt-request-failed", &[("status", status.to_string())]),
            &body,
        ));
    }

    let stt_response = match response.json::<SttResponse>().await {
        Ok(stt_response) => stt_response,
        Err(error) => {
            debug_log::log_model_event(
                app,
                "speechToText.error",
                ModelRunStage::SpeechToText,
                log_context.as_ref(),
                serde_json::json!({
                    "response": {
                        "status": status.as_u16(),
                    },
                    "error": error.to_string(),
                }),
            );

            return Err(error.into());
        }
    };
    let duration_ms = elapsed_ms(started_at);
    let cost = stt_cost(snapshot, audio_duration_ms, stt_response.usage.as_ref());
    let text = stt_response.text.trim().to_string();

    debug_log::log_model_event(
        app,
        "speechToText.response",
        ModelRunStage::SpeechToText,
        log_context.as_ref(),
        serde_json::json!({
            "response": {
                "status": status.as_u16(),
                "durationMs": duration_ms,
                "text": text.clone(),
                "usage": stt_response.usage,
                "cost": cost,
            },
        }),
    );

    Ok(SttRunOutput {
        text,
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
    log_context: Option<ModelRunLogContext>,
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
        let mut r = serde_json::json!({ "effort": reasoning.effort });
        if reasoning.exclude {
            r["exclude"] = serde_json::json!(true);
        }
        body["reasoning"] = r;
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

    if matches!(snapshot.provider.provider_kind, ProviderKind::Openrouter) {
        if let Some(upstream_provider) = snapshot
            .openrouter_provider
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            body["provider"] = serde_json::json!({
                "order": [upstream_provider],
                "allow_fallbacks": true,
            });
        }
    }

    let url = format!(
        "{}/chat/completions",
        snapshot.provider.base_url.trim_end_matches('/')
    );
    debug_log::log_model_event(
        app,
        "postProcessing.request",
        ModelRunStage::PostProcessing,
        log_context.as_ref(),
        serde_json::json!({
            "provider": provider_payload(&snapshot.provider),
            "request": {
                "method": "POST",
                "url": url,
                "endpoint": "/chat/completions",
                "headers": debug_log::sanitized_headers(&snapshot.provider.headers),
                "body": body,
            },
        }),
    );
    let client = build_client()?;
    let started_at = Instant::now();
    let mut request = client
        .post(url)
        .header(AUTHORIZATION, format!("Bearer {api_key}"))
        .json(&body);

    let headers = header_map_from_snapshot(
        get_effective_ui_language(app).unwrap_or_default(),
        &snapshot.provider.headers,
    )?;

    if !headers.is_empty() {
        request = request.headers(headers);
    }

    let response = match request.send().await {
        Ok(response) => response,
        Err(error) => {
            debug_log::log_model_event(
                app,
                "postProcessing.error",
                ModelRunStage::PostProcessing,
                log_context.as_ref(),
                serde_json::json!({
                    "error": error.to_string(),
                }),
            );

            return Err(error.into());
        }
    };
    let status = response.status();

    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        debug_log::log_model_event(
            app,
            "postProcessing.error",
            ModelRunStage::PostProcessing,
            log_context.as_ref(),
            serde_json::json!({
                "response": {
                    "status": status.as_u16(),
                    "body": body,
                },
            }),
        );
        return Err(AppError::api(
            i18n::text_with(
                app,
                "post-process-request-failed",
                &[("status", status.to_string())],
            ),
            &body,
        ));
    }

    let chat_response = match response.json::<ChatResponse>().await {
        Ok(chat_response) => chat_response,
        Err(error) => {
            debug_log::log_model_event(
                app,
                "postProcessing.error",
                ModelRunStage::PostProcessing,
                log_context.as_ref(),
                serde_json::json!({
                    "response": {
                        "status": status.as_u16(),
                    },
                    "error": error.to_string(),
                }),
            );

            return Err(error.into());
        }
    };
    let duration_ms = elapsed_ms(started_at);
    let resolved_provider = chat_response.provider.clone();
    let content = chat_response
        .choices
        .into_iter()
        .next()
        .and_then(|c| c.message.content)
        .unwrap_or_default();
    let cost = post_process_cost(
        app,
        snapshot,
        chat_response.id.as_deref(),
        chat_response.usage.as_ref(),
    )
    .await;
    let usage = chat_response.usage.clone();

    debug_log::log_model_event(
        app,
        "postProcessing.response",
        ModelRunStage::PostProcessing,
        log_context.as_ref(),
        serde_json::json!({
            "response": {
                "status": status.as_u16(),
                "durationMs": duration_ms,
                "text": content.clone(),
                "usage": usage,
                "cost": cost,
            },
        }),
    );

    Ok(PostProcessRunOutput {
        text: content,
        provider: snapshot.provider.provider_name.clone(),
        model: snapshot.model_label.clone(),
        duration_ms,
        usage: chat_response.usage.map(|raw| RunUsage { raw }),
        cost,
        resolved_provider,
        settings_snapshot: snapshot.clone(),
    })
}

pub fn build_stt_snapshot(app: &tauri::AppHandle) -> AppResult<SttSettingsSnapshot> {
    let config = load_processing_config(app)?;
    let stt = &config.stt;
    let provider_id = stt
        .provider_id
        .clone()
        .ok_or_else(|| i18n::text(app, "config-error-provider-not-selected"))?;
    let model_key = stt
        .model_key
        .clone()
        .ok_or_else(|| i18n::text(app, "config-error-model-not-selected"))?;
    let credentials = resolve_provider_credentials(app, &provider_id)?;
    let model = model_by_key(&model_key)
        .ok_or_else(|| i18n::text(app, "config-error-model-not-found-in-catalog"))?;
    let provider_entry = model
        .entry_for(credentials.kind)
        .ok_or_else(|| i18n::text(app, "config-error-model-not-available-for-provider"))?;
    let ModelParams::Stt(params) = &model.params else {
        return Err(i18n::text(app, "config-error-selected-model-is-not-speech-to-text").into());
    };
    let dictionary = dictionary::load_dictionary_words(app)?.join(", ");
    let system_prompt = stt.effective_system_prompt()?;
    let prompt = apply_template(
        &system_prompt,
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
        prompt_token_limit: params.prompt_token_limit,
    })
}

#[tauri::command]
pub fn analyze_stt_prompt(
    app: tauri::AppHandle,
    system_prompt: Option<String>,
) -> Result<Option<SttPromptAnalysis>, String> {
    build_stt_prompt_analysis(&app, system_prompt).map_err(AppError::into_message)
}

/// Проверяет лимит до передачи аудио провайдеру. Prompt никогда не обрезается.
pub fn ensure_stt_prompt_within_limit(
    app: &tauri::AppHandle,
    snapshot: &SttSettingsSnapshot,
) -> AppResult<()> {
    let Some(analysis) = stt_prompt_analysis(snapshot.prompt_token_limit, &snapshot.prompt) else {
        return Ok(());
    };

    if analysis.excluded_token_count == 0 {
        return Ok(());
    }

    Err(i18n::text_with(
        app,
        "stt-prompt-token-limit-exceeded",
        &[
            ("count", analysis.token_count.to_string()),
            ("limit", analysis.limit.to_string()),
        ],
    )
    .into())
}

fn build_stt_prompt_analysis(
    app: &tauri::AppHandle,
    system_prompt: Option<String>,
) -> AppResult<Option<SttPromptAnalysis>> {
    let config = load_processing_config(app)?;
    let stt = &config.stt;
    let Some(model_key) = stt.model_key.as_deref() else {
        return Ok(None);
    };
    let model = model_by_key(model_key)
        .ok_or_else(|| i18n::text(app, "config-error-model-not-found-in-catalog"))?;
    let ModelParams::Stt(params) = &model.params else {
        return Err(i18n::text(app, "config-error-selected-model-is-not-speech-to-text").into());
    };
    let dictionary = dictionary::load_dictionary_words(app)?.join(", ");
    let prompt = apply_template(
        &system_prompt.unwrap_or(stt.effective_system_prompt()?),
        &[
            ("STT_DICTIONARY", dictionary.as_str()),
            ("CLEANUP_TOOL_AGENT_NAME", AGENT_NAME),
        ],
    );

    Ok(stt_prompt_analysis(params.prompt_token_limit, &prompt))
}

fn stt_prompt_analysis(limit: Option<usize>, prompt: &str) -> Option<SttPromptAnalysis> {
    let limit = limit?;
    let token_count = stt_prompt::count_tokens(prompt);

    Some(SttPromptAnalysis {
        limit,
        token_count,
        usage_percent: token_count as f64 / limit as f64 * 100.0,
        fitting_token_count: token_count.min(limit),
        excluded_token_count: token_count.saturating_sub(limit),
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
        .ok_or_else(|| i18n::text(app, "config-error-provider-not-selected"))?;
    let model_key = post_process
        .model_key
        .clone()
        .ok_or_else(|| i18n::text(app, "config-error-model-not-selected"))?;
    let credentials = resolve_provider_credentials(app, &provider_id)?;
    let model = model_by_key(&model_key)
        .ok_or_else(|| i18n::text(app, "config-error-model-not-found-in-catalog"))?;
    let provider_entry = model
        .entry_for(credentials.kind)
        .ok_or_else(|| i18n::text(app, "config-error-model-not-available-for-provider"))?;
    let ModelParams::PostProcess(params) = &model.params else {
        return Err(i18n::text(app, "config-error-selected-model-is-not-post-processing").into());
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
        openrouter_provider: post_process.openrouter_provider.clone(),
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

fn provider_payload(provider: &ProviderSnapshot) -> serde_json::Value {
    serde_json::json!({
        "providerId": provider.provider_id,
        "providerName": provider.provider_name,
        "providerKind": provider.provider_kind,
        "baseUrl": provider.base_url,
        "headers": debug_log::sanitized_headers(&provider.headers),
    })
}

fn optional_stt_language(snapshot: &SttSettingsSnapshot) -> Option<&str> {
    if snapshot.language == "auto" || snapshot.language.trim().is_empty() {
        None
    } else {
        Some(snapshot.language.as_str())
    }
}

fn optional_prompt(prompt: &str) -> Option<&str> {
    if prompt.trim().is_empty() {
        None
    } else {
        Some(prompt)
    }
}

fn header_map_from_snapshot(
    language: EffectiveUiLanguage,
    headers: &[HeaderSnapshot],
) -> AppResult<HeaderMap> {
    let mut header_map = HeaderMap::new();

    for header in headers {
        let name = HeaderName::from_bytes(header.name.as_bytes()).map_err(|error| {
            i18n::text_for_language(
                language,
                "header-name-invalid",
                &[("name", header.name.clone()), ("error", error.to_string())],
            )
        })?;
        let value = HeaderValue::from_str(&header.value).map_err(|error| {
            i18n::text_for_language(
                language,
                "header-value-invalid",
                &[("name", header.name.clone()), ("error", error.to_string())],
            )
        })?;

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
