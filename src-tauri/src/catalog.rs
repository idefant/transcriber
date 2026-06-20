use serde::Serialize;

use crate::providers::ProviderKind;

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelTask {
    Stt,
    PostProcess,
}

pub struct SttParams {
    pub temperature: f32,
    pub response_format: &'static str,
}

pub struct PostProcessParams {
    pub temperature: f32,
    pub max_tokens: u32,
    /// Append `/no_think` to the system prompt for Qwen-style models.
    pub disable_thinking_prompt: bool,
    /// Send `thinking: { type: "disabled" }` for Claude models on OpenRouter.
    pub disable_thinking_body: bool,
}

pub enum ModelParams {
    Stt(SttParams),
    PostProcess(PostProcessParams),
}

pub struct ProviderApiEntry {
    pub provider: ProviderKind,
    pub api_id: &'static str,
    pub is_recommended: bool,
    pub disable_reasoning: bool,
}

pub struct CuratedModel {
    pub key: &'static str,
    pub label: &'static str,
    pub task: ModelTask,
    pub entries: Vec<ProviderApiEntry>,
    pub params: ModelParams,
}

impl CuratedModel {
    pub fn entry_for(&self, provider: ProviderKind) -> Option<&ProviderApiEntry> {
        self.entries.iter().find(|entry| entry.provider == provider)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CuratedModelInfo {
    pub key: &'static str,
    pub label: &'static str,
    pub task: ModelTask,
    pub provider_kinds: Vec<ProviderKind>,
    pub provider_entries: Vec<ProviderModelInfo>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderModelInfo {
    pub api_id: &'static str,
    pub provider: ProviderKind,
    pub is_recommended: bool,
}

pub fn curated_models() -> Vec<CuratedModel> {
    vec![
        // ── STT ──────────────────────────────────────────────────────────────
        CuratedModel {
            key: "gpt-4o-transcribe",
            label: "GPT-4o Transcribe",
            task: ModelTask::Stt,
            entries: vec![ProviderApiEntry {
                provider: ProviderKind::Openai,
                api_id: "gpt-4o-transcribe",
                is_recommended: true,
                disable_reasoning: false,
            }],
            params: ModelParams::Stt(SttParams {
                temperature: 0.0,
                response_format: "json",
            }),
        },
        CuratedModel {
            key: "gpt-4o-mini-transcribe",
            label: "GPT-4o mini Transcribe",
            task: ModelTask::Stt,
            entries: vec![ProviderApiEntry {
                provider: ProviderKind::Openai,
                api_id: "gpt-4o-mini-transcribe",
                is_recommended: true,
                disable_reasoning: false,
            }],
            params: ModelParams::Stt(SttParams {
                temperature: 0.0,
                response_format: "json",
            }),
        },
        CuratedModel {
            key: "whisper-large-v3",
            label: "Whisper Large v3",
            task: ModelTask::Stt,
            entries: vec![ProviderApiEntry {
                provider: ProviderKind::Groq,
                api_id: "whisper-large-v3",
                is_recommended: true,
                disable_reasoning: false,
            }],
            params: ModelParams::Stt(SttParams {
                temperature: 0.0,
                response_format: "json",
            }),
        },
        CuratedModel {
            key: "whisper-large-v3-turbo",
            label: "Whisper Large v3 Turbo",
            task: ModelTask::Stt,
            entries: vec![ProviderApiEntry {
                provider: ProviderKind::Groq,
                api_id: "whisper-large-v3-turbo",
                is_recommended: true,
                disable_reasoning: false,
            }],
            params: ModelParams::Stt(SttParams {
                temperature: 0.0,
                response_format: "json",
            }),
        },
        // ── PostProcess ──────────────────────────────────────────────────────
        CuratedModel {
            key: "claude-haiku-4-5",
            label: "Claude Haiku 4.5",
            task: ModelTask::PostProcess,
            entries: vec![ProviderApiEntry {
                provider: ProviderKind::Openrouter,
                api_id: "anthropic/claude-haiku-4.5",
                is_recommended: true,
                disable_reasoning: false,
            }],
            params: ModelParams::PostProcess(PostProcessParams {
                temperature: 0.2,
                max_tokens: 1024,
                disable_thinking_prompt: false,
                disable_thinking_body: true,
            }),
        },
        CuratedModel {
            key: "gpt-4o-mini",
            label: "GPT-4o mini",
            task: ModelTask::PostProcess,
            entries: vec![
                ProviderApiEntry {
                    provider: ProviderKind::Openai,
                    api_id: "gpt-4o-mini",
                    is_recommended: true,
                    disable_reasoning: false,
                },
                ProviderApiEntry {
                    provider: ProviderKind::Openrouter,
                    api_id: "openai/gpt-4o-mini",
                    is_recommended: true,
                    disable_reasoning: false,
                },
            ],
            params: ModelParams::PostProcess(PostProcessParams {
                temperature: 0.2,
                max_tokens: 1024,
                disable_thinking_prompt: false,
                disable_thinking_body: false,
            }),
        },
        CuratedModel {
            key: "gpt-4-1-mini",
            label: "GPT-4.1 mini",
            task: ModelTask::PostProcess,
            entries: vec![
                ProviderApiEntry {
                    provider: ProviderKind::Openai,
                    api_id: "gpt-4.1-mini",
                    is_recommended: true,
                    disable_reasoning: false,
                },
                ProviderApiEntry {
                    provider: ProviderKind::Openrouter,
                    api_id: "openai/gpt-4.1-mini",
                    is_recommended: true,
                    disable_reasoning: false,
                },
            ],
            params: ModelParams::PostProcess(PostProcessParams {
                temperature: 0.2,
                max_tokens: 1024,
                disable_thinking_prompt: false,
                disable_thinking_body: false,
            }),
        },
        CuratedModel {
            key: "gpt-5-mini",
            label: "GPT-5 mini",
            task: ModelTask::PostProcess,
            entries: vec![
                ProviderApiEntry {
                    provider: ProviderKind::Openai,
                    api_id: "gpt-5-mini",
                    is_recommended: false,
                    disable_reasoning: false,
                },
                ProviderApiEntry {
                    provider: ProviderKind::Openrouter,
                    api_id: "openai/gpt-5-mini",
                    is_recommended: false,
                    disable_reasoning: false,
                },
            ],
            params: ModelParams::PostProcess(PostProcessParams {
                temperature: 1.0,
                max_tokens: 1024,
                disable_thinking_prompt: false,
                disable_thinking_body: false,
            }),
        },
        CuratedModel {
            key: "gpt-5-4-mini",
            label: "GPT-5.4 mini",
            task: ModelTask::PostProcess,
            entries: vec![
                ProviderApiEntry {
                    provider: ProviderKind::Openai,
                    api_id: "gpt-5.4-mini",
                    is_recommended: true,
                    disable_reasoning: false,
                },
                ProviderApiEntry {
                    provider: ProviderKind::Openrouter,
                    api_id: "openai/gpt-5.4-mini",
                    is_recommended: true,
                    disable_reasoning: false,
                },
            ],
            params: ModelParams::PostProcess(PostProcessParams {
                temperature: 1.0,
                max_tokens: 1024,
                disable_thinking_prompt: false,
                disable_thinking_body: false,
            }),
        },
        CuratedModel {
            key: "gpt-oss-120b",
            label: "GPT OSS 120B",
            task: ModelTask::PostProcess,
            entries: vec![
                ProviderApiEntry {
                    provider: ProviderKind::Groq,
                    api_id: "openai/gpt-oss-120b",
                    is_recommended: true,
                    disable_reasoning: false,
                },
                ProviderApiEntry {
                    provider: ProviderKind::Openrouter,
                    api_id: "openai/gpt-oss-120b",
                    is_recommended: true,
                    disable_reasoning: false,
                },
            ],
            params: ModelParams::PostProcess(PostProcessParams {
                temperature: 0.2,
                max_tokens: 1024,
                disable_thinking_prompt: false,
                disable_thinking_body: false,
            }),
        },
        CuratedModel {
            key: "qwen-3-6-27b",
            label: "Qwen 3.6 27B",
            task: ModelTask::PostProcess,
            entries: vec![
                ProviderApiEntry {
                    provider: ProviderKind::Groq,
                    api_id: "qwen/qwen3.6-27b",
                    is_recommended: true,
                    disable_reasoning: false,
                },
                ProviderApiEntry {
                    provider: ProviderKind::Openrouter,
                    api_id: "qwen/qwen3.6-27b",
                    is_recommended: true,
                    disable_reasoning: true,
                },
            ],
            params: ModelParams::PostProcess(PostProcessParams {
                temperature: 0.2,
                max_tokens: 1024,
                disable_thinking_prompt: true,
                disable_thinking_body: false,
            }),
        },
        CuratedModel {
            key: "gemini-2-5-flash",
            label: "Gemini 2.5 Flash",
            task: ModelTask::PostProcess,
            entries: vec![ProviderApiEntry {
                provider: ProviderKind::Openrouter,
                api_id: "google/gemini-2.5-flash",
                is_recommended: true,
                disable_reasoning: false,
            }],
            params: ModelParams::PostProcess(PostProcessParams {
                temperature: 0.2,
                max_tokens: 1024,
                disable_thinking_prompt: false,
                disable_thinking_body: false,
            }),
        },
        CuratedModel {
            key: "gemini-2-5-flash-lite",
            label: "Gemini 2.5 Flash Lite",
            task: ModelTask::PostProcess,
            entries: vec![ProviderApiEntry {
                provider: ProviderKind::Openrouter,
                api_id: "google/gemini-2.5-flash-lite",
                is_recommended: true,
                disable_reasoning: false,
            }],
            params: ModelParams::PostProcess(PostProcessParams {
                temperature: 0.2,
                max_tokens: 1024,
                disable_thinking_prompt: false,
                disable_thinking_body: false,
            }),
        },
        CuratedModel {
            key: "gemini-3-1-flash-lite",
            label: "Gemini 3.1 Flash Lite",
            task: ModelTask::PostProcess,
            entries: vec![ProviderApiEntry {
                provider: ProviderKind::Openrouter,
                api_id: "google/gemini-3.1-flash-lite",
                is_recommended: true,
                disable_reasoning: false,
            }],
            params: ModelParams::PostProcess(PostProcessParams {
                temperature: 0.2,
                max_tokens: 1024,
                disable_thinking_prompt: false,
                disable_thinking_body: false,
            }),
        },
        CuratedModel {
            key: "llama-4-scout",
            label: "Llama 4 Scout",
            task: ModelTask::PostProcess,
            entries: vec![
                ProviderApiEntry {
                    provider: ProviderKind::Groq,
                    api_id: "meta-llama/llama-4-scout-17b-16e-instruct",
                    is_recommended: true,
                    disable_reasoning: false,
                },
                ProviderApiEntry {
                    provider: ProviderKind::Openrouter,
                    api_id: "meta-llama/llama-4-scout",
                    is_recommended: true,
                    disable_reasoning: false,
                },
            ],
            params: ModelParams::PostProcess(PostProcessParams {
                temperature: 0.2,
                max_tokens: 1024,
                disable_thinking_prompt: false,
                disable_thinking_body: false,
            }),
        },
    ]
}

pub fn model_by_key(key: &str) -> Option<CuratedModel> {
    curated_models().into_iter().find(|model| model.key == key)
}

#[tauri::command]
pub fn get_model_catalog() -> Vec<CuratedModelInfo> {
    curated_models()
        .into_iter()
        .map(|model| CuratedModelInfo {
            key: model.key,
            label: model.label,
            task: model.task,
            provider_kinds: model.entries.iter().map(|e| e.provider).collect(),
            provider_entries: model
                .entries
                .into_iter()
                .map(|entry| ProviderModelInfo {
                    api_id: entry.api_id,
                    provider: entry.provider,
                    is_recommended: entry.is_recommended,
                })
                .collect(),
        })
        .collect()
}
