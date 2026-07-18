import { type FC, type ReactNode } from 'react';
import { Button, Segmented, Select, Switch } from 'antd';
import { MonitorIcon, MoonIcon, SunIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import SettingRow from '../SettingRow';

import styles from './GeneralSettingsTab.module.scss';

import type {
  OverlayScreenMode,
  OverlayVariant,
  RecordingAudioMode,
  ThemePreference,
  UiLanguage,
} from '#/models/Settings';

interface GeneralSettingsTabProps {
  isDebugLoggingEnabled: boolean;
  isLaunchAtLoginEnabled: boolean;
  isRestoreAudioWhilePausedEnabled: boolean;
  onDebugLogsFolderOpen: () => void;
  onDebugLoggingEnabledChange: (value: boolean) => void;
  onLaunchAtLoginEnabledChange: (value: boolean) => void;
  onOverlayScreenModeChange: (value: OverlayScreenMode) => void;
  onOverlayVariantChange: (value: OverlayVariant) => void;
  onRecordingAudioModeChange: (value: RecordingAudioMode) => void;
  onRestoreAudioWhilePausedEnabledChange: (value: boolean) => void;
  onThemePreferenceChange: (value: ThemePreference) => void;
  onUiLanguageChange: (value: UiLanguage) => void;
  overlayScreenMode: OverlayScreenMode;
  overlayVariant: OverlayVariant;
  recordingAudioMode: RecordingAudioMode;
  themePreference: ThemePreference;
  uiLanguage: UiLanguage;
}

const GeneralSettingsTab: FC<GeneralSettingsTabProps> = ({
  isDebugLoggingEnabled,
  isLaunchAtLoginEnabled,
  isRestoreAudioWhilePausedEnabled,
  onDebugLogsFolderOpen,
  onDebugLoggingEnabledChange,
  onLaunchAtLoginEnabledChange,
  onOverlayScreenModeChange,
  onOverlayVariantChange,
  onRecordingAudioModeChange,
  onRestoreAudioWhilePausedEnabledChange,
  onThemePreferenceChange,
  onUiLanguageChange,
  overlayScreenMode,
  overlayVariant,
  recordingAudioMode,
  themePreference,
  uiLanguage,
}) => {
  const { t } = useTranslation();
  const themeOptions: { icon: ReactNode; label: string; value: ThemePreference }[] = [
    {
      icon: <SunIcon size={15} strokeWidth={2} />,
      label: t('settings.general.theme.light'),
      value: 'light',
    },
    {
      icon: <MoonIcon size={15} strokeWidth={2} />,
      label: t('settings.general.theme.dark'),
      value: 'dark',
    },
    {
      icon: <MonitorIcon size={15} strokeWidth={2} />,
      label: t('settings.general.theme.auto'),
      value: 'auto',
    },
  ];

  return (
    <div className={styles.settingsList}>
      <SettingRow
        description={t('settings.general.theme.description')}
        title={t('settings.general.theme.title')}
      >
        <Segmented<ThemePreference>
          className={styles.themePicker}
          options={themeOptions}
          value={themePreference}
          onChange={onThemePreferenceChange}
        />
      </SettingRow>

      <SettingRow
        description={t('settings.general.recordingAudio.description')}
        title={t('settings.general.recordingAudio.title')}
      >
        <Select
          className={styles.recordingAudioSelect}
          placeholder={t('settings.general.recordingAudio.placeholder')}
          value={recordingAudioMode}
          options={[
            {
              label: t('settings.general.recordingAudio.mute'),
              value: 'mute',
            },
            {
              label: t('settings.general.recordingAudio.pause'),
              value: 'pause',
            },
            {
              label: t('settings.general.recordingAudio.off'),
              value: 'off',
            },
          ]}
          onChange={onRecordingAudioModeChange}
        />
      </SettingRow>

      {recordingAudioMode !== 'off' && (
        <SettingRow
          description={t('settings.general.restoreAudioWhilePaused.description')}
          title={t('settings.general.restoreAudioWhilePaused.title')}
        >
          <Switch
            checked={isRestoreAudioWhilePausedEnabled}
            onChange={onRestoreAudioWhilePausedEnabledChange}
          />
        </SettingRow>
      )}

      <SettingRow
        description={t('settings.general.launchAtLogin.description')}
        title={t('settings.general.launchAtLogin.title')}
      >
        <Switch checked={isLaunchAtLoginEnabled} onChange={onLaunchAtLoginEnabledChange} />
      </SettingRow>

      <SettingRow
        description={t('settings.general.language.description')}
        title={t('settings.general.language.title')}
      >
        <Select
          className={styles.languageSelect}
          placeholder={t('settings.general.language.placeholder')}
          value={uiLanguage}
          options={[
            {
              label: t('settings.general.language.system'),
              value: 'system',
            },
            {
              label: t('settings.general.language.ru'),
              value: 'ru',
            },
            {
              label: t('settings.general.language.en'),
              value: 'en',
            },
          ]}
          onChange={onUiLanguageChange}
        />
      </SettingRow>

      <SettingRow
        description={t('settings.general.overlayVariant.description')}
        title={t('settings.general.overlayVariant.title')}
      >
        <Select
          className={styles.overlaySelect}
          placeholder={t('settings.general.overlayVariant.placeholder')}
          value={overlayVariant}
          options={[
            {
              label: t('settings.general.overlayVariant.center'),
              value: 'center',
            },
            {
              label: t('settings.general.overlayVariant.bottom'),
              value: 'bottom',
            },
          ]}
          onChange={onOverlayVariantChange}
        />
      </SettingRow>

      <SettingRow
        description={t('settings.general.overlayScreenMode.description')}
        title={t('settings.general.overlayScreenMode.title')}
      >
        <Select
          className={styles.overlaySelect}
          placeholder={t('settings.general.overlayScreenMode.placeholder')}
          value={overlayScreenMode}
          options={[
            {
              label: t('settings.general.overlayScreenMode.all'),
              value: 'all',
            },
            {
              label: t('settings.general.overlayScreenMode.cursor'),
              value: 'cursor',
            },
          ]}
          onChange={onOverlayScreenModeChange}
        />
      </SettingRow>

      <div className={styles.debugLoggingGroup}>
        <SettingRow
          description={t('settings.general.debugLogging.description')}
          title={t('settings.general.debugLogging.title')}
        >
          <Switch checked={isDebugLoggingEnabled} onChange={onDebugLoggingEnabledChange} />
        </SettingRow>
        {isDebugLoggingEnabled && (
          <Button block size="middle" type="primary" onClick={onDebugLogsFolderOpen}>
            {t('settings.general.debugLogging.openFolder')}
          </Button>
        )}
      </div>
    </div>
  );
};

export default GeneralSettingsTab;
