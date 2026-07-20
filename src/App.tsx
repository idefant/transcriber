import { type FC, useEffect, useRef } from 'react';
import { RouterProvider } from 'react-router';
import { listen } from '@tauri-apps/api/event';
import { notification } from 'antd';

import ApplicationErrorBoundary from '#/app/ApplicationErrorBoundary';
import AppThemeProvider from '#/app/AppThemeProvider';
import CloseWindowHotkey from '#/app/CloseWindowHotkey';
import {
  createUpdateNotificationArgs,
  updateNotificationKey,
} from '#/app/createUpdateNotificationArgs';
import DictationHotkeyFallback from '#/app/DictationHotkeyFallback';
import I18nProvider from '#/app/I18nProvider';
import { i18n } from '#/app/I18nProvider/i18n';
import { router } from '#/app/router';
import StartupGate from '#/app/StartupGate';
import { routes } from '#/shared/routes';
import { configureTelemetry } from '#/shared/telemetry';

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

// Запускает первоначальную загрузку данных для всех сторов один раз при монтировании приложения.
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

// Подписывается на события истории Tauri на всё время жизни приложения.
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

// Открывает запись истории в главном окне, когда это запрашивает уведомление
// об ошибке/предупреждении из оверлея. Переходит на страницу истории, а затем
// передаёт запись в стор истории, чтобы страница могла её выбрать.
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

// Открывает модальное окно настроек на нужной вкладке, когда пользователь кликает
// по системному уведомлению об ошибке конфигурации. Бэкенд показывает главное окно
// перед отправкой этого события.
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

/** Открывает словарь по клику на системное уведомление о превышении STT-prompt. */
const OpenDictionarySubscription: FC = () => {
  useEffect(() => {
    let unlisten: (() => void) | undefined;

    void listen('open-dictionary', () => {
      useUiStore.getState().closeSettings();
      void router.navigate(routes.dictionary);
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

// Выполняет одну тихую проверку обновлений после загрузки настроек.
// Если обновление доступно, показывает временное уведомление внутри приложения.
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

/** Синхронизирует разрешение пользователя на отправку обезличенных отчётов об ошибках. */
const Telemetry: FC = () => {
  const isSettingsLoaded = useSettingsStore((s) => !s.isLoading && !s.error);
  const isTelemetryEnabled = useSettingsStore((s) => s.settings.isTelemetryEnabled);

  useEffect(() => {
    if (isSettingsLoaded) {
      configureTelemetry(isTelemetryEnabled);
    }
  }, [isSettingsLoaded, isTelemetryEnabled]);

  return null;
};

const App: FC = () => {
  const isTelemetryEnabled = useSettingsStore((s) => s.settings.isTelemetryEnabled);

  return (
    <I18nProvider>
      <AppThemeProvider>
        <ApplicationErrorBoundary isTelemetryEnabled={isTelemetryEnabled}>
          <StartupGate>
            <StoreLoader />
            <HistorySubscription />
            <OpenRecordSubscription />
            <OpenSettingsSubscription />
            <OpenDictionarySubscription />
            <DictationHotkeyFallback />
            <CloseWindowHotkey />
            <Telemetry />
            <UpdateChecker />
            <RouterProvider router={router} />
          </StartupGate>
        </ApplicationErrorBoundary>
      </AppThemeProvider>
    </I18nProvider>
  );
};

export default App;
