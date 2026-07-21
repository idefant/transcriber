import { type FC } from 'react';
import { Segmented, Select, Switch } from 'antd';
import { useTranslation } from 'react-i18next';

import SettingRow from '../SettingRow';

import styles from './GeneralSettingsTab.module.scss';

import type { RecordingAudioMode, TriggerMode } from '#/models/Settings';

interface GeneralSettingsTabProps {
  isLaunchAtLoginEnabled: boolean;
  isRestoreAudioWhilePausedEnabled: boolean;
  isSilenceTrimmingEnabled: boolean;
  onLaunchAtLoginEnabledChange: (value: boolean) => void;
  onRecordingAudioModeChange: (value: RecordingAudioMode) => void;
  onRestoreAudioWhilePausedEnabledChange: (value: boolean) => void;
  onSilenceTrimmingEnabledChange: (value: boolean) => void;
  recordingAudioMode: RecordingAudioMode;
  triggerMode: TriggerMode;
  onTriggerModeChange: (value: TriggerMode) => void;
}

const GeneralSettingsTab: FC<GeneralSettingsTabProps> = ({
  isLaunchAtLoginEnabled,
  isRestoreAudioWhilePausedEnabled,
  isSilenceTrimmingEnabled,
  onLaunchAtLoginEnabledChange,
  onRecordingAudioModeChange,
  onRestoreAudioWhilePausedEnabledChange,
  onSilenceTrimmingEnabledChange,
  recordingAudioMode,
  triggerMode,
  onTriggerModeChange,
}) => {
  const { t } = useTranslation();

  return (
    <div className={styles.settingsList}>
      <SettingRow
        description={t('settings.general.triggerMode.description')}
        title={t('settings.general.triggerMode.title')}
      >
        <Segmented<TriggerMode>
          className={styles.triggerModePicker}
          options={[
            {
              label: t('settings.general.triggerMode.press'),
              value: 'press',
            },
            {
              label: t('settings.general.triggerMode.hold'),
              value: 'hold',
            },
          ]}
          value={triggerMode}
          onChange={onTriggerModeChange}
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
          notice={
            triggerMode === 'hold'
              ? t('settings.general.restoreAudioWhilePaused.disabledNotice')
              : undefined
          }
          title={t('settings.general.restoreAudioWhilePaused.title')}
        >
          <Switch
            checked={isRestoreAudioWhilePausedEnabled}
            disabled={triggerMode === 'hold'}
            onChange={onRestoreAudioWhilePausedEnabledChange}
          />
        </SettingRow>
      )}

      <SettingRow
        description={t('settings.general.silenceTrimming.description')}
        title={t('settings.general.silenceTrimming.title')}
      >
        <Switch checked={isSilenceTrimmingEnabled} onChange={onSilenceTrimmingEnabledChange} />
      </SettingRow>

      <SettingRow
        description={t('settings.general.launchAtLogin.description')}
        title={t('settings.general.launchAtLogin.title')}
      >
        <Switch checked={isLaunchAtLoginEnabled} onChange={onLaunchAtLoginEnabledChange} />
      </SettingRow>
    </div>
  );
};

export default GeneralSettingsTab;
