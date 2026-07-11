import { useShallow } from 'zustand/react/shallow';

export { useCatalogStore } from './catalogStore';
export { useDictionaryStore } from './dictionaryStore';
export { initHistoryEventSubscription, useHistoryStore } from './historyStore';
export { useProcessingStore } from './processingStore';
export { useProvidersStore } from './providersStore';
export { useSettingsStore } from './settingsStore';
export { useUiStore } from './uiStore';
export { useUpdaterStore } from './updaterStore';

// Хуки для совместимости — тот же интерфейс, что и у старых хуков React Context.
// Потребители, которым нужно деструктурировать несколько полей, используют их, чтобы избежать лишних ре-рендеров.

import { useCatalogStore } from './catalogStore';
import { useProcessingStore } from './processingStore';
import { useProvidersStore } from './providersStore';
import { useSettingsStore } from './settingsStore';

export const useAppSettings = () =>
  useSettingsStore(
    useShallow((s) => ({
      error: s.error,
      isLoading: s.isLoading,
      settings: s.settings,
      reloadSettings: s.load,
      updateSettings: s.updateSettings,
    })),
  );

export const useProviders = () =>
  useProvidersStore(
    useShallow((s) => ({
      createProvider: s.createProvider,
      deleteProvider: s.deleteProvider,
      error: s.error,
      isLoading: s.isLoading,
      listProviderModels: s.listProviderModels,
      providers: s.providers,
      reloadProviders: s.load,
      updateProvider: s.updateProvider,
      validateProviderConfig: s.validateProviderConfig,
    })),
  );

export const useProcessing = () =>
  useProcessingStore(
    useShallow((s) => ({
      config: s.config,
      defaultPrompts: s.defaultPrompts,
      isLoading: s.isLoading,
      loadDefaultPrompts: s.loadDefaultPrompts,
      updatePostProcessConfig: s.updatePostProcessConfig,
      updateSttConfig: s.updateSttConfig,
    })),
  );

export const useCatalog = () =>
  useCatalogStore(
    useShallow((s) => ({
      catalog: s.catalog,
      isLoading: s.isLoading,
    })),
  );
