export type SettingsSectionKey =
  | 'general'
  | 'hotkeys'
  | 'providers'
  | 'speechToText'
  | 'postProcessing';

export type ThemeMode = 'dark' | 'light';
export type ThemePreference = 'auto' | ThemeMode;
export type TriggerMode = 'hold' | 'press';
export type UiLanguage = 'en' | 'ru';

export interface AppSettings {
  areDictationSoundsEnabled: boolean;
  hotkey: string;
  themePreference: ThemePreference;
  triggerMode: TriggerMode;
  uiLanguage: UiLanguage;
}

export type AppSettingsInput = Partial<AppSettings>;
