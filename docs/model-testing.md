# Model Testing

The post-processing model test runner is a standalone Node tool. It does not
change application settings and does not require the Tauri UI.

## Configuration

Files live in `scripts/model-testing/`:

- `models.json` - model list, provider URL, API key env var, enabled flag, request params, and prices per 1M tokens.
- `cases.json` - input prompts and deterministic scoring rules.
- `provider-rules.json` - per-provider concurrency and cooldown rules.
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

## Provider Scheduling

Requests are scheduled by the `provider` field from `models.json`. Each provider
has an independent queue, so OpenAI requests do not block OpenRouter requests,
and OpenRouter requests do not block Groq requests.

`provider-rules.json` controls each queue:

```json
{
  "default": {
    "concurrency": 1,
    "delayAfterMs": 0
  },
  "groq": {
    "concurrency": 1,
    "delayAfterMs": 1000
  },
  "openrouter": {
    "concurrency": 5,
    "delayAfterMs": 0
  },
  "openai": {
    "concurrency": 5,
    "delayAfterMs": 0
  }
}
```

`concurrency` is the maximum number of active requests for that provider.
`delayAfterMs` is the pause after a completed request before the same provider
queue starts another request. Models without an explicit provider rule use
`default`.

The rule is matched by `model.provider`, not by `providerRouting`. For example,
an OpenRouter model that routes to Groq still uses the `openrouter` queue because
the HTTP request is sent to OpenRouter.

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
- average score, errors, latency, and average estimated cost;
- per-model details for every case, language, and repeat in a modal opened from the model name;
- penalties that formed each response score;
- a per-model toggle that shows only invalid tasks and invalid runs inside them.

Latency and price are shown for humans only. They do not affect the ranking.

## Scoring

Every response starts at `100` and receives deterministic penalties for:

- role drift: answering the speaker instead of cleaning text;
- semantic addition: solving or completing the dictated text;
- address shift: changing informal `ты` to formal `вы`;
- unexpected script artifacts, such as CJK characters;
- optional language preservation via `preserveLanguage`: the output should keep the dominant script of the input, using a 65% script ratio threshold when both sides have at least four letters;
- meta output, labels, markdown, or `<think>` leakage;
- excessive length growth via optional `lengthRatio.max` (`1.5` by default);
- excessive length drop via optional `lengthRatio.min`;
- case-specific `textCondition` checks with `contains`, `notContains`, `op: "and"`, and `op: "or"`;
- optional minimum punctuation via `minPunctuationMarks`, counting `.`, `,`, `!`, `?`, `;`, `:`, and `…`; useful for multi-clause cases and not needed for math-operation cases;
- optional sentence boundaries via `requireSentenceBoundaries`: the first letter must be uppercase and the output must end with `.`, `!`, `?`, or `…`.

`textCondition` supports `mode: "sequence"` by default and `mode: "word"` for
whole-word token sequence matching. `caseSensitive` is `false` by default.
Quote variants such as `"..."` and `«...»` are normalized for these checks.
The score penalty is binary, but the report detail includes only failed
condition branches.

Transport/API errors are recorded as failed runs with score `0`.

## Model Maintenance

Set `enabled: false` to keep a model in the config without running it by
default. You can still target it explicitly with `--models`.

Prices are configured as USD per 1M input/output tokens. If a provider returns
usage, the runner uses it. Otherwise it estimates tokens from text length.
