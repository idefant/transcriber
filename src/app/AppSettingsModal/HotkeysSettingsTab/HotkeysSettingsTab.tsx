import { type FC } from 'react';
import { Input, Segmented } from 'antd';

import SettingRow from '../SettingRow';

import styles from './HotkeysSettingsTab.module.scss';

import type { TriggerMode } from '#/models/Settings';

interface HotkeysSettingsTabProps {
  hotkey: string;
  onHotkeyChange: (value: string) => void;
  onTriggerModeChange: (value: TriggerMode) => void;
  triggerMode: TriggerMode;
}

const HotkeysSettingsTab: FC<HotkeysSettingsTabProps> = ({
  hotkey,
  onHotkeyChange,
  onTriggerModeChange,
  triggerMode,
}) => (
  <div className={styles.settingsList}>
    <SettingRow description="Комбинация клавиш, которая запускает запись" title="Старт записи">
      <Input
        className={styles.hotkeyInput}
        value={hotkey}
        onChange={(event) => {
          onHotkeyChange(event.target.value);
        }}
      />
    </SettingRow>

    <SettingRow
      description="Запускать запись сразу по нажатию или только пока комбинация зажата"
      title="Режим запуска"
    >
      <Segmented<TriggerMode>
        className={styles.triggerModePicker}
        options={[
          {
            label: 'По нажатию',
            value: 'press',
          },
          {
            label: 'По зажатию',
            value: 'hold',
          },
        ]}
        value={triggerMode}
        onChange={onTriggerModeChange}
      />
    </SettingRow>
  </div>
);

export default HotkeysSettingsTab;
