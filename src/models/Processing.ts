export interface SttConfig {
  language: string;
  modelKey: string | null;
  providerId: string | null;
  systemPrompt: string;
  useCustomPrompt: boolean;
}

export interface PostProcessConfig {
  enabled: boolean;
  modelKey: string | null;
  providerId: string | null;
  systemPrompt: string;
  useCustomPrompts: boolean;
  userPromptTemplate: string;
}

export interface ProcessingConfig {
  postProcess: PostProcessConfig;
  stt: SttConfig;
}

export type SttConfigInput = Partial<SttConfig>;

export type PostProcessConfigInput = Partial<PostProcessConfig>;

export interface DefaultPrompts {
  postProcessSystem: string;
  postProcessUserTemplate: string;
  sttSystem: string;
}
