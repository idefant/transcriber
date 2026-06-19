use std::time::Duration;

use reqwest::header::{HeaderMap, AUTHORIZATION};
use serde::Deserialize;

use crate::{
    catalog::{model_by_key, ModelParams},
    dictionary,
    error::{AppError, AppResult},
    processing::load_processing_config,
    providers::resolve_provider_credentials,
    settings::get_effective_ui_language,
};

// Substituted for `{{CLEANUP_TOOL_AGENT_NAME}}`. The app has no dedicated agent
// name setting yet, so the product name is used.
const AGENT_NAME: &str = "Transcriber";

// ── Response types ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct SttResponse {
    text: String,
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
    choices: Vec<ChatChoice>,
}

// ── Tauri commands ────────────────────────────────────────────────────────────

#[tauri::command]
pub async fn run_stt_test(
    app: tauri::AppHandle,
    audio: Vec<u8>,
    file_name: String,
) -> Result<String, String> {
    run_stt_test_inner(&app, audio, file_name)
        .await
        .map_err(AppError::into_message)
}

#[tauri::command]
pub async fn run_post_process_test(app: tauri::AppHandle, text: String) -> Result<String, String> {
    run_post_process_test_inner(&app, text)
        .await
        .map_err(AppError::into_message)
}

// ── Inner functions ───────────────────────────────────────────────────────────

async fn run_stt_test_inner(
    app: &tauri::AppHandle,
    audio: Vec<u8>,
    file_name: String,
) -> AppResult<String> {
    let config = load_processing_config(app)?;
    let ui_language = get_effective_ui_language(app)?;
    let stt = &config.stt;

    let provider_id = stt.provider_id.clone().ok_or("Provider is not selected")?;
    let model_key = stt.model_key.clone().ok_or("Model is not selected")?;

    let credentials = resolve_provider_credentials(app, &provider_id)?;
    let model = model_by_key(&model_key).ok_or("Model not found in catalog")?;
    let api_id = model
        .api_id_for(credentials.kind)
        .ok_or("Model is not available for this provider")?;

    let ModelParams::Stt(params) = model.params else {
        return Err("Expected STT model params".into());
    };

    let dictionary = dictionary::load_dictionary_words(app)?.join(", ");
    let prompt = apply_template(
        stt.effective_system_prompt(&ui_language),
        &[
            ("STT_DICTIONARY", dictionary.as_str()),
            ("CLEANUP_TOOL_AGENT_NAME", AGENT_NAME),
        ],
    );

    let mime = mime_from_filename(&file_name);
    let file_part = reqwest::multipart::Part::bytes(audio)
        .file_name(file_name)
        .mime_str(mime)
        .map_err(|e| format!("Invalid MIME type: {e}"))?;

    let form = reqwest::multipart::Form::new()
        .part("file", file_part)
        .text("model", api_id)
        .text("response_format", params.response_format)
        .text("temperature", params.temperature.to_string());

    let language = stt.language.trim();
    let form = if language != "auto" && !language.is_empty() {
        form.text("language", language.to_string())
    } else {
        form
    };

    let form = if !prompt.trim().is_empty() {
        form.text("prompt", prompt)
    } else {
        form
    };

    let url = format!(
        "{}/audio/transcriptions",
        credentials.base_url.trim_end_matches('/')
    );

    let client = build_client()?;
    let mut request = client
        .post(url)
        .header(AUTHORIZATION, format!("Bearer {}", credentials.api_key))
        .multipart(form);

    if !credentials.headers.is_empty() {
        request = request.headers(credentials.headers);
    }

    let response = request.send().await?;
    let status = response.status();

    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("STT request failed with status {status}: {body}").into());
    }

    let stt_response = response.json::<SttResponse>().await?;

    Ok(stt_response.text)
}

async fn run_post_process_test_inner(app: &tauri::AppHandle, text: String) -> AppResult<String> {
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
    let api_id = model
        .api_id_for(credentials.kind)
        .ok_or("Model is not available for this provider")?;

    let ModelParams::PostProcess(params) = model.params else {
        return Err("Expected PostProcess model params".into());
    };

    let system_prompt = apply_template(
        post_process.effective_system_prompt(&ui_language),
        &[("CLEANUP_TOOL_AGENT_NAME", AGENT_NAME)],
    );
    let user_content = apply_template(
        post_process.effective_user_template(),
        &[("TRANSCRIBED_TEXT", text.as_str())],
    );

    let mut body = serde_json::json!({
        "model": api_id,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_content }
        ],
        "temperature": params.temperature,
        // max_completion_tokens is the modern name (required by GPT-5+ on OpenAI);
        // OpenRouter and other compatible APIs also accept it.
        "max_completion_tokens": params.max_tokens,
    });

    if params.disable_thinking {
        body["thinking"] = serde_json::json!({ "type": "disabled" });
    }

    let url = format!(
        "{}/chat/completions",
        credentials.base_url.trim_end_matches('/')
    );

    let mut extra_headers = HeaderMap::new();

    if !credentials.headers.is_empty() {
        extra_headers = credentials.headers;
    }

    let client = build_client()?;
    let response = client
        .post(url)
        .header(AUTHORIZATION, format!("Bearer {}", credentials.api_key))
        .headers(extra_headers)
        .json(&body)
        .send()
        .await?;

    let status = response.status();

    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(format!("Post-process request failed with status {status}: {body}").into());
    }

    let chat_response = response.json::<ChatResponse>().await?;
    let content = chat_response
        .choices
        .into_iter()
        .next()
        .and_then(|c| c.message.content)
        .ok_or("Provider returned an empty response")?;

    Ok(content)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Replace `{{KEY}}` placeholders with their runtime values.
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
