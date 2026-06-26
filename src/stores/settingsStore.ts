import { create } from 'zustand';

import * as settingsApi from '#/shared/settingsApi';

import type { AppSettings, AppSettingsInput } from '#/models/Settings';

const defaultAppSettings: AppSettings = {
  cancelHotkey: 'Ctrl+Z',
  effectiveUiLanguage: 'en',
  hotkey: 'Ctrl+Space',
  isDebugLoggingEnabled: false,
  isLaunchAtLoginEnabled: true,
  isMuteWhileRecordingEnabled: true,
  isOfferUnstableVersionsEnabled: false,
  overlayScreenMode: 'all',
  overlayVariant: 'center',
  themePreference: 'light',
  triggerMode: 'press',
  uiLanguage: 'system',
};

interface SettingsState {
  settings: AppSettings;
  isLoading: boolean;
  error?: string;
  load: () => Promise<void>;
  updateSettings: (input: AppSettingsInput) => Promise<AppSettings>;
}

export const useSettingsStore = create<SettingsState>((set) => ({
  settings: defaultAppSettings,
  isLoading: true,
  error: undefined,

  load: async () => {
    set({ isLoading: true, error: undefined });
    try {
      const settings = await settingsApi.getAppSettings();
      set({ settings });
    } catch (error) {
      set({ error: error instanceof Error ? error.message : String(error) });
      throw error;
    } finally {
      set({ isLoading: false });
    }
  },

  updateSettings: async (input) => {
    try {
      const settings = await settingsApi.updateAppSettings(input);
      set({ settings, error: undefined });
      return settings;
    } catch (error) {
      set({ error: error instanceof Error ? error.message : String(error) });
      throw error;
    }
  },
}));
