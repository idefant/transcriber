import { type FC } from 'react';
import { useTranslation } from 'react-i18next';

import SettingRow from '../SettingRow';

import HotkeyInput from './HotkeyInput';

import styles from './HotkeysSettingsTab.module.scss';

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
  pasteLatestHotkey: string;
  pauseHotkey: string;
  repeatLatestHotkey: string;
  isPauseHotkeyDisabled: boolean;
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
  pasteLatestHotkey,
  pauseHotkey,
  repeatLatestHotkey,
  isPauseHotkeyDisabled,
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
        description={t('settings.hotkeys.pauseRecording.description')}
        notice={
          isPauseHotkeyDisabled ? t('settings.hotkeys.pauseRecording.disabledNotice') : undefined
        }
        title={t('settings.hotkeys.pauseRecording.title')}
      >
        {/* Смена режима запуска может заблокировать поле прямо во время записи комбинации,
            поэтому key перемонтирует его и прекращает захват клавиш. */}
        <HotkeyInput
          allowEmpty
          defaultValue=""
          emptyPlaceholder={emptyPlaceholder}
          isDisabled={isPauseHotkeyDisabled}
          key={String(isPauseHotkeyDisabled)}
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
