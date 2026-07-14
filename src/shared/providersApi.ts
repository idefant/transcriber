import { invoke } from '@tauri-apps/api/core';

import type {
  ModelInfo,
  OpenRouterProviderOption,
  ProviderConfig,
  ProviderConnectionInput,
  ProviderInput,
  ProviderValidationResult,
} from '#/models/Provider';

export const getProviders = () => invoke<ProviderConfig[]>('get_providers');

export const createProvider = (input: ProviderInput) =>
  invoke<ProviderConfig>('create_provider', { input });

export const updateProvider = (providerId: string, input: ProviderInput) =>
  invoke<ProviderConfig>('update_provider', { input, providerId });

export const deleteProvider = async (providerId: string): Promise<void> => {
  await invoke('delete_provider', { providerId });
};

export const validateProviderConfig = (input: ProviderConnectionInput) =>
  invoke<ProviderValidationResult>('validate_provider_config', { input });

export const listProviderModels = (input: ProviderConnectionInput) =>
  invoke<ModelInfo[]>('list_provider_models', { input });

export const listOpenRouterModelProviders = (providerId: string, apiModelId: string) =>
  invoke<OpenRouterProviderOption[]>('list_openrouter_model_providers', {
    providerId,
    apiModelId,
  });
