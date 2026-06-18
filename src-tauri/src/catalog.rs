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
    /// Send `thinking: { type: "disabled" }` for Claude models on OpenRouter.
    pub disable_thinking: bool,
}

pub enum ModelParams {
    Stt(SttParams),
    PostProcess(PostProcessParams),
}

pub struct ProviderApiEntry {
    pub provider: ProviderKind,
    pub api_id: &'static str,
}

pub struct CuratedModel {
    pub key: &'static str,
    pub label: &'static str,
    pub task: ModelTask,
    pub entries: Vec<ProviderApiEntry>,
    pub params: ModelParams,
}

impl CuratedModel {
    pub fn api_id_for(&self, provider: ProviderKind) -> Option<&'static str> {
        self.entries
            .iter()
            .find(|entry| entry.provider == provider)
            .map(|entry| entry.api_id)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CuratedModelInfo {
    pub key: &'static str,
    pub label: &'static str,
    pub task: ModelTask,
    pub provider_kinds: Vec<ProviderKind>,
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
            }],
            params: ModelParams::Stt(SttParams {
                temperature: 0.0,
                response_format: "json",
            }),
        },
        // ── PostProcess ──────────────────────────────────────────────────────
        CuratedModel {
            key: "claude-haiku-3-5",
            label: "Claude Haiku 3.5",
            task: ModelTask::PostProcess,
            entries: vec![ProviderApiEntry {
                provider: ProviderKind::Openrouter,
                api_id: "anthropic/claude-3-5-haiku",
            }],
            params: ModelParams::PostProcess(PostProcessParams {
                temperature: 0.2,
                max_tokens: 4096,
                disable_thinking: true,
            }),
        },
        CuratedModel {
            key: "claude-haiku-4-5",
            label: "Claude Haiku 4.5",
            task: ModelTask::PostProcess,
            entries: vec![ProviderApiEntry {
                provider: ProviderKind::Openrouter,
                api_id: "anthropic/claude-haiku-4-5",
            }],
            params: ModelParams::PostProcess(PostProcessParams {
                temperature: 0.2,
                max_tokens: 4096,
                disable_thinking: true,
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
                },
                ProviderApiEntry {
                    provider: ProviderKind::Openrouter,
                    api_id: "openai/gpt-4o-mini",
                },
            ],
            params: ModelParams::PostProcess(PostProcessParams {
                temperature: 0.2,
                max_tokens: 4096,
                disable_thinking: false,
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
                },
                ProviderApiEntry {
                    provider: ProviderKind::Openrouter,
                    api_id: "openai/gpt-5-mini",
                },
            ],
            params: ModelParams::PostProcess(PostProcessParams {
                temperature: 1.0,
                max_tokens: 4096,
                disable_thinking: false,
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
                },
                ProviderApiEntry {
                    provider: ProviderKind::Openrouter,
                    api_id: "openai/gpt-5.4-mini",
                },
            ],
            params: ModelParams::PostProcess(PostProcessParams {
                temperature: 0.2,
                max_tokens: 4096,
                disable_thinking: false,
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
            provider_kinds: model.entries.into_iter().map(|e| e.provider).collect(),
        })
        .collect()
}
