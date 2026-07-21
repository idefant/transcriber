export interface SttConfig {
  language: string;
  modelKey: string | null;
  providerId: string | null;
  systemPrompt: string | null;
  useCustomPrompt: boolean;
}

export interface PostProcessConfig {
  enabled: boolean;
  modelKey: string | null;
  openrouterAllowFallbacks: boolean;
  openrouterProvider: string | null;
  priorityProcessing: boolean;
  providerId: string | null;
  systemPrompt: string | null;
  useCustomPrompts: boolean;
  userPromptTemplate: string | null;
}

export interface ProcessingConfig {
  postProcess: PostProcessConfig;
  stt: SttConfig;
}

export interface SttConfigInput extends Partial<Omit<SttConfig, 'systemPrompt'>> {
  systemPrompt?: string | null;
}

export interface PostProcessConfigInput extends Partial<
  Omit<PostProcessConfig, 'systemPrompt' | 'userPromptTemplate'>
> {
  systemPrompt?: string | null;
  userPromptTemplate?: string | null;
}

export interface DefaultPrompts {
  postProcessSystem: string;
  postProcessUserTemplate: string;
  sttSystem: string;
}
