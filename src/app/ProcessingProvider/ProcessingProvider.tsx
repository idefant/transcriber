import { type FC, type ReactNode, useCallback, useEffect, useMemo, useState } from 'react';

import { ProcessingContext } from '#/app/processingContext';
import * as processingApi from '#/shared/processingApi';

import type {
  PostProcessConfig,
  PostProcessConfigInput,
  ProcessingConfig,
  SttConfigInput,
} from '#/models/Processing';

const DEFAULT_CONFIG: ProcessingConfig = {
  postProcess: {
    enabled: false,
    modelKey: null,
    providerId: null,
    systemPrompt: '',
    useCustomPrompts: false,
    userPromptTemplate: '',
  } satisfies PostProcessConfig,
  stt: {
    language: 'auto',
    modelKey: null,
    providerId: null,
    systemPrompt: '',
    useCustomPrompt: false,
  },
};

interface ProcessingProviderProps {
  children: ReactNode;
}

const ProcessingProvider: FC<ProcessingProviderProps> = ({ children }) => {
  const [config, setConfig] = useState<ProcessingConfig>(DEFAULT_CONFIG);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    const loadConfig = async () => {
      try {
        const data = await processingApi.getProcessingConfig();

        setConfig(data);
      } catch {
        // Keep default config on error.
      } finally {
        setIsLoading(false);
      }
    };

    queueMicrotask(() => {
      void loadConfig().catch(() => {
        // loadConfig handles all errors internally.
      });
    });
  }, []);

  const updateSttConfig = useCallback(async (input: SttConfigInput) => {
    const next = await processingApi.updateSttConfig(input);

    setConfig(next);
  }, []);

  const updatePostProcessConfig = useCallback(async (input: PostProcessConfigInput) => {
    const next = await processingApi.updatePostProcessConfig(input);

    setConfig(next);
  }, []);

  const contextValue = useMemo(
    () => ({
      config,
      isLoading,
      updatePostProcessConfig,
      updateSttConfig,
    }),
    [config, isLoading, updatePostProcessConfig, updateSttConfig],
  );

  return <ProcessingContext.Provider value={contextValue}>{children}</ProcessingContext.Provider>;
};

export default ProcessingProvider;
