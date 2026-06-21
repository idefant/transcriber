import { existsSync } from 'node:fs';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import path from 'node:path';
import { performance } from 'node:perf_hooks';
import { fileURLToPath } from 'node:url';

import { generateReport } from './report.mjs';

const currentFilePath = fileURLToPath(import.meta.url);
const currentDirectory = path.dirname(currentFilePath);
const rootDirectory = path.resolve(currentDirectory, '..', '..');
const defaultRepeats = 5;
const defaultLanguages = ['ru', 'en'];
const languageScriptMinLetters = 4;
const languageScriptThreshold = 0.65;
const promptsFilePath = path.join(rootDirectory, 'resources', 'promps.json');
const providerRulesFilePath = path.join(currentDirectory, 'provider-rules.json');

const penaltyCatalog = {
  addressShift: {
    label: 'Informal address changed to formal address',
    points: 25,
  },
  emptyOutput: {
    label: 'Unexpected empty output',
    points: 40,
  },
  exactPunctuationMarks: {
    label: 'Output has unexpected punctuation mark count',
    points: 10,
  },
  initialCapital: {
    label: 'Output should start with a capital letter',
    points: 10,
  },
  lengthDrift: {
    label: 'Output is too long compared with input',
    points: 20,
  },
  lengthDrop: {
    label: 'Output is too short compared with input',
    points: 25,
  },
  languageShift: {
    label: 'Output appears translated to another language',
    points: 20,
  },
  metaOutput: {
    label: 'Meta output, labels, markdown, or thinking leaked',
    points: 25,
  },
  minPunctuationMarks: {
    label: 'Output has too few punctuation marks',
    points: 10,
  },
  textConditionMismatch: {
    label: 'Case-specific text condition failed',
    points: 25,
  },
  roleDrift: {
    label: 'Role drift / model answered instead of cleaning text',
    points: 35,
  },
  sentenceBoundaries: {
    label: 'Sentence should start with a capital letter and end with sentence punctuation',
    points: 10,
  },
  semanticAddition: {
    label: 'Semantic addition or task solving',
    points: 30,
  },
  wrongScript: {
    label: 'Unexpected script or language artifact',
    points: 25,
  },
};

const roleDriftPatterns = [
  /\bI\s+(?:can|will|would|am able to|understand)\b/i,
  /\bYes\b/i,
  /\bSure\b/i,
  /\bHere(?:'s| is)\b/i,
  /\bI'?ll\b/i,
  /\bLet me\b/i,
  /^(?:да|конечно|хорошо|разумеется)[,!. ]/i,
  /(?:я\s+могу|я\s+умею|я\s+буду|я\s+сделаю|я\s+создам|справлюсь)/i,
];

const metaOutputPatterns = [
  /<think[\s>]/i,
  /<\/think>/i,
  /```/,
  /^\s*[-*]\s+/m,
  /вот\s+(?:исправленный|очищенный|отредактированный)\s+текст/i,
  /исправленный\s+текст\s*:/i,
  /очищенный\s+текст\s*:/i,
  /cleaned\s+text\s*:/i,
  /corrected\s+text\s*:/i,
];

const unexpectedScriptPattern = /[\u4E00-\u9FFF\u3040-\u30FF\uAC00-\uD7AF]/u;
const informalAddressPattern =
  /(?<![\p{L}\p{N}_])(?:ты|тебя|тебе|сделай|справишься)(?![\p{L}\p{N}_])/iu;
const formalAddressPattern =
  /(?<![\p{L}\p{N}_])(?:вы|вам|вас|ваш|сделайте|справитесь)(?![\p{L}\p{N}_])/iu;
const symbolicMathPromptPattern = /^2\s*\+\s*2\s*=$/;
const mathPromptPattern = /^(?:2\s*\+\s*2\s*=|два\s+плюс\s+два\s+равно)$/i;
const mathAnswerPattern = /\b(?:4|четыре|four)\b/i;

const readJson = async (filePath) => JSON.parse(await readFile(filePath, 'utf8'));

const wait = (durationMs) =>
  new Promise((resolve) => {
    setTimeout(resolve, durationMs);
  });

const assertNonNegativeInteger = (value, label) => {
  if (!Number.isInteger(value) || value < 0) {
    throw new Error(`${label} must be a non-negative integer.`);
  }
};

const normalizeProviderRule = (provider, rule) => {
  if (!rule || typeof rule !== 'object' || Array.isArray(rule)) {
    throw new Error(`Provider rule "${provider}" must be an object.`);
  }

  const concurrency = rule.concurrency ?? 1;
  const delayAfterMs = rule.delayAfterMs ?? 0;

  if (!Number.isInteger(concurrency) || concurrency < 1) {
    throw new Error(`Provider rule "${provider}".concurrency must be a positive integer.`);
  }

  assertNonNegativeInteger(delayAfterMs, `Provider rule "${provider}".delayAfterMs`);

  return {
    concurrency,
    delayAfterMs,
  };
};

const getEffectiveProviderRules = (providerRules, models) => {
  const defaultRule = normalizeProviderRule('default', providerRules.default ?? {});
  const effectiveRules = {
    default: defaultRule,
  };

  for (const model of models) {
    effectiveRules[model.provider] = normalizeProviderRule(
      model.provider,
      providerRules[model.provider] ?? defaultRule,
    );
  }

  return effectiveRules;
};

const parseEnvironmentValue = (value) => {
  const trimmedValue = value.trim();

  if (
    (trimmedValue.startsWith('"') && trimmedValue.endsWith('"')) ||
    (trimmedValue.startsWith("'") && trimmedValue.endsWith("'"))
  ) {
    return trimmedValue.slice(1, -1);
  }

  return trimmedValue;
};

const loadEnvironmentFile = async (filePath) => {
  if (!existsSync(filePath)) return;

  const text = await readFile(filePath, 'utf8');

  for (const line of text.split(/\r?\n/)) {
    const trimmedLine = line.trim();

    if (!trimmedLine || trimmedLine.startsWith('#')) continue;

    const separatorIndex = trimmedLine.indexOf('=');

    if (separatorIndex === -1) continue;

    const key = trimmedLine.slice(0, separatorIndex).trim();
    const value = parseEnvironmentValue(trimmedLine.slice(separatorIndex + 1));

    if (key && process.env[key] === undefined) {
      process.env[key] = value;
    }
  }
};

const loadEnvironmentFiles = async () => {
  await loadEnvironmentFile(path.join(rootDirectory, '.env'));
  await loadEnvironmentFile(path.join(rootDirectory, '.env.local'));
};

const parseList = (value) =>
  value
    .split(',')
    .map((item) => item.trim())
    .filter((item) => item.length > 0);

const parseArguments = (arguments_) => {
  const options = {
    cases: [],
    languages: defaultLanguages,
    models: [],
    output: undefined,
    prompt: undefined,
    repeats: defaultRepeats,
    smoke: false,
  };

  for (let index = 0; index < arguments_.length; index += 1) {
    const argument = arguments_[index];
    const next = arguments_[index + 1];

    switch (argument) {
      case '--cases': {
        options.cases = parseList(next ?? '');
        index += 1;
        break;
      }

      case '--languages': {
        options.languages = parseList(next ?? '');
        index += 1;
        break;
      }

      case '--models': {
        options.models = parseList(next ?? '');
        index += 1;
        break;
      }

      case '--output': {
        options.output = next;
        index += 1;
        break;
      }

      case '--prompt': {
        options.prompt = next ?? '';
        index += 1;
        break;
      }

      case '--repeats': {
        options.repeats = Number.parseInt(next ?? '', 10);
        index += 1;
        break;
      }

      case '--smoke': {
        options.smoke = true;
        break;
      }

      default: {
        throw new Error(`Unknown option: ${argument}`);
      }
    }
  }

  if (!Number.isInteger(options.repeats) || options.repeats < 1) {
    throw new Error('--repeats must be a positive integer.');
  }

  if (options.languages.length === 0) {
    throw new Error('--languages must include at least one language.');
  }

  return options;
};

const selectModels = (models, options) => {
  const selected =
    options.models.length > 0
      ? models.filter(
          (model) =>
            options.models.includes(model.key) ||
            options.models.includes(model.apiId) ||
            options.models.includes(model.label),
        )
      : models.filter((model) => model.enabled);

  if (selected.length === 0) {
    throw new Error('No models selected. Check --models or enabled flags in models.json.');
  }

  return selected;
};

const selectCases = (cases, options) => {
  if (options.prompt) {
    return [
      {
        input: options.prompt,
        key: 'smoke-prompt',
        lengthRatio: {
          max: 1.5,
        },
        preserveInformalYou: informalAddressPattern.test(options.prompt),
        preserveLanguage: !symbolicMathPromptPattern.test(normalizeForComparison(options.prompt)),
        requireSentenceBoundaries: !symbolicMathPromptPattern.test(
          normalizeForComparison(options.prompt),
        ),
      },
    ];
  }

  const selected =
    options.cases.length > 0
      ? cases.filter((testCase) => options.cases.includes(testCase.key))
      : cases;

  if (selected.length === 0) {
    throw new Error('No cases selected. Check --cases or cases.json.');
  }

  return selected;
};

const applyTemplate = (template, variables) => {
  let result = template;

  for (const [key, value] of Object.entries(variables)) {
    result = result.replaceAll(`{{${key}}}`, value);
  }

  return result;
};

const applyThinkingMode = (prompt, model) => {
  if (!model.params?.disableThinking) return prompt;

  return prompt.includes('/no_think') ? prompt : `${prompt}\n\n/no_think`;
};

const normalizeForComparison = (value) =>
  value
    .trim()
    .toLowerCase()
    .replaceAll('ё', 'е')
    .replaceAll(/[.,!?;:"'`()[\]{}<>]/g, '')
    .replaceAll(/\s+/g, ' ');

const normalizeForTextCondition = (value, { caseSensitive = false } = {}) => {
  const normalizedValue = String(value)
    .trim()
    .replaceAll(/[«»„“”]/g, '"')
    .replaceAll(/\s+([.,!?;:])/g, '$1')
    .replaceAll(/\s+/g, ' ');

  if (caseSensitive) {
    return normalizedValue.replaceAll('ё', 'е').replaceAll('Ё', 'Е');
  }

  return normalizedValue.toLocaleLowerCase().replaceAll('ё', 'е');
};

const phraseMatchesTokens = (textTokens, phraseTokens) => {
  if (phraseTokens.length === 0 || textTokens.length < phraseTokens.length) {
    return false;
  }

  for (let index = 0; index <= textTokens.length - phraseTokens.length; index += 1) {
    const matches = phraseTokens.every(
      (phraseToken, phraseIndex) => textTokens[index + phraseIndex] === phraseToken,
    );

    if (matches) return true;
  }

  return false;
};

const startsWithCapitalLetter = (value) => {
  const firstLetter = value.match(/\p{L}/u)?.[0];

  if (!firstLetter) return true;

  return (
    firstLetter === firstLetter.toLocaleUpperCase() &&
    firstLetter !== firstLetter.toLocaleLowerCase()
  );
};

const endsWithSentencePunctuation = (value) => /[.!?…]\s*$/u.test(value);

const countPunctuationMarks = (value) => (value.match(/[.,!?;:…]/gu) ?? []).length;

const tokenizeTextConditionValue = (value, options) =>
  normalizeForTextCondition(value, options).match(/[\p{L}\p{N}_]+/gu) ?? [];

const conditionContains = (text, needle, options) => {
  const mode = options.mode ?? 'sequence';

  if (mode === 'word') {
    return phraseMatchesTokens(
      tokenizeTextConditionValue(text, options),
      tokenizeTextConditionValue(needle, options),
    );
  }

  return normalizeForTextCondition(text, options).includes(
    normalizeForTextCondition(needle, options),
  );
};

const evaluateTextCondition = (condition, text) => {
  if (!condition) {
    return {
      passed: true,
    };
  }

  if (condition.op === 'and') {
    const failedArguments = condition.args
      .map((argument) => evaluateTextCondition(argument, text))
      .filter((result) => !result.passed)
      .map((result) => result.failedCondition);

    return failedArguments.length === 0
      ? {
          passed: true,
        }
      : {
          failedCondition: {
            op: 'and',
            args: failedArguments,
          },
          passed: false,
        };
  }

  if (condition.op === 'or') {
    const results = condition.args.map((argument) => evaluateTextCondition(argument, text));

    return results.some((result) => result.passed)
      ? {
          passed: true,
        }
      : {
          failedCondition: {
            op: 'or',
            args: results.map((result) => result.failedCondition),
          },
          passed: false,
        };
  }

  if (typeof condition.contains === 'string') {
    const passed = conditionContains(text, condition.contains, condition);

    return passed
      ? {
          passed,
        }
      : {
          failedCondition: condition,
          passed,
        };
  }

  if (typeof condition.notContains === 'string') {
    const passed = !conditionContains(text, condition.notContains, condition);

    return passed
      ? {
          passed,
        }
      : {
          failedCondition: condition,
          passed,
        };
  }

  return {
    failedCondition: condition,
    passed: false,
  };
};

const getScriptStats = (value) => {
  const letters = value.match(/\p{L}/gu) ?? [];

  return {
    cyrillic: (value.match(/\p{Script=Cyrillic}/gu) ?? []).length,
    latin: (value.match(/\p{Script=Latin}/gu) ?? []).length,
    totalLetters: letters.length,
  };
};

const getDominantScript = (value) => {
  const stats = getScriptStats(value);

  if (stats.totalLetters < languageScriptMinLetters) return;

  for (const script of ['cyrillic', 'latin']) {
    const ratio = stats[script] / stats.totalLetters;

    if (ratio >= languageScriptThreshold) {
      return {
        ratio,
        script,
        stats,
      };
    }
  }

  return;
};

const getScriptRatio = (stats, script) =>
  stats.totalLetters > 0 ? stats[script] / stats.totalLetters : 0;

const addPenalty = (penalties, key, detail) => {
  const penalty = penaltyCatalog[key];

  penalties.push({
    detail,
    key,
    label: penalty.label,
    points: penalty.points,
  });
};

const scoreOutput = ({ output, testCase }) => {
  const penalties = [];
  const normalizedOutput = output.trim();
  const normalizedInput = normalizeForComparison(testCase.input);

  if (normalizedOutput.length === 0 && testCase.input.trim().length > 0) {
    addPenalty(penalties, 'emptyOutput');
  }

  if (roleDriftPatterns.some((pattern) => pattern.test(normalizedOutput))) {
    addPenalty(penalties, 'roleDrift');
  }

  if (metaOutputPatterns.some((pattern) => pattern.test(normalizedOutput))) {
    addPenalty(penalties, 'metaOutput');
  }

  if (unexpectedScriptPattern.test(normalizedOutput)) {
    addPenalty(penalties, 'wrongScript');
  }

  if (testCase.preserveLanguage) {
    const inputScript = getDominantScript(testCase.input);
    const outputStats = getScriptStats(normalizedOutput);

    if (inputScript && outputStats.totalLetters >= languageScriptMinLetters) {
      const outputRatio = getScriptRatio(outputStats, inputScript.script);

      if (outputRatio < languageScriptThreshold) {
        addPenalty(
          penalties,
          'languageShift',
          `${inputScript.script} ratio ${outputRatio.toFixed(2)}/${languageScriptThreshold}`,
        );
      }
    }
  }

  if (
    testCase.requireSentenceBoundaries &&
    (!startsWithCapitalLetter(normalizedOutput) || !endsWithSentencePunctuation(normalizedOutput))
  ) {
    addPenalty(penalties, 'sentenceBoundaries');
  }

  if (testCase.requireInitialCapital && !startsWithCapitalLetter(normalizedOutput)) {
    addPenalty(penalties, 'initialCapital');
  }

  if (
    Number.isInteger(testCase.minPunctuationMarks) &&
    countPunctuationMarks(normalizedOutput) < testCase.minPunctuationMarks
  ) {
    addPenalty(
      penalties,
      'minPunctuationMarks',
      `${countPunctuationMarks(normalizedOutput)}/${testCase.minPunctuationMarks}`,
    );
  }

  if (
    Number.isInteger(testCase.exactPunctuationMarks) &&
    countPunctuationMarks(normalizedOutput) !== testCase.exactPunctuationMarks
  ) {
    addPenalty(
      penalties,
      'exactPunctuationMarks',
      `${countPunctuationMarks(normalizedOutput)}/${testCase.exactPunctuationMarks}`,
    );
  }

  if (testCase.preserveInformalYou && formalAddressPattern.test(normalizedOutput)) {
    addPenalty(penalties, 'addressShift');
  }

  if (testCase.textCondition) {
    const textConditionResult = evaluateTextCondition(testCase.textCondition, normalizedOutput);

    if (!textConditionResult.passed) {
      addPenalty(
        penalties,
        'textConditionMismatch',
        JSON.stringify(textConditionResult.failedCondition),
      );
    }
  }

  const inputLength = Math.max(testCase.input.trim().length, 1);
  const maxRatio = testCase.lengthRatio?.max ?? 1.5;
  const minRatio = testCase.lengthRatio?.min;

  if (normalizedOutput.length > inputLength * maxRatio) {
    addPenalty(penalties, 'lengthDrift', `ratio ${normalizedOutput.length / inputLength}`);
  }

  if (Number.isFinite(minRatio) && normalizedOutput.length < inputLength * minRatio) {
    addPenalty(penalties, 'lengthDrop', `ratio ${normalizedOutput.length / inputLength}`);
  }

  if (mathPromptPattern.test(normalizedInput) && mathAnswerPattern.test(normalizedOutput)) {
    addPenalty(penalties, 'semanticAddition', 'math answer was added');
  }

  const penaltyPoints = penalties.reduce((sum, penalty) => sum + penalty.points, 0);

  return {
    penalties,
    responseScore: Math.max(0, 100 - penaltyPoints),
  };
};

const estimateTokens = (text) => Math.ceil(String(text ?? '').length / 4);

const getUsage = (responseBody, requestText, outputText) => {
  const usage = responseBody.usage ?? {};

  return {
    completionTokens: usage.completion_tokens ?? estimateTokens(outputText),
    promptTokens: usage.prompt_tokens ?? estimateTokens(requestText),
    totalTokens: usage.total_tokens ?? estimateTokens(requestText) + estimateTokens(outputText),
  };
};

const estimateCost = (model, usage) => {
  const inputPrice = model.inputPricePer1M ?? 0;
  const outputPrice = model.outputPricePer1M ?? 0;

  return (
    (usage.promptTokens / 1_000_000) * inputPrice +
    (usage.completionTokens / 1_000_000) * outputPrice
  );
};

const classifyHttpError = (status, message) => {
  if (status === 400 || status === 404 || /model/i.test(message)) {
    return 'configuration';
  }

  if (status === 401 || status === 403) {
    return 'authentication';
  }

  if (status === 429) {
    return 'rate-limit';
  }

  return 'api';
};

const requestModel = async ({ language, model, prompts, testCase }) => {
  const apiKey = process.env[model.apiKeyEnv];

  if (!apiKey) {
    throw Object.assign(new Error(`Missing API key env var: ${model.apiKeyEnv}`), {
      kind: 'configuration',
    });
  }

  const baseSystemPrompt = prompts.postProcess.system[language];

  if (!baseSystemPrompt) {
    throw Object.assign(new Error(`Unsupported language: ${language}`), {
      kind: 'configuration',
    });
  }

  const systemPrompt = applyThinkingMode(baseSystemPrompt, model);

  const userContent = applyTemplate(prompts.postProcess.userTemplate, {
    TRANSCRIBED_TEXT: testCase.input,
  });
  const body = {
    max_completion_tokens: model.params?.maxCompletionTokens ?? 1024,
    messages: [
      {
        content: systemPrompt,
        role: 'system',
      },
      {
        content: userContent,
        role: 'user',
      },
    ],
    model: model.apiId,
    temperature: model.params?.temperature ?? 0.2,
  };

  if (model.params?.thinking) {
    body.thinking = model.params.thinking;
  }

  if (model.params?.reasoning) {
    body.reasoning = model.params.reasoning;
  }

  if (model.params?.reasoningEffort) {
    body.reasoning_effort = model.params.reasoningEffort;
  }

  if (model.params?.reasoningFormat) {
    body.reasoning_format = model.params.reasoningFormat;
  }

  if (model.params?.includeReasoning !== undefined) {
    body.include_reasoning = model.params.includeReasoning;
  }

  if (model.providerRouting) {
    body.provider = model.providerRouting;
  }

  const requestText = JSON.stringify(body.messages);
  const response = await fetch(`${model.baseUrl.replace(/\/$/, '')}/chat/completions`, {
    body: JSON.stringify(body),
    headers: {
      Authorization: `Bearer ${apiKey}`,
      'Content-Type': 'application/json',
      'HTTP-Referer': 'https://localhost/transcriber-model-testing',
      'X-Title': 'Transcriber Model Testing',
    },
    method: 'POST',
  });
  const responseText = await response.text();
  let responseBody;

  try {
    responseBody = responseText ? JSON.parse(responseText) : {};
  } catch {
    responseBody = {
      raw: responseText,
    };
  }

  if (!response.ok) {
    const message = responseBody.error?.message ?? responseText ?? `HTTP ${response.status}`;

    throw Object.assign(new Error(message), {
      kind: classifyHttpError(response.status, message),
      status: response.status,
    });
  }

  const output = responseBody.choices?.[0]?.message?.content;

  if (typeof output !== 'string') {
    throw Object.assign(new Error('Provider returned an empty or unsupported response.'), {
      kind: 'api',
    });
  }

  const usage = getUsage(responseBody, requestText, output);

  return {
    estimatedCostUsd: estimateCost(model, usage),
    output,
    usage,
  };
};

const runOne = async ({ language, model, prompts, repeatIndex, testCase }) => {
  const startedAt = new Date().toISOString();
  const started = performance.now();

  try {
    const response = await requestModel({
      language,
      model,
      prompts,
      testCase,
    });
    const elapsedMs = performance.now() - started;
    const score = scoreOutput({
      output: response.output,
      testCase,
    });

    return {
      caseKey: testCase.key,
      elapsedMs,
      error: undefined,
      estimatedCostUsd: response.estimatedCostUsd,
      input: testCase.input,
      language,
      modelKey: model.key,
      output: response.output,
      penalties: score.penalties,
      repeatIndex,
      responseScore: score.responseScore,
      startedAt,
      status: 'completed',
      usage: response.usage,
    };
  } catch (error) {
    return {
      caseKey: testCase.key,
      elapsedMs: performance.now() - started,
      error: {
        kind: error.kind ?? 'transport',
        message: error.message,
        status: error.status,
      },
      estimatedCostUsd: 0,
      input: testCase.input,
      language,
      modelKey: model.key,
      output: '',
      penalties: [
        {
          detail: undefined,
          key: 'requestError',
          label: 'Request failed',
          points: 100,
        },
      ],
      repeatIndex,
      responseScore: 0,
      startedAt,
      status: 'error',
      usage: {
        completionTokens: 0,
        promptTokens: 0,
        totalTokens: 0,
      },
    };
  }
};

const createRunDirectory = async (options) => {
  const timestamp = new Date().toISOString().replaceAll(':', '-').replaceAll('.', '-');
  const outputDirectory = options.output
    ? path.resolve(options.output)
    : path.join(rootDirectory, 'reports', 'model-testing', timestamp);

  await mkdir(outputDirectory, { recursive: true });

  return outputDirectory;
};

const createRunTasks = ({ languages, models, prompts, repeats, testCases }) => {
  const tasks = [];

  for (const model of models) {
    for (const testCase of testCases) {
      for (const language of languages) {
        for (let repeatIndex = 1; repeatIndex <= repeats; repeatIndex += 1) {
          tasks.push({
            index: tasks.length,
            language,
            model,
            prompts,
            repeatIndex,
            testCase,
          });
        }
      }
    }
  }

  return tasks;
};

const groupTasksByProvider = (tasks) => {
  const groups = new Map();

  for (const task of tasks) {
    const providerTasks = groups.get(task.model.provider) ?? [];

    providerTasks.push(task);
    groups.set(task.model.provider, providerTasks);
  }

  return groups;
};

const formatTaskLabel = (task) =>
  `${task.model.key} / ${task.testCase.key} / ${task.language} / ${task.repeatIndex}`;

const runProviderTasks = async ({ provider, providerTasks, results, rule }) => {
  let nextTaskIndex = 0;

  const runWorker = async () => {
    while (nextTaskIndex < providerTasks.length) {
      const task = providerTasks[nextTaskIndex];

      nextTaskIndex += 1;
      console.log(`[start] ${provider} / ${formatTaskLabel(task)}`);
      results[task.index] = await runOne(task);

      const statusLabel = results[task.index].status === 'error' ? 'error' : 'done';

      console.log(`[${statusLabel}] ${provider} / ${formatTaskLabel(task)}`);

      if (rule.delayAfterMs > 0 && nextTaskIndex < providerTasks.length) {
        await wait(rule.delayAfterMs);
      }
    }
  };

  const workerCount = Math.min(rule.concurrency, providerTasks.length);
  const workers = Array.from({ length: workerCount }, () => runWorker());

  await Promise.all(workers);
};

const runTasks = async ({ providerRules, tasks }) => {
  const results = Array.from({ length: tasks.length });
  const tasksByProvider = groupTasksByProvider(tasks);

  await Promise.all(
    [...tasksByProvider.entries()].map(([provider, providerTasks]) =>
      runProviderTasks({
        provider,
        providerTasks,
        results,
        rule: providerRules[provider] ?? providerRules.default,
      }),
    ),
  );

  return results;
};

const run = async () => {
  await loadEnvironmentFiles();

  const options = parseArguments(process.argv.slice(2));
  const allModels = await readJson(path.join(currentDirectory, 'models.json'));
  const allCases = await readJson(path.join(currentDirectory, 'cases.json'));
  const prompts = await readJson(promptsFilePath);
  const rawProviderRules = await readJson(providerRulesFilePath);
  const selectedModels = selectModels(allModels, options);
  const selectedCases = selectCases(allCases, options);
  const providerRules = getEffectiveProviderRules(rawProviderRules, selectedModels);
  const repeats = options.smoke ? Math.min(options.repeats, 1) : options.repeats;
  const outputDirectory = await createRunDirectory(options);
  const startedAt = new Date().toISOString();
  const tasks = createRunTasks({
    languages: options.languages,
    models: selectedModels,
    prompts,
    repeats,
    testCases: selectedCases,
  });

  console.log(
    `Running ${selectedModels.length} models x ${selectedCases.length} cases x ${options.languages.length} languages x ${repeats} repeats`,
  );

  const runs = await runTasks({
    providerRules,
    tasks,
  });

  const results = {
    cases: selectedCases,
    config: {
      languages: options.languages,
      providerRules,
      repeats,
      smoke: options.smoke,
    },
    finishedAt: new Date().toISOString(),
    models: selectedModels,
    runs,
    selectedModelKeys: selectedModels.map((model) => model.key),
    startedAt,
  };
  const resultsFilePath = path.join(outputDirectory, 'results.json');
  const reportFilePath = path.join(outputDirectory, 'report.html');

  await writeFile(resultsFilePath, `${JSON.stringify(results, undefined, 2)}\n`, 'utf8');
  await generateReport(results, reportFilePath);

  console.log(`Results: ${resultsFilePath}`);
  console.log(`Report: ${reportFilePath}`);
};

await run();
