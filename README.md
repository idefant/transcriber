# Transcriber

Transcriber is a desktop dictation app. It records voice from the microphone, sends audio to a speech-to-text model, can clean the transcript with a post-processing model, pastes the final text into the active application, and stores the audio and processing results in history.

The product behavior is documented in the functional specification: [docs/functional-spec/index.md](docs/functional-spec/index.md).

Development setup, build commands, quality checks, and project tooling are documented in [docs/development/index.md](docs/development/index.md).

## Available Post-processing Models

| Модель                 | Провайдер          | Рекомендуется | Балл  |
| ---------------------- | ------------------ | ------------- | ----- |
| ❰ OpenAI ❱             |                    |               |       |
| GPT OSS 120B           | Groq, OpenRouter   | ✅            | 98.38 |
| gpt-5.4-mini           | OpenAI, OpenRouter | ✅            | 97.88 |
| gpt-5-mini             | OpenAI, OpenRouter | ✅            | 98.68 |
| gpt-4.1-mini           | OpenAI, OpenRouter | ❌            | 93.26 |
| gpt-4o-mini            | OpenAI, OpenRouter | ❌            | 92.71 |
| ❰ Alibaba ❱            |                    |               |       |
| Qwen 3.6 35B A3B       | OpenRouter         | ✅            | ???   |
| Qwen 3.6 27B           | Groq, OpenRouter   | ✅            | 97.50 |
| ❰ Meta ❱               |                    |               |       |
| Llama 4 Scout          | OpenRouter         | ❌            | 84.50 |
| ❰ Google ❱             |                    |               |       |
| Gemini 3.8 Flash       | OpenRouter         | ✅            | 100   |
| Gemini 3.7 Flash       | OpenRouter         | ✅            | 100   |
| Gemini 3.1 Flash Lite  | OpenRouter         | ❌            | 92.18 |
| Gemini 2.5 Flash       | OpenRouter         | ✅            | 97.71 |
| Gemini 2.5 Flash Lite  | OpenRouter         | ❌            | 90.09 |
| ❰ Claude ❱             |                    |               |       |
| Claude Haiku 4.5       | OpenRouter         | ✅            | 92.50 |
| ❰ Z.ai ❱               |                    |               |       |
| GLM 5.3                | OpenRouter         | ✅            | 97.94 |
| GLM 5.3 Flash          | OpenRouter         | ✅            | 100   |
| ❰ DeepSeek ❱           |                    |               |       |
| DeepSeek V4.1 Flash    | OpenRouter         | ✅            | 100   |
| DeepSeek V4 Pro 0813   | OpenRouter         | ✅            | 100   |
| DeepSeek V4 Flash 0731 | OpenRouter         | ✅            | 99.56 |
