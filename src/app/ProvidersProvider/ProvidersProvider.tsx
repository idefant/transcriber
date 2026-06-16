import { type FC, type ReactNode, useCallback, useEffect, useMemo, useState } from 'react';

import { ProvidersContext } from '#/app/providersContext';
import * as providersApi from '#/shared/providersApi';

import type {
  ModelInfo,
  ProviderConfig,
  ProviderConnectionInput,
  ProviderInput,
  ProviderValidationResult,
} from '#/models/Provider';

interface ProvidersProviderProps {
  children: ReactNode;
}

const ProvidersProvider: FC<ProvidersProviderProps> = ({ children }) => {
  const [providers, setProviders] = useState<ProviderConfig[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string>();

  const reloadProviders = useCallback(async () => {
    setIsLoading(true);
    setError(undefined);

    try {
      const nextProviders = await providersApi.getProviders();

      setProviders(nextProviders);
    } catch (unknownError) {
      const message = unknownError instanceof Error ? unknownError.message : String(unknownError);

      setError(message);
      throw unknownError;
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    queueMicrotask(() => {
      void reloadProviders().catch(() => {
        // The error is already stored in context.
      });
    });
  }, [reloadProviders]);

  const createProvider = useCallback(async (input: ProviderInput) => {
    const provider = await providersApi.createProvider(input);

    setProviders((currentProviders) => [...currentProviders, provider]);

    return provider;
  }, []);

  const updateProvider = useCallback(async (providerId: string, input: ProviderInput) => {
    const provider = await providersApi.updateProvider(providerId, input);

    setProviders((currentProviders) =>
      currentProviders.map((currentProvider) =>
        currentProvider.id === provider.id ? provider : currentProvider,
      ),
    );

    return provider;
  }, []);

  const deleteProvider = useCallback(async (providerId: string) => {
    await providersApi.deleteProvider(providerId);

    setProviders((currentProviders) =>
      currentProviders.filter((provider) => provider.id !== providerId),
    );
  }, []);

  const toggleFavoriteModel = useCallback(async (providerId: string, modelName: string) => {
    const provider = await providersApi.toggleFavoriteModel(providerId, modelName);

    setProviders((currentProviders) =>
      currentProviders.map((currentProvider) =>
        currentProvider.id === provider.id ? provider : currentProvider,
      ),
    );

    return provider;
  }, []);

  const validateProviderConfig = useCallback(
    (input: ProviderConnectionInput): Promise<ProviderValidationResult> =>
      providersApi.validateProviderConfig(input),
    [],
  );

  const listProviderModels = useCallback(
    (input: ProviderConnectionInput): Promise<ModelInfo[]> =>
      providersApi.listProviderModels(input),
    [],
  );

  const contextValue = useMemo(
    () => ({
      createProvider,
      deleteProvider,
      error,
      isLoading,
      listProviderModels,
      providers,
      reloadProviders,
      toggleFavoriteModel,
      updateProvider,
      validateProviderConfig,
    }),
    [
      createProvider,
      deleteProvider,
      error,
      isLoading,
      listProviderModels,
      providers,
      reloadProviders,
      toggleFavoriteModel,
      updateProvider,
      validateProviderConfig,
    ],
  );

  return <ProvidersContext.Provider value={contextValue}>{children}</ProvidersContext.Provider>;
};

export default ProvidersProvider;
