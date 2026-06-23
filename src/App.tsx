import { type FC, useEffect } from 'react';
import { RouterProvider } from 'react-router';

import AppThemeProvider from '#/app/AppThemeProvider';
import DictationHotkeyFallback from '#/app/DictationHotkeyFallback';
import I18nProvider from '#/app/I18nProvider';
import { router } from '#/app/router';

import {
  initHistoryEventSubscription,
  useCatalogStore,
  useProcessingStore,
  useProvidersStore,
  useSettingsStore,
} from '#/stores';

// Triggers initial data loads for all stores once on app mount.
const StoreLoader: FC = () => {
  useEffect(() => {
    queueMicrotask(() => {
      void useSettingsStore.getState().load();
      void useProvidersStore.getState().load();
      void useProcessingStore.getState().load();
      void useProcessingStore.getState().loadDefaultPrompts();
      void useCatalogStore.getState().load();
    });
  }, []);

  return null;
};

// Subscribes to Tauri history events for the lifetime of the app.
const HistorySubscription: FC = () => {
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    void initHistoryEventSubscription().then((fn) => {
      unlisten = fn;
      return null;
    });

    return () => {
      unlisten?.();
    };
  }, []);

  return null;
};

const App: FC = () => {
  return (
    <I18nProvider>
      <AppThemeProvider>
        <StoreLoader />
        <HistorySubscription />
        <DictationHotkeyFallback />
        <RouterProvider router={router} />
      </AppThemeProvider>
    </I18nProvider>
  );
};

export default App;
