import { type FC } from 'react';
import { Segmented } from 'antd';
import { useTranslation } from 'react-i18next';

import SettingRow from '../SettingRow';

import HotkeyInput from './HotkeyInput';

import styles from './HotkeysSettingsTab.module.scss';

import type { TriggerMode } from '#/models/Settings';

interface HotkeysSettingsTabProps {
  cancelHotkey: string;
  copyLatestHotkey: string;
  hotkey: string;
  onCancelHotkeyChange: (value: string) => void;
  onCopyLatestHotkeyChange: (value: string) => void;
  onHotkeyChange: (value: string) => void;
  onPasteLatestHotkeyChange: (value: string) => void;
  onPauseHotkeyChange: (value: string) => void;
  onRepeatLatestHotkeyChange: (value: string) => void;
  onTriggerModeChange: (value: TriggerMode) => void;
  pasteLatestHotkey: string;
  pauseHotkey: string;
  repeatLatestHotkey: string;
  triggerMode: TriggerMode;
}

const RECORDING_HOTKEY_DEFAULT = 'Ctrl+Space';
const CANCEL_HOTKEY_DEFAULT = 'Ctrl+Z';

const HotkeysSettingsTab: FC<HotkeysSettingsTabProps> = ({
  cancelHotkey,
  copyLatestHotkey,
  hotkey,
  onCancelHotkeyChange,
  onCopyLatestHotkeyChange,
  onHotkeyChange,
  onPasteLatestHotkeyChange,
  onPauseHotkeyChange,
  onRepeatLatestHotkeyChange,
  onTriggerModeChange,
  pasteLatestHotkey,
  pauseHotkey,
  repeatLatestHotkey,
  triggerMode,
}) => {
  const { t } = useTranslation();
  const emptyPlaceholder = t('settings.hotkeys.unused');
  const resetLabel = t('settings.hotkeys.reset');

  return (
    <div className={styles.settingsList}>
      <SettingRow
        description={t('settings.hotkeys.startRecording.description')}
        title={t('settings.hotkeys.startRecording.title')}
      >
        <HotkeyInput
          allowEmpty={false}
          defaultValue={RECORDING_HOTKEY_DEFAULT}
          emptyPlaceholder={emptyPlaceholder}
          resetLabel={resetLabel}
          value={hotkey}
          onChange={onHotkeyChange}
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

      <SettingRow
        description={t('settings.hotkeys.pauseRecording.description')}
        title={t('settings.hotkeys.pauseRecording.title')}
      >
        {/* Смена режима запуска может заблокировать поле прямо во время записи комбинации,
            поэтому key перемонтирует его и прекращает захват клавиш. */}
        <HotkeyInput
          allowEmpty
          defaultValue=""
          emptyPlaceholder={emptyPlaceholder}
          isDisabled={triggerMode === 'hold'}
          key={triggerMode}
          resetLabel={resetLabel}
          value={pauseHotkey}
          onChange={onPauseHotkeyChange}
        />
      </SettingRow>

      <SettingRow
        description={t('settings.hotkeys.cancelRecording.description')}
        title={t('settings.hotkeys.cancelRecording.title')}
      >
        <HotkeyInput
          allowEmpty
          defaultValue={CANCEL_HOTKEY_DEFAULT}
          emptyPlaceholder={emptyPlaceholder}
          resetLabel={resetLabel}
          value={cancelHotkey}
          onChange={onCancelHotkeyChange}
        />
      </SettingRow>

      <SettingRow
        description={t('settings.hotkeys.copyLatest.description')}
        title={t('settings.hotkeys.copyLatest.title')}
      >
        <HotkeyInput
          allowEmpty
          defaultValue=""
          emptyPlaceholder={emptyPlaceholder}
          resetLabel={resetLabel}
          value={copyLatestHotkey}
          onChange={onCopyLatestHotkeyChange}
        />
      </SettingRow>

      <SettingRow
        description={t('settings.hotkeys.pasteLatest.description')}
        title={t('settings.hotkeys.pasteLatest.title')}
      >
        <HotkeyInput
          allowEmpty
          defaultValue=""
          emptyPlaceholder={emptyPlaceholder}
          resetLabel={resetLabel}
          value={pasteLatestHotkey}
          onChange={onPasteLatestHotkeyChange}
        />
      </SettingRow>

      <SettingRow
        description={t('settings.hotkeys.repeatLatest.description')}
        title={t('settings.hotkeys.repeatLatest.title')}
      >
        <HotkeyInput
          allowEmpty
          defaultValue=""
          emptyPlaceholder={emptyPlaceholder}
          resetLabel={resetLabel}
          value={repeatLatestHotkey}
          onChange={onRepeatLatestHotkeyChange}
        />
      </SettingRow>
    </div>
  );
};

export default HotkeysSettingsTab;
