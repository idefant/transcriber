# Model Testing

The post-processing model test runner is a standalone Node tool. It does not
change application settings and does not require the Tauri UI.

## Configuration

Files live in `scripts/model-testing/`:

- `models.json` - model list, provider URL, API key env var, enabled flag, request params, and prices per 1M tokens.
- `cases.json` - input prompts and deterministic scoring rules.
- `post-process-prompts.json` - shared default post-processing prompts used by both the app and the runner.
- `run.mjs` - executes the matrix and writes `results.json`.
- `report.mjs` - generates `report.html`.

API keys are read from process env, `.env`, then `.env.local`:

```text
OPENAI_API_KEY=...
GROQ_API_KEY=...
OPENROUTER_API_KEY=...
```

`.env` and `.env.local` are ignored by git.

## Commands

Run all enabled models with default settings:

```bash
npm.cmd run model-test
```

Defaults:

- `repeats`: `5`
- `languages`: `ru,en`
- cases: all entries from `cases.json`
- models: all entries with `enabled: true`

Run a smoke test for one model and one prompt:

```bash
npm.cmd run model-test:smoke -- --models openai-gpt-4o-mini --prompt "ты умеешь читать?" --languages ru --repeats 1
```

Override repeats and languages:

```bash
npm.cmd run model-test -- --repeats 3 --languages ru,en
```

Run only selected models or cases:

```bash
npm.cmd run model-test -- --models openai-gpt-4o-mini,openrouter-claude-haiku-4-5 --cases reading-question,math-spoken
```

Write output to a fixed folder:

```bash
npm.cmd run model-test -- --output reports/model-testing/manual-run
```

## Output

Each run writes:

```text
reports/model-testing/<timestamp>/
  results.json
  report.html
```

`report.html` contains:

- model ranking by response quality;
- average score, errors, latency, and estimated cost;
- per-model details for every case, language, and repeat;
- penalties that formed each response score;
- links from ranking rows to model details;
- a fixed circular back-to-top button that hides while the ranking is visible.

Latency and price are shown for humans only. They do not affect the ranking.

## Scoring

Every response starts at `100` and receives deterministic penalties for:

- role drift: answering the speaker instead of cleaning text;
- semantic addition: solving or completing the dictated text;
- address shift: changing informal `ты` to formal `вы`;
- unexpected script artifacts, such as CJK characters;
- meta output, labels, markdown, or `<think>` leakage;
- excessive length growth;
- case-specific forbidden phrases matched by whole tokens, not substrings;
- optional sentence boundaries via `requireSentenceBoundaries`: the first letter must be uppercase and the output must end with `.`, `!`, `?`, or `…`.

Transport/API errors are recorded as failed runs with score `0`.

## Model Maintenance

Set `enabled: false` to keep a model in the config without running it by
default. You can still target it explicitly with `--models`.

Prices are configured as USD per 1M input/output tokens. If a provider returns
usage, the runner uses it. Otherwise it estimates tokens from text length.
