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

## Linux development (Omarchy / Hyprland)

Windows remains supported with its existing WASAPI capture, keyboard hook, clipboard handling and NSIS packaging. Tauri automatically applies the platform configuration (`tauri.windows.conf.json` or `tauri.linux.conf.json`).

On Arch Linux, install `base-devel rust nodejs npm webkit2gtk-4.1 gtk3 libayatana-appindicator librsvg patchelf pipewire-audio wireplumber wl-clipboard wtype playerctl libnotify`. The application expects a working PipeWire and Hyprland session. Then run:

```bash
npm ci
npm run setup:linux
npm run dev:linux
```

`setup:linux` downloads ONNX Runtime 1.22.0 for Linux x86_64 from the upstream release and verifies pinned SHA-256 checksums. The generated shared library is excluded from Git; the VAD model and third-party notices are included on both platforms. Linux ARM64 packaging is not currently provided.

`npm run build:linux` builds Linux distribution bundles. `npm run dev:linux:debug` exposes the WebKit inspector on loopback port 9222 for local development only. Ordinary development and production builds do not enable that server.

Linux capture uses the default PipeWire source and converts it to mono float32 at 48 kHz. Pausing, cancelling or stopping a recording terminates the capture client and releases the microphone. Resuming opens the source again, so Linux does not share Windows' fully prewarmed hardware stream latency.
