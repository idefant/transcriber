import { create } from 'zustand';

import * as processingApi from '#/shared/processingApi';

import type {
  DefaultPrompts,
  PostProcessConfig,
  PostProcessConfigInput,
  ProcessingConfig,
  SttConfigInput,
} from '#/models/Processing';

const DEFAULT_CONFIG: ProcessingConfig = {
  postProcess: {
    enabled: false,
    modelKey: null,
    openrouterProvider: null,
    providerId: null,
    systemPrompt: null,
    useCustomPrompts: false,
    userPromptTemplate: null,
  } satisfies PostProcessConfig,
  stt: {
    language: 'auto',
    modelKey: null,
    providerId: null,
    systemPrompt: null,
    useCustomPrompt: false,
  },
};

interface ProcessingState {
  config: ProcessingConfig;
  defaultPrompts: DefaultPrompts | undefined;
  isLoading: boolean;
  load: () => Promise<void>;
  loadDefaultPrompts: () => Promise<void>;
  updateSttConfig: (input: SttConfigInput) => Promise<void>;
  updatePostProcessConfig: (input: PostProcessConfigInput) => Promise<void>;
}

export const useProcessingStore = create<ProcessingState>((set) => ({
  config: DEFAULT_CONFIG,
  defaultPrompts: undefined,
  isLoading: true,

  load: async () => {
    set({ isLoading: true });
    try {
      const config = await processingApi.getProcessingConfig();
      set({ config });
    } catch {
      // При ошибке оставляем конфигурацию по умолчанию.
    } finally {
      set({ isLoading: false });
    }
  },

  loadDefaultPrompts: async () => {
    const defaultPrompts = await processingApi.getDefaultPrompts();
    set({ defaultPrompts });
  },

  updateSttConfig: async (input) => {
    const config = await processingApi.updateSttConfig(input);
    set({ config });
  },

  updatePostProcessConfig: async (input) => {
    const config = await processingApi.updatePostProcessConfig(input);
    set({ config });
  },
}));
