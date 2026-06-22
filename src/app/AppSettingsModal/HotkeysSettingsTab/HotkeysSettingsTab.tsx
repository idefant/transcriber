import { type FC } from 'react';
import { Input, Segmented } from 'antd';
import { useTranslation } from 'react-i18next';

import SettingRow from '../SettingRow';

import styles from './HotkeysSettingsTab.module.scss';

import type { TriggerMode } from '#/models/Settings';

interface HotkeysSettingsTabProps {
  cancelHotkey: string;
  hotkey: string;
  onCancelHotkeyChange: (value: string) => void;
  onHotkeyChange: (value: string) => void;
  onTriggerModeChange: (value: TriggerMode) => void;
  triggerMode: TriggerMode;
}

const HotkeysSettingsTab: FC<HotkeysSettingsTabProps> = ({
  cancelHotkey,
  hotkey,
  onCancelHotkeyChange,
  onHotkeyChange,
  onTriggerModeChange,
  triggerMode,
}) => {
  const { t } = useTranslation();

  return (
    <div className={styles.settingsList}>
      <SettingRow
        description={t('settings.hotkeys.startRecording.description')}
        title={t('settings.hotkeys.startRecording.title')}
      >
        <Input
          className={styles.hotkeyInput}
          value={hotkey}
          onChange={(event) => {
            onHotkeyChange(event.target.value);
          }}
        />
      </SettingRow>

      <SettingRow
        description={t('settings.hotkeys.cancelRecording.description')}
        title={t('settings.hotkeys.cancelRecording.title')}
      >
        <Input
          allowClear
          className={styles.hotkeyInput}
          placeholder={t('settings.hotkeys.cancelRecording.placeholder')}
          value={cancelHotkey}
          onChange={(event) => {
            onCancelHotkeyChange(event.target.value);
          }}
        />
      </SettingRow>

      <SettingRow
        description={t('settings.hotkeys.triggerMode.description')}
        title={t('settings.hotkeys.triggerMode.title')}
      >
        <Segmented<TriggerMode>
          className={styles.triggerModePicker}
          options={[
            {
              label: t('settings.hotkeys.triggerMode.press'),
              value: 'press',
            },
            {
              label: t('settings.hotkeys.triggerMode.hold'),
              value: 'hold',
            },
          ]}
          value={triggerMode}
          onChange={onTriggerModeChange}
        />
      </SettingRow>
    </div>
  );
};

export default HotkeysSettingsTab;
