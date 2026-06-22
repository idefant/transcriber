import { type FC, type ReactNode, useCallback, useEffect, useMemo, useState } from 'react';

import { AppSettingsContext } from '#/app/settingsContext';
import * as settingsApi from '#/shared/settingsApi';

import type { AppSettings, AppSettingsInput } from '#/models/Settings';

interface AppSettingsProviderProps {
  children: ReactNode;
}

const defaultAppSettings: AppSettings = {
  effectiveUiLanguage: 'en',
  hotkey: 'Ctrl+Space',
  isDebugLoggingEnabled: false,
  isLaunchAtLoginEnabled: true,
  isMuteWhileRecordingEnabled: true,
  themePreference: 'light',
  triggerMode: 'press',
  uiLanguage: 'system',
};

const AppSettingsProvider: FC<AppSettingsProviderProps> = ({ children }) => {
  const [settings, setSettings] = useState<AppSettings>(defaultAppSettings);
  const [isLoading, setIsLoading] = useState(true);
  const [error, setError] = useState<string>();

  const reloadSettings = useCallback(async () => {
    setIsLoading(true);
    setError(undefined);

    try {
      const nextSettings = await settingsApi.getAppSettings();

      setSettings(nextSettings);
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
      void reloadSettings().catch(() => {
        // The error is already stored in context.
      });
    });
  }, [reloadSettings]);

  const updateSettings = useCallback(async (input: AppSettingsInput) => {
    try {
      const nextSettings = await settingsApi.updateAppSettings(input);

      setSettings(nextSettings);
      setError(undefined);

      return nextSettings;
    } catch (unknownError) {
      const message = unknownError instanceof Error ? unknownError.message : String(unknownError);

      setError(message);
      throw unknownError;
    }
  }, []);

  const contextValue = useMemo(
    () => ({
      error,
      isLoading,
      reloadSettings,
      settings,
      updateSettings,
    }),
    [error, isLoading, reloadSettings, settings, updateSettings],
  );

  return <AppSettingsContext.Provider value={contextValue}>{children}</AppSettingsContext.Provider>;
};

export default AppSettingsProvider;
