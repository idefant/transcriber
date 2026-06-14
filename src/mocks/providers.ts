import type { ModelInfo, ProviderConfig, ProviderKind } from '#/models/Provider';

export const providerModels: Record<ProviderKind, ModelInfo[]> = {
  custom: [
    {
      description: 'Модель будет загружена из пользовательского OpenAI-compatible endpoint.',
      name: 'custom-model',
    },
  ],
  grok: [
    {
      description: 'Быстрая модель для черновой транскрибации и коротких аудио.',
      name: 'grok-stt-beta',
    },
    {
      description: 'Модель для более точного распознавания речи в длинных записях.',
      name: 'grok-stt-large',
    },
  ],
  openai: [
    {
      description: 'Универсальная модель распознавания речи.',
      name: 'gpt-4o-transcribe',
    },
    {
      description: 'Лёгкая модель для быстрых транскрибаций.',
      name: 'gpt-4o-mini-transcribe',
    },
  ],
  openrouter: [
    {
      description: 'Маршрутизируемая модель распознавания речи через OpenRouter.',
      name: 'openrouter/auto-stt',
    },
    {
      description: 'Резервная модель для аудио с шумом.',
      name: 'openrouter/stt-balanced',
    },
  ],
};

export const defaultProviders: ProviderConfig[] = [
  {
    id: 'openai-default',
    keyPreview: 'sk-...42f9',
    name: 'OpenAI Gateway',
    provider: 'openai',
  },
];
