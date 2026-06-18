import { createContext, useContext } from 'react';

import type {
  ModelInfo,
  ProviderConfig,
  ProviderConnectionInput,
  ProviderInput,
  ProviderValidationResult,
} from '#/models/Provider';

interface ProvidersContextValue {
  error?: string;
  isLoading: boolean;
  providers: ProviderConfig[];
  createProvider: (input: ProviderInput) => Promise<ProviderConfig>;
  deleteProvider: (providerId: string) => Promise<void>;
  listProviderModels: (input: ProviderConnectionInput) => Promise<ModelInfo[]>;
  reloadProviders: () => Promise<void>;
  updateProvider: (providerId: string, input: ProviderInput) => Promise<ProviderConfig>;
  validateProviderConfig: (input: ProviderConnectionInput) => Promise<ProviderValidationResult>;
}

export const ProvidersContext = createContext<ProvidersContextValue | undefined>(undefined);

export const useProviders = () => {
  const value = useContext(ProvidersContext);

  if (value === undefined) {
    throw new Error('useProviders must be used inside ProvidersProvider');
  }

  return value;
};
