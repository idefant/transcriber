import type { ProviderKind, ProviderOption } from '#/models/Provider';

// Must match ProviderKind::default_base_url() in src-tauri/src/providers.rs
export const providerDefaultBaseUrls: Record<ProviderKind, string | undefined> = {
  custom: undefined,
  groq: 'https://api.groq.com/openai/v1',
  openai: 'https://api.openai.com/v1',
  openrouter: 'https://openrouter.ai/api/v1',
};

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
