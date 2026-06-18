import { invoke } from '@tauri-apps/api/core';

import type {
  DefaultPrompts,
  PostProcessConfigInput,
  ProcessingConfig,
  SttConfigInput,
} from '#/models/Processing';

export const getProcessingConfig = () => invoke<ProcessingConfig>('get_processing_config');

export const getDefaultPrompts = () => invoke<DefaultPrompts>('get_default_prompts');

export const updateSttConfig = (input: SttConfigInput) =>
  invoke<ProcessingConfig>('update_stt_config', { input });

export const updatePostProcessConfig = (input: PostProcessConfigInput) =>
  invoke<ProcessingConfig>('update_post_process_config', { input });

export interface SttTestInput {
  audio: number[];
  fileName: string;
}

export const runSttTest = (input: SttTestInput) =>
  invoke<string>('run_stt_test', {
    audio: input.audio,
    fileName: input.fileName,
  });

export interface PostProcessTestInput {
  text: string;
}

export const runPostProcessTest = (input: PostProcessTestInput) =>
  invoke<string>('run_post_process_test', {
    text: input.text,
  });
