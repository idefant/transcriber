export type SettingsSectionKey =
  | 'general'
  | 'design'
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
export type RecordingAudioMode = 'mute' | 'off' | 'pause';

export interface AppSettings {
  cancelHotkey: string;
  copyLatestHotkey: string;
  effectiveUiLanguage: EffectiveUiLanguage;
  hotkey: string;
  isDebugLoggingEnabled: boolean;
  isLaunchAtLoginEnabled: boolean;
  isRestoreAudioWhilePausedEnabled: boolean;
  isSilenceTrimmingEnabled: boolean;
  isTelemetryEnabled: boolean;
  isUpdateNotificationsEnabled: boolean;
  isOfferUnstableVersionsEnabled: boolean;
  overlayScreenMode: OverlayScreenMode;
  overlayVariant: OverlayVariant;
  pauseHotkey: string;
  pasteLatestHotkey: string;
  recordingAudioMode: RecordingAudioMode;
  repeatLatestHotkey: string;
  themePreference: ThemePreference;
  triggerMode: TriggerMode;
  uiLanguage: UiLanguage;
}

export type AppSettingsInput = Partial<Omit<AppSettings, 'effectiveUiLanguage'>>;
