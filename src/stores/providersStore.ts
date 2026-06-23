import { create } from 'zustand';

import * as providersApi from '#/shared/providersApi';

import type {
  ModelInfo,
  ProviderConfig,
  ProviderConnectionInput,
  ProviderInput,
  ProviderValidationResult,
} from '#/models/Provider';

interface ProvidersState {
  providers: ProviderConfig[];
  isLoading: boolean;
  error?: string;
  load: () => Promise<void>;
  createProvider: (input: ProviderInput) => Promise<ProviderConfig>;
  updateProvider: (providerId: string, input: ProviderInput) => Promise<ProviderConfig>;
  deleteProvider: (providerId: string) => Promise<void>;
  validateProviderConfig: (input: ProviderConnectionInput) => Promise<ProviderValidationResult>;
  listProviderModels: (input: ProviderConnectionInput) => Promise<ModelInfo[]>;
}

export const useProvidersStore = create<ProvidersState>((set) => ({
  providers: [],
  isLoading: true,
  error: undefined,

  load: async () => {
    set({ isLoading: true, error: undefined });
    try {
      const providers = await providersApi.getProviders();
      set({ providers });
    } catch (error) {
      set({ error: error instanceof Error ? error.message : String(error) });
      throw error;
    } finally {
      set({ isLoading: false });
    }
  },

  createProvider: async (input) => {
    const provider = await providersApi.createProvider(input);
    set((s) => ({ providers: [...s.providers, provider] }));
    return provider;
  },

  updateProvider: async (providerId, input) => {
    const provider = await providersApi.updateProvider(providerId, input);
    set((s) => ({
      providers: s.providers.map((p) => (p.id === provider.id ? provider : p)),
    }));
    return provider;
  },

  deleteProvider: async (providerId) => {
    await providersApi.deleteProvider(providerId);
    set((s) => ({ providers: s.providers.filter((p) => p.id !== providerId) }));
  },

  validateProviderConfig: (input) => providersApi.validateProviderConfig(input),

  listProviderModels: (input) => providersApi.listProviderModels(input),
}));
