import { type FC, useEffect, useRef } from 'react';
import { RouterProvider } from 'react-router';
import { notification } from 'antd';
import { useTranslation } from 'react-i18next';

import AppThemeProvider from '#/app/AppThemeProvider';
import DictationHotkeyFallback from '#/app/DictationHotkeyFallback';
import I18nProvider from '#/app/I18nProvider';
import { router } from '#/app/router';
import * as updaterApi from '#/shared/updaterApi';

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

// Performs a single silent update check after settings have loaded.
// If an update is available, shows a non-intrusive notification.
const UpdateChecker: FC = () => {
  const { t } = useTranslation();
  const [notificationApi, notificationContextHolder] = notification.useNotification();
  const hasChecked = useRef(false);
  const isSettingsLoaded = useSettingsStore((s) => !s.isLoading && !s.error);
  const isOfferUnstableVersionsEnabled = useSettingsStore(
    (s) => s.settings.isOfferUnstableVersionsEnabled,
  );

  useEffect(() => {
    if (!isSettingsLoaded || hasChecked.current) {
      return;
    }

    hasChecked.current = true;

    void (async () => {
      const info = await updaterApi.checkForUpdate(isOfferUnstableVersionsEnabled);
      if (!info) {
        return;
      }

      notificationApi.info({
        message: t('settings.about.updateAvailable', { version: info.version }),
        description: info.notes ?? undefined,
        placement: 'bottomRight',
        duration: 0,
        key: 'update-available',
      });
    })();
  }, [isSettingsLoaded, isOfferUnstableVersionsEnabled, notificationApi, t]);

  return <>{notificationContextHolder}</>;
};

const App: FC = () => {
  return (
    <I18nProvider>
      <AppThemeProvider>
        <StoreLoader />
        <HistorySubscription />
        <DictationHotkeyFallback />
        <UpdateChecker />
        <RouterProvider router={router} />
      </AppThemeProvider>
    </I18nProvider>
  );
};

export default App;
