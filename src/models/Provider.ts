export type ProviderKind = 'custom' | 'grok' | 'openai' | 'openrouter';

export interface ModelInfo {
  description: string;
  name: string;
}

export interface ProviderConfig {
  id: string;
  keyPreview: string;
  name: string;
  provider: ProviderKind;
}

export interface ProviderOption {
  label: string;
  value: ProviderKind;
}
