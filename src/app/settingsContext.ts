import { createContext, useContext } from 'react';

import type { AppSettings, AppSettingsInput } from '#/models/Settings';

interface AppSettingsContextValue {
  error?: string;
  isLoading: boolean;
  settings: AppSettings;
  reloadSettings: () => Promise<void>;
  updateSettings: (input: AppSettingsInput) => Promise<AppSettings>;
}

export const AppSettingsContext = createContext<AppSettingsContextValue | undefined>(undefined);

export const useAppSettings = () => {
  const value = useContext(AppSettingsContext);

  if (value === undefined) {
    throw new Error('useAppSettings must be used inside AppSettingsProvider');
  }

  return value;
};
