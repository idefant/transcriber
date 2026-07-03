import { type FC, useEffect, useRef } from 'react';
import { RouterProvider } from 'react-router';
import { listen } from '@tauri-apps/api/event';
import { notification } from 'antd';

import AppThemeProvider from '#/app/AppThemeProvider';
import {
  createUpdateNotificationArgs,
  updateNotificationKey,
} from '#/app/createUpdateNotificationArgs';
import DictationHotkeyFallback from '#/app/DictationHotkeyFallback';
import I18nProvider from '#/app/I18nProvider';
import { i18n } from '#/app/I18nProvider/i18n';
import { router } from '#/app/router';
import { routes } from '#/shared/routes';

import type { SettingsSectionKey } from '#/models/Settings';
import {
  initHistoryEventSubscription,
  useCatalogStore,
  useHistoryStore,
  useProcessingStore,
  useProvidersStore,
  useSettingsStore,
  useUiStore,
  useUpdaterStore,
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
      useUiStore.getState().closeSettings();
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

// Opens the settings modal on the relevant tab when the user clicks the system
// notification about a configuration error. The backend shows the main window
// before emitting this event.
const OpenSettingsSubscription: FC = () => {
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    void listen<{ section: SettingsSectionKey }>('open-settings', (event) => {
      useUiStore.getState().openSettings(event.payload.section);
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
// If an update is available, shows the timed in-app notification.
const UpdateChecker: FC = () => {
  const [notificationApi, notificationContextHolder] = notification.useNotification();
  const hasChecked = useRef(false);
  const isSettingsLoaded = useSettingsStore((s) => !s.isLoading && !s.error);
  const isUpdateNotificationsEnabled = useSettingsStore(
    (s) => s.settings.isUpdateNotificationsEnabled,
  );
  const isOfferUnstableVersionsEnabled = useSettingsStore(
    (s) => s.settings.isOfferUnstableVersionsEnabled,
  );

  useEffect(() => {
    if (!isSettingsLoaded || hasChecked.current) {
      return;
    }

    hasChecked.current = true;

    if (!isUpdateNotificationsEnabled) {
      return;
    }

    void (async () => {
      try {
        const info = await useUpdaterStore
          .getState()
          .checkForUpdates(isOfferUnstableVersionsEnabled);

        if (!info) {
          return;
        }

        notificationApi.open(
          createUpdateNotificationArgs({
            info,
            onDownload: () => {
              notificationApi.destroy(updateNotificationKey);
              useUiStore.getState().openSettings('about');
            },
            t: i18n.t.bind(i18n),
          }),
        );
      } catch {
        return;
      }
    })();
  }, [
    isOfferUnstableVersionsEnabled,
    isSettingsLoaded,
    isUpdateNotificationsEnabled,
    notificationApi,
  ]);

  return <>{notificationContextHolder}</>;
};

const App: FC = () => {
  return (
    <I18nProvider>
      <AppThemeProvider>
        <StoreLoader />
        <HistorySubscription />
        <OpenRecordSubscription />
        <OpenSettingsSubscription />
        <DictationHotkeyFallback />
        <UpdateChecker />
        <RouterProvider router={router} />
      </AppThemeProvider>
    </I18nProvider>
  );
};

export default App;
