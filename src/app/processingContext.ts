import { createContext, useContext } from 'react';

import type { PostProcessConfigInput, ProcessingConfig, SttConfigInput } from '#/models/Processing';

interface ProcessingContextValue {
  config: ProcessingConfig;
  isLoading: boolean;
  updatePostProcessConfig: (input: PostProcessConfigInput) => Promise<void>;
  updateSttConfig: (input: SttConfigInput) => Promise<void>;
}

export const ProcessingContext = createContext<ProcessingContextValue | undefined>(undefined);

export const useProcessing = () => {
  const value = useContext(ProcessingContext);

  if (value === undefined) {
    throw new Error('useProcessing must be used inside ProcessingProvider');
  }

  return value;
};
