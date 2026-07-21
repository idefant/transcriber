use std::time::Instant;

use reqwest::header::{HeaderMap, HeaderName, HeaderValue, AUTHORIZATION};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    catalog::{model_by_key, ModelParams},
    debug_log::{self, ModelRunLogContext, ModelRunSource, ModelRunStage},
    dictionary,
    error::{AppError, AppResult},
    http, i18n,
    metrics::{ProviderCall, ProviderCallStage, ProviderTimings, RunOutcome, RunTimer},
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
    /// Частота дискретизации, в которую приводится запись перед отправкой.
    #[serde(default = "default_input_sample_rate")]
    pub input_sample_rate: u32,
}

/// Значение для снимков, сохранённых до появления поля: у них в истории лежит
/// аудио в частоте устройства, и поле нужно лишь для того, чтобы снимок
/// прочитался.
fn default_input_sample_rate() -> u32 {
    16_000
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
    pub priority_processing: bool,
    pub openrouter_allow_fallbacks: bool,
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
    let timer = RunTimer::new(&ModelRunSource::SettingsTest);
    let result = run_stt_detailed(
        app,
        audio,
        file_name,
        None,
        Some(ModelRunLogContext::settings_test()),
        Some(&timer),
    )
    .await;

    timer.finish(app, run_outcome(result.is_ok(), RunOutcome::SttError));

    Ok(result?.text)
}

pub async fn run_post_process(app: &tauri::AppHandle, text: String) -> AppResult<String> {
    let timer = RunTimer::new(&ModelRunSource::SettingsTest);
    let result = run_post_process_detailed(
        app,
        text,
        Some(ModelRunLogContext::settings_test()),
        Some(&timer),
    )
    .await;

    timer.finish(
        app,
        run_outcome(result.is_ok(), RunOutcome::PostProcessError),
    );

    Ok(result?.text)
}

fn run_outcome(is_ok: bool, failure: RunOutcome) -> RunOutcome {
    if is_ok {
        RunOutcome::Completed
    } else {
        failure
    }
}

pub async fn run_stt_detailed(
    app: &tauri::AppHandle,
    audio: Vec<u8>,
    file_name: String,
    audio_duration_ms: Option<u64>,
    log_context: Option<ModelRunLogContext>,
    timer: Option<&RunTimer>,
) -> AppResult<SttRunOutput> {
    let snapshot = build_stt_snapshot(app)?;

    run_stt_with_snapshot(
        app,
        &snapshot,
        audio,
        file_name,
        audio_duration_ms,
        log_context,
        timer,
    )
    .await
}

pub async fn run_post_process_detailed(
    app: &tauri::AppHandle,
    text: String,
    log_context: Option<ModelRunLogContext>,
    timer: Option<&RunTimer>,
) -> AppResult<PostProcessRunOutput> {
    let snapshot = build_post_process_snapshot(app)?;

    run_post_process_with_snapshot(app, &snapshot, text, log_context, timer).await
}

pub async fn run_stt_with_snapshot(
    app: &tauri::AppHandle,
    snapshot: &SttSettingsSnapshot,
    audio: Vec<u8>,
    file_name: String,
    audio_duration_ms: Option<u64>,
    log_context: Option<ModelRunLogContext>,
    timer: Option<&RunTimer>,
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

    let mut call = CallMetrics::new(
        ProviderCallStage::Stt,
        &snapshot.provider,
        &snapshot.api_model_id,
        Some(audio_size_bytes as u64),
        started_at,
    );
    let response = match request.send().await {
        Ok(response) => response,
        Err(error) => {
            call.fail(&error);
            call.commit(timer);
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
    let headers = response.headers().clone();
    call.received_headers(status.as_u16());

    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        call.received_body(provider_timings(
            snapshot.provider.provider_kind,
            &headers,
            None,
        ));
        call.commit(timer);
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
            call.fail(&error);
            call.commit(timer);
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
    call.received_body(provider_timings(
        snapshot.provider.provider_kind,
        &headers,
        stt_response.usage.as_ref(),
    ));
    call.commit(timer);
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
    timer: Option<&RunTimer>,
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

    if snapshot.priority_processing {
        body["service_tier"] = serde_json::json!("priority");
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
                "allow_fallbacks": snapshot.openrouter_allow_fallbacks,
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

    let request_bytes = serde_json::to_vec(&body)
        .map(|bytes| bytes.len() as u64)
        .ok();
    let mut call = CallMetrics::new(
        ProviderCallStage::PostProcess,
        &snapshot.provider,
        &snapshot.api_model_id,
        request_bytes,
        started_at,
    );
    let response = match request.send().await {
        Ok(response) => response,
        Err(error) => {
            call.fail(&error);
            call.commit(timer);
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
    let response_headers = response.headers().clone();
    call.received_headers(status.as_u16());

    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        call.received_body(provider_timings(
            snapshot.provider.provider_kind,
            &response_headers,
            None,
        ));
        call.commit(timer);
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
            call.fail(&error);
            call.commit(timer);
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
    let mut timings = provider_timings(
        snapshot.provider.provider_kind,
        &response_headers,
        chat_response.usage.as_ref(),
    );
    timings.upstream_provider = resolved_provider.clone();
    call.received_body(timings);
    call.commit(timer);
    let content = chat_response
        .choices
        .into_iter()
        .next()
        .and_then(|c| c.message.content)
        .unwrap_or_default();
    let cost = chat_response.usage.as_ref().and_then(find_cost);

    if cost.is_none() {
        spawn_openrouter_cost_backfill(
            app,
            snapshot,
            chat_response.id.as_deref(),
            log_context
                .as_ref()
                .and_then(|context| context.history_record_id.clone()),
            timer.map(|timer| timer.id().to_string()),
        );
    }

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
        input_sample_rate: params.input_sample_rate,
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
        priority_processing: post_process.priority_processing
            && (credentials.kind == ProviderKind::Openrouter
                || model.supports_priority_processing(credentials.kind)),
        openrouter_allow_fallbacks: post_process.openrouter_allow_fallbacks,
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

/// Догружает стоимость генерации OpenRouter уже после того, как пользователь
/// получил свой текст.
///
/// Раньше этот запрос выполнялся до вставки и добавлял на горячий путь целый
/// лишний round-trip. Стоимость нужна только для истории, поэтому её не
/// обязательно знать к моменту вставки: фоновая задача дозаписывает её в
/// запись, а фронтенд перечитывает месяц по событию `history-updated`.
///
/// Стоимость запрашивается, только если её не оказалось в `usage` ответа, и
/// только у OpenRouter: другие провайдеры такого эндпоинта не имеют.
fn spawn_openrouter_cost_backfill(
    app: &tauri::AppHandle,
    snapshot: &PostProcessSettingsSnapshot,
    generation_id: Option<&str>,
    history_record_id: Option<String>,
    run_id: Option<String>,
) {
    if !matches!(snapshot.provider.provider_kind, ProviderKind::Openrouter) {
        return;
    }

    let Some(generation_id) = generation_id.map(ToString::to_string) else {
        return;
    };

    let app = app.clone();
    let provider = snapshot.provider.clone();
    let model = snapshot.api_model_id.clone();

    tauri::async_runtime::spawn(async move {
        let started_at = Instant::now();
        let Some((cost, call)) =
            request_openrouter_generation(&app, &provider, &model, &generation_id, started_at)
                .await
        else {
            return;
        };

        if let Some(run_id) = run_id {
            crate::metrics::record_background_call(&app, &run_id, call);
        }

        if let (Some(cost), Some(record_id)) = (cost, history_record_id) {
            let _ = crate::history::set_post_processing_cost(&app, &record_id, cost);
        }
    });
}

/// Запрашивает у OpenRouter сведения о завершённой генерации. Возвращает
/// стоимость и метрики самого запроса: он тоже часть картины задержек, пусть и
/// вне пути пользователя.
async fn request_openrouter_generation(
    app: &tauri::AppHandle,
    provider: &ProviderSnapshot,
    model: &str,
    generation_id: &str,
    started_at: Instant,
) -> Option<(Option<f64>, ProviderCall)> {
    let api_key = resolve_provider_api_key(app, &provider.provider_id).ok()?;
    let url = format!(
        "{}/generation?id={}",
        provider.base_url.trim_end_matches('/'),
        generation_id
    );
    let client = build_client().ok()?;
    let mut call = CallMetrics::new(
        ProviderCallStage::OpenrouterGeneration,
        provider,
        model,
        None,
        started_at,
    );
    let response = match client
        .get(url)
        .header(AUTHORIZATION, format!("Bearer {api_key}"))
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => {
            call.fail(&error);

            return Some((None, call.into_provider_call()));
        }
    };
    let status = response.status();
    let headers = response.headers().clone();
    call.received_headers(status.as_u16());

    let value = response.json::<serde_json::Value>().await.ok();
    let mut timings = provider_timings(provider.provider_kind, &headers, None);

    if let Some(value) = value.as_ref() {
        apply_openrouter_generation_timings(&mut timings, value);
    }

    call.received_body(timings);

    Some((
        value.as_ref().and_then(find_cost),
        call.into_provider_call(),
    ))
}

/// Переносит тайминги генерации OpenRouter в общий вид.
///
/// `latency` — время до первого токена, `generation_time` — длительность самой
/// генерации; остальное складывается в `raw`, потому что своих колонок не имеет.
fn apply_openrouter_generation_timings(timings: &mut ProviderTimings, value: &serde_json::Value) {
    let data = value.get("data").unwrap_or(value);

    timings.ttft_ms = data
        .get("latency")
        .and_then(serde_json::Value::as_f64)
        .map(|value| value as u64);
    timings.total_ms = data
        .get("generation_time")
        .and_then(serde_json::Value::as_f64)
        .map(|value| value as u64)
        .or(timings.total_ms);
    timings.upstream_provider = data
        .get("provider_name")
        .and_then(serde_json::Value::as_str)
        .map(ToString::to_string);

    if let Some(moderation_latency) = data.get("moderation_latency") {
        let mut raw = match timings.raw.take() {
            Some(serde_json::Value::Object(raw)) => raw,
            _ => serde_json::Map::new(),
        };

        raw.insert("moderation_latency".to_string(), moderation_latency.clone());
        timings.raw = Some(serde_json::Value::Object(raw));
    }
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
    http::elapsed_ms(started_at)
}

/// Накопитель метрик одного HTTP-вызова.
///
/// Разделяет время до заголовков ответа и время дочитывания тела: первое —
/// это отправка запроса плюс работа провайдера, второе — только загрузка
/// ответа. Раньше обе части сливались в одно число, и по нему нельзя было
/// понять, чего именно ждали.
struct CallMetrics {
    id: String,
    stage: ProviderCallStage,
    provider_kind: String,
    provider_id: String,
    base_url: String,
    model: String,
    request_bytes: Option<u64>,
    status: Option<u16>,
    error_kind: Option<String>,
    started_at: Instant,
    headers_ms: u64,
    headers_at: Option<Instant>,
    body_ms: u64,
    provider: ProviderTimings,
}

impl CallMetrics {
    fn new(
        stage: ProviderCallStage,
        provider: &ProviderSnapshot,
        model: &str,
        request_bytes: Option<u64>,
        started_at: Instant,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            stage,
            provider_kind: provider.provider_kind.as_str().to_string(),
            provider_id: provider.provider_id.clone(),
            base_url: provider.base_url.clone(),
            model: model.to_string(),
            request_bytes,
            status: None,
            error_kind: None,
            started_at,
            headers_ms: 0,
            headers_at: None,
            body_ms: 0,
            provider: ProviderTimings::default(),
        }
    }

    fn received_headers(&mut self, status: u16) {
        self.status = Some(status);
        self.headers_ms = elapsed_ms(self.started_at);
        self.headers_at = Some(Instant::now());
    }

    fn received_body(&mut self, provider: ProviderTimings) {
        if let Some(headers_at) = self.headers_at {
            self.body_ms = elapsed_ms(headers_at);
        }

        self.provider = provider;
    }

    /// Фиксирует неудачу. Если ответ не пришёл вовсе, время до отказа
    /// записывается как `headers_ms`: ждали именно его.
    fn fail(&mut self, error: &reqwest::Error) {
        self.error_kind = Some(http::classify_error(error));

        if self.headers_at.is_none() {
            self.headers_ms = elapsed_ms(self.started_at);
        }
    }

    fn commit(self, timer: Option<&RunTimer>) {
        let Some(timer) = timer else {
            return;
        };

        timer.record_call(self.into_provider_call());
    }

    fn into_provider_call(self) -> ProviderCall {
        ProviderCall {
            id: self.id,
            stage: self.stage,
            provider_kind: self.provider_kind,
            provider_id: self.provider_id,
            base_url: self.base_url,
            model: self.model,
            status: self.status,
            error_kind: self.error_kind,
            request_bytes: self.request_bytes,
            headers_ms: self.headers_ms,
            body_ms: self.body_ms,
            provider: self.provider,
        }
    }
}

/// Собирает тайминги, сообщённые провайдером, из заголовков ответа и `usage`.
///
/// Провайдеры делятся ими по-разному: OpenAI кладёт время обработки в заголовок
/// `openai-processing-ms`, Groq — в `usage` ответа chat/completions (в
/// секундах), а для распознавания речи не сообщает его вовсе. Ничего из этого
/// не является обязательным, поэтому все поля результата необязательные.
fn provider_timings(
    kind: ProviderKind,
    headers: &HeaderMap,
    usage: Option<&serde_json::Value>,
) -> ProviderTimings {
    let mut timings = ProviderTimings {
        total_ms: header_as_f64(headers, "openai-processing-ms").map(|value| value as u64),
        request_id: header_as_string(headers, "x-request-id"),
        retry_after_ms: header_as_f64(headers, "retry-after")
            .map(|seconds| (seconds * 1000.0) as u64),
        ..ProviderTimings::default()
    };

    if let Some(usage) = usage {
        if matches!(kind, ProviderKind::Groq) {
            // Groq отдаёт длительности в секундах с плавающей точкой.
            timings.total_ms = seconds_field_as_ms(usage, "total_time").or(timings.total_ms);
            timings.queue_ms = seconds_field_as_ms(usage, "queue_time");
        }
    }

    let raw = raw_timing_headers(headers);

    if !raw.is_empty() {
        timings.raw = Some(serde_json::Value::Object(raw));
    }

    timings
}

/// Заголовки, полезные для разбора задержек, но не имеющие своей колонки.
/// Значения не содержат секретов: это идентификаторы запроса, лимиты и
/// диагностика прокси.
const RAW_TIMING_HEADERS: [&str; 6] = [
    "server-timing",
    "x-ratelimit-remaining-requests",
    "x-ratelimit-remaining-tokens",
    "x-ratelimit-reset-requests",
    "x-groq-region",
    "cf-ray",
];

fn raw_timing_headers(headers: &HeaderMap) -> serde_json::Map<String, serde_json::Value> {
    RAW_TIMING_HEADERS
        .iter()
        .filter_map(|name| {
            let value = header_as_string(headers, name)?;

            Some(((*name).to_string(), serde_json::Value::String(value)))
        })
        .collect()
}

fn header_as_string(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)?
        .to_str()
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn header_as_f64(headers: &HeaderMap, name: &str) -> Option<f64> {
    header_as_string(headers, name)?.parse().ok()
}

fn seconds_field_as_ms(value: &serde_json::Value, key: &str) -> Option<u64> {
    let seconds = value.get(key)?.as_f64()?;

    if seconds < 0.0 {
        return None;
    }

    Some((seconds * 1000.0).round() as u64)
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
    http::processing_client()
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
