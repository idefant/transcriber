import type { ProviderOption } from '#/models/Provider';

export const providerOptions: ProviderOption[] = [
  {
    label: 'OpenAI',
    value: 'openai',
  },
  {
    label: 'Grok',
    value: 'grok',
  },
  {
    label: 'OpenRouter',
    value: 'openrouter',
  },
  {
    label: 'Custom',
    value: 'custom',
  },
];
