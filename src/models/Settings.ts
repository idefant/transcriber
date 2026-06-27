export type SettingsSectionKey =
  | 'general'
  | 'hotkeys'
  | 'providers'
  | 'speechToText'
  | 'postProcessing'
  | 'about';

export type ThemeMode = 'dark' | 'light';
export type ThemePreference = 'auto' | ThemeMode;
export type TriggerMode = 'hold' | 'press';
export type EffectiveUiLanguage = 'en' | 'ru';
export type UiLanguage = 'en' | 'ru' | 'system';
export type OverlayVariant = 'bottom' | 'center';
export type OverlayScreenMode = 'all' | 'cursor';

export interface AppSettings {
  cancelHotkey: string;
  copyLatestHotkey: string;
  effectiveUiLanguage: EffectiveUiLanguage;
  hotkey: string;
  isDebugLoggingEnabled: boolean;
  isLaunchAtLoginEnabled: boolean;
  isMuteWhileRecordingEnabled: boolean;
  isOfferUnstableVersionsEnabled: boolean;
  overlayScreenMode: OverlayScreenMode;
  overlayVariant: OverlayVariant;
  pasteLatestHotkey: string;
  repeatLatestHotkey: string;
  themePreference: ThemePreference;
  triggerMode: TriggerMode;
  uiLanguage: UiLanguage;
}

export type AppSettingsInput = Partial<Omit<AppSettings, 'effectiveUiLanguage'>>;
