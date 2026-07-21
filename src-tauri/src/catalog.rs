use serde::Serialize;

use crate::providers::ProviderKind;

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ModelTask {
    Stt,
    PostProcess,
}

pub struct SttParams {
    pub prompt_token_limit: Option<usize>,
    pub temperature: f32,
    pub response_format: &'static str,
    /// Частота дискретизации, на которой модель работает внутри.
    ///
    /// Запись приводится к ней перед отправкой: посылать больше бессмысленно —
    /// провайдер всё равно пересчитает вход и лишние байты уйдут впустую, — а
    /// посылать меньше значит отдать модели полосу уже, чем та, на которой её
    /// обучали. Апсемплинг не выполняется: если устройство отдаёт меньше, чем
    /// хочет модель, запись уходит как есть.
    pub input_sample_rate: u32,
}

pub struct PostProcessParams {
    pub temperature: f32,
    pub max_tokens: u32,
    /// Добавляет `/no_think` к системному промпту для моделей в стиле Qwen.
    pub disable_thinking_prompt: bool,
    /// Отправляет `thinking: { type: "disabled" }` для моделей Claude в OpenRouter.
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
    pub reasoning_effort: Option<&'static str>,
    pub reasoning_format: Option<&'static str>,
    pub include_reasoning: Option<bool>,
    pub reasoning: Option<ReasoningParams>,
}

pub struct ReasoningParams {
    pub effort: &'static str,
    pub exclude: bool,
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

    /// Поддерживает ли связка модели и прямого провайдера приоритетную обработку.
    pub fn supports_priority_processing(&self, provider: ProviderKind) -> bool {
        self.task == ModelTask::PostProcess
            && provider == ProviderKind::Openai
            && matches!(
                self.key,
                "gpt-4o-mini" | "gpt-4-1-mini" | "gpt-5-mini" | "gpt-5-4-mini"
            )
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
    pub stt_prompt_token_limit: Option<usize>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderModelInfo {
    pub api_id: &'static str,
    pub provider: ProviderKind,
    pub is_recommended: bool,
    pub supports_priority_processing: bool,
}

pub fn curated_models() -> Vec<CuratedModel> {
    vec![
        // STT
        CuratedModel {
            key: "gpt-4o-transcribe",
            label: "GPT-4o Transcribe",
            task: ModelTask::Stt,
            entries: vec![ProviderApiEntry {
                provider: ProviderKind::Openai,
                api_id: "gpt-4o-transcribe",
                is_recommended: true,
                reasoning_effort: None,
                reasoning_format: None,
                include_reasoning: None,
                reasoning: None,
            }],
            params: ModelParams::Stt(SttParams {
                prompt_token_limit: None,
                temperature: 0.0,
                response_format: "json",
                // Аудио-стек GPT-4o работает на 24 кГц: Realtime API той же
                // модели принимает PCM16 строго на этой частоте. Препроцессинг
                // batch-эндпоинта OpenAI не документирует, поэтому берём
                // осторожное значение — понижать до 16 кГц значило бы отдать
                // модели полосу уже, чем та, на которой её обучали.
                input_sample_rate: 24_000,
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
                reasoning_effort: None,
                reasoning_format: None,
                include_reasoning: None,
                reasoning: None,
            }],
            params: ModelParams::Stt(SttParams {
                prompt_token_limit: None,
                temperature: 0.0,
                response_format: "json",
                input_sample_rate: 24_000,
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
                reasoning_effort: None,
                reasoning_format: None,
                include_reasoning: None,
                reasoning: None,
            }],
            params: ModelParams::Stt(SttParams {
                prompt_token_limit: Some(224),
                temperature: 0.0,
                response_format: "json",
                // Whisper приводит любой вход к 16 кГц моно сам: `SAMPLE_RATE`
                // зашит в архитектуру, mel-спектрограмма считается на ней.
                // Отправлять больше — значит платить за передачу того, что
                // модель всё равно отбросит.
                input_sample_rate: 16_000,
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
                reasoning_effort: None,
                reasoning_format: None,
                include_reasoning: None,
                reasoning: None,
            }],
            params: ModelParams::Stt(SttParams {
                prompt_token_limit: Some(224),
                temperature: 0.0,
                response_format: "json",
                input_sample_rate: 16_000,
            }),
        },
        // PostProcess
        CuratedModel {
            key: "claude-haiku-4-5",
            label: "Claude Haiku 4.5",
            task: ModelTask::PostProcess,
            entries: vec![ProviderApiEntry {
                provider: ProviderKind::Openrouter,
                api_id: "anthropic/claude-haiku-4.5",
                is_recommended: true,
                reasoning_effort: None,
                reasoning_format: None,
                include_reasoning: None,
                reasoning: None,
            }],
            params: ModelParams::PostProcess(PostProcessParams {
                temperature: 0.2,
                max_tokens: 4096,
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
                    is_recommended: false,
                    reasoning_effort: None,
                    reasoning_format: None,
                    include_reasoning: None,
                    reasoning: None,
                },
                ProviderApiEntry {
                    provider: ProviderKind::Openrouter,
                    api_id: "openai/gpt-4o-mini",
                    is_recommended: false,
                    reasoning_effort: None,
                    reasoning_format: None,
                    include_reasoning: None,
                    reasoning: None,
                },
            ],
            params: ModelParams::PostProcess(PostProcessParams {
                temperature: 0.2,
                max_tokens: 4096,
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
                    is_recommended: false,
                    reasoning_effort: None,
                    reasoning_format: None,
                    include_reasoning: None,
                    reasoning: None,
                },
                ProviderApiEntry {
                    provider: ProviderKind::Openrouter,
                    api_id: "openai/gpt-4.1-mini",
                    is_recommended: false,
                    reasoning_effort: None,
                    reasoning_format: None,
                    include_reasoning: None,
                    reasoning: None,
                },
            ],
            params: ModelParams::PostProcess(PostProcessParams {
                temperature: 0.2,
                max_tokens: 4096,
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
                    is_recommended: true,
                    reasoning_effort: Some("minimal"),
                    reasoning_format: None,
                    include_reasoning: None,
                    reasoning: None,
                },
                ProviderApiEntry {
                    provider: ProviderKind::Openrouter,
                    api_id: "openai/gpt-5-mini",
                    is_recommended: true,
                    reasoning_effort: None,
                    reasoning_format: None,
                    include_reasoning: None,
                    reasoning: Some(ReasoningParams {
                        effort: "minimal",
                        exclude: true,
                    }),
                },
            ],
            params: ModelParams::PostProcess(PostProcessParams {
                temperature: 1.0,
                max_tokens: 4096,
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
                    reasoning_effort: Some("none"),
                    reasoning_format: None,
                    include_reasoning: None,
                    reasoning: None,
                },
                ProviderApiEntry {
                    provider: ProviderKind::Openrouter,
                    api_id: "openai/gpt-5.4-mini",
                    is_recommended: true,
                    reasoning_effort: None,
                    reasoning_format: None,
                    include_reasoning: None,
                    reasoning: Some(ReasoningParams {
                        effort: "none",
                        exclude: false,
                    }),
                },
            ],
            params: ModelParams::PostProcess(PostProcessParams {
                temperature: 1.0,
                max_tokens: 4096,
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
                    reasoning_effort: Some("low"),
                    reasoning_format: None,
                    include_reasoning: Some(false),
                    reasoning: None,
                },
                ProviderApiEntry {
                    provider: ProviderKind::Openrouter,
                    api_id: "openai/gpt-oss-120b",
                    is_recommended: true,
                    reasoning_effort: None,
                    reasoning_format: None,
                    include_reasoning: None,
                    reasoning: Some(ReasoningParams {
                        effort: "low",
                        exclude: true,
                    }),
                },
            ],
            params: ModelParams::PostProcess(PostProcessParams {
                temperature: 0.2,
                max_tokens: 4096,
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
                    reasoning_effort: Some("none"),
                    reasoning_format: Some("hidden"),
                    include_reasoning: None,
                    reasoning: None,
                },
                ProviderApiEntry {
                    provider: ProviderKind::Openrouter,
                    api_id: "qwen/qwen3.6-27b",
                    is_recommended: true,
                    reasoning_effort: None,
                    reasoning_format: None,
                    include_reasoning: None,
                    reasoning: Some(ReasoningParams {
                        effort: "none",
                        exclude: false,
                    }),
                },
            ],
            params: ModelParams::PostProcess(PostProcessParams {
                temperature: 0.2,
                max_tokens: 4096,
                disable_thinking_prompt: true,
                disable_thinking_body: false,
            }),
        },
        CuratedModel {
            key: "qwen-3-6-35b-a3b",
            label: "Qwen 3.6 35B A3B",
            task: ModelTask::PostProcess,
            entries: vec![ProviderApiEntry {
                provider: ProviderKind::Openrouter,
                api_id: "qwen/qwen3.6-35b-a3b",
                is_recommended: true,
                reasoning_effort: None,
                reasoning_format: None,
                include_reasoning: None,
                reasoning: Some(ReasoningParams {
                    effort: "none",
                    exclude: false,
                }),
            }],
            params: ModelParams::PostProcess(PostProcessParams {
                temperature: 0.2,
                max_tokens: 4096,
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
                reasoning_effort: None,
                reasoning_format: None,
                include_reasoning: None,
                reasoning: Some(ReasoningParams {
                    effort: "none",
                    exclude: false,
                }),
            }],
            params: ModelParams::PostProcess(PostProcessParams {
                temperature: 0.2,
                max_tokens: 4096,
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
                is_recommended: false,
                reasoning_effort: None,
                reasoning_format: None,
                include_reasoning: None,
                reasoning: Some(ReasoningParams {
                    effort: "none",
                    exclude: false,
                }),
            }],
            params: ModelParams::PostProcess(PostProcessParams {
                temperature: 0.2,
                max_tokens: 4096,
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
                is_recommended: false,
                reasoning_effort: None,
                reasoning_format: None,
                include_reasoning: None,
                reasoning: Some(ReasoningParams {
                    effort: "none",
                    exclude: false,
                }),
            }],
            params: ModelParams::PostProcess(PostProcessParams {
                temperature: 0.2,
                max_tokens: 4096,
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
                    is_recommended: false,
                    reasoning_effort: None,
                    reasoning_format: None,
                    include_reasoning: None,
                    reasoning: None,
                },
                ProviderApiEntry {
                    provider: ProviderKind::Openrouter,
                    api_id: "meta-llama/llama-4-scout",
                    is_recommended: false,
                    reasoning_effort: None,
                    reasoning_format: None,
                    include_reasoning: None,
                    reasoning: None,
                },
            ],
            params: ModelParams::PostProcess(PostProcessParams {
                temperature: 0.2,
                max_tokens: 4096,
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
                .iter()
                .map(|entry| ProviderModelInfo {
                    api_id: entry.api_id,
                    provider: entry.provider,
                    is_recommended: entry.is_recommended,
                    supports_priority_processing: model
                        .supports_priority_processing(entry.provider),
                })
                .collect(),
            stt_prompt_token_limit: match model.params {
                ModelParams::Stt(ref params) => params.prompt_token_limit,
                ModelParams::PostProcess(_) => None,
            },
        })
        .collect()
}
