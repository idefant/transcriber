export type SettingsSectionKey =
  | 'general'
  | 'hotkeys'
  | 'providers'
  | 'speechToText'
  | 'postProcessing';

export type ThemeMode = 'dark' | 'light';
export type ThemePreference = 'auto' | ThemeMode;
export type TriggerMode = 'hold' | 'press';
export type EffectiveUiLanguage = 'en' | 'ru';
export type UiLanguage = 'en' | 'ru' | 'system';

export interface AppSettings {
  areDictationSoundsEnabled: boolean;
  effectiveUiLanguage: EffectiveUiLanguage;
  hotkey: string;
  isDebugLoggingEnabled: boolean;
  isLaunchAtLoginEnabled: boolean;
  isMuteWhileRecordingEnabled: boolean;
  themePreference: ThemePreference;
  triggerMode: TriggerMode;
  uiLanguage: UiLanguage;
}

export type AppSettingsInput = Partial<Omit<AppSettings, 'effectiveUiLanguage'>>;
