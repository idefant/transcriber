# Transcriber

Transcriber is a desktop dictation app. It records voice from the microphone, sends audio to a speech-to-text model, can clean the transcript with a post-processing model, pastes the final text into the active application, and stores the audio and processing results in history.

The product behavior is documented in the functional specification: [docs/functional-spec/index.md](docs/functional-spec/index.md).

Development setup, build commands, quality checks, and project tooling are documented in [docs/development/index.md](docs/development/index.md).

## Available Post-processing Models

| Модель                | Провайдер          | Рекомендуется |
| --------------------- | ------------------ | ------------- |
| gpt-4o-mini           | OpenAI, OpenRouter | ✅            |
| gpt-4.1-mini          | OpenAI, OpenRouter | ✅            |
| gpt-5.4-mini          | OpenAI, OpenRouter | ✅            |
| gpt-5-mini            | OpenAI, OpenRouter | ❌            |
| Qwen 3.6 27B          | Groq, OpenRouter   | ✅            |
| GPT OSS 120B          | Groq, OpenRouter   | ✅            |
| Llama 4 Scout         | Groq, OpenRouter   | ✅            |
| Gemini 2.5 Flash      | OpenRouter         | ✅            |
| Gemini 2.5 Flash Lite | OpenRouter         | ✅            |
| Gemini 3.1 Flash Lite | OpenRouter         | ✅            |
| Claude Haiku 4.5      | OpenRouter         | ✅            |
