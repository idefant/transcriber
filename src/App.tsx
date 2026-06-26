import { type FC, useEffect, useRef } from 'react';
import { RouterProvider } from 'react-router';
import { listen } from '@tauri-apps/api/event';
import { notification } from 'antd';
import { useTranslation } from 'react-i18next';

import AppThemeProvider from '#/app/AppThemeProvider';
import DictationHotkeyFallback from '#/app/DictationHotkeyFallback';
import I18nProvider from '#/app/I18nProvider';
import { router } from '#/app/router';
import { routes } from '#/shared/routes';
import * as updaterApi from '#/shared/updaterApi';

import {
  initHistoryEventSubscription,
  useCatalogStore,
  useHistoryStore,
  useProcessingStore,
  useProvidersStore,
  useSettingsStore,
} from '#/stores';

interface OpenHistoryRecordEvent {
  recordId: string;
  month: string;
  date: string;
}

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

// Reveals a history record in the main window when the overlay error/warning
// notification requests it. Navigates to the history page, then hands the record
// off to the history store for the page to select.
const OpenRecordSubscription: FC = () => {
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    void listen<OpenHistoryRecordEvent>('open-history-record', (event) => {
      void router.navigate(routes.history);
      useHistoryStore.getState().openRecord(event.payload);
    }).then((fn) => {
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
        <OpenRecordSubscription />
        <DictationHotkeyFallback />
        <UpdateChecker />
        <RouterProvider router={router} />
      </AppThemeProvider>
    </I18nProvider>
  );
};

export default App;
