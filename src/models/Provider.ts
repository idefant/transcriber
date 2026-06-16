export type ProviderKind = 'custom' | 'groq' | 'openai' | 'openrouter';

export interface ModelInfo {
  description: string;
  name: string;
}

export interface ProviderConfig {
  baseUrl?: string;
  createdAt: string;
  favoriteModels: string[];
  hasApiKey: boolean;
  headers?: string;
  id: string;
  keyPreview: string;
  name: string;
  provider: ProviderKind;
  updatedAt: string;
  useAdvancedSettings: boolean;
}

export interface ProviderConnectionInput {
  apiKey?: string;
  baseUrl?: string;
  headers?: string;
  provider: ProviderKind;
  providerId?: string;
  useAdvancedSettings?: boolean;
}

export interface ProviderInput {
  apiKey?: string;
  baseUrl?: string;
  favoriteModels?: string[];
  headers?: string;
  name?: string;
  provider: ProviderKind;
  useAdvancedSettings?: boolean;
}

export interface ProviderOption {
  label: string;
  value: ProviderKind;
}

export interface ProviderValidationResult {
  message: string;
  ok: boolean;
}
