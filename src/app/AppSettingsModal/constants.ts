import type { ProviderOption } from '#/models/Provider';

export const providerOptions: ProviderOption[] = [
  {
    label: 'OpenAI',
    value: 'openai',
  },
  {
    label: 'Groq',
    value: 'groq',
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
