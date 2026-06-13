export type ProviderKind = 'custom' | 'grok' | 'openai' | 'openrouter';
export type SettingsSectionKey =
  | 'general'
  | 'hotkeys'
  | 'providers'
  | 'speechToText'
  | 'postProcessing';
export type TriggerMode = 'hold' | 'press';
export type UiLanguage = 'en' | 'ru';

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
