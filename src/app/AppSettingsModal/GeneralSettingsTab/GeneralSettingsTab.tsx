import { type FC, type ReactNode } from 'react';
import { Segmented, Select, Switch } from 'antd';
import { MonitorIcon, MoonIcon, SunIcon } from 'lucide-react';

import SettingRow from '../SettingRow';

import styles from './GeneralSettingsTab.module.scss';

import type { ThemePreference, UiLanguage } from '#/models/Settings';

interface GeneralSettingsTabProps {
  areDictationSoundsEnabled: boolean;
  onDictationSoundsEnabledChange: (value: boolean) => void;
  onThemePreferenceChange: (value: ThemePreference) => void;
  onUiLanguageChange: (value: UiLanguage) => void;
  themePreference: ThemePreference;
  uiLanguage: UiLanguage;
}

const themeOptions: { icon: ReactNode; label: string; value: ThemePreference }[] = [
  {
    icon: <SunIcon size={15} strokeWidth={2} />,
    label: 'Светлая',
    value: 'light',
  },
  {
    icon: <MoonIcon size={15} strokeWidth={2} />,
    label: 'Темная',
    value: 'dark',
  },
  {
    icon: <MonitorIcon size={15} strokeWidth={2} />,
    label: 'Авто',
    value: 'auto',
  },
];

const GeneralSettingsTab: FC<GeneralSettingsTabProps> = ({
  areDictationSoundsEnabled,
  onDictationSoundsEnabledChange,
  onThemePreferenceChange,
  onUiLanguageChange,
  themePreference,
  uiLanguage,
}) => (
  <div className={styles.settingsList}>
    <SettingRow description="Выберите светлую, темную или системную тему" title="Тема">
      <Segmented<ThemePreference>
        className={styles.themePicker}
        options={themeOptions}
        value={themePreference}
        onChange={onThemePreferenceChange}
      />
    </SettingRow>

    <SettingRow
      description="Воспроизводить звук при старте и остановке записи"
      title="Звуки диктовки"
    >
      <Switch checked={areDictationSoundsEnabled} onChange={onDictationSoundsEnabledChange} />
    </SettingRow>

    <SettingRow
      description="Выберите язык, который используется в интерфейсе Transcriber"
      title="Язык интерфейса"
    >
      <Select
        className={styles.languageSelect}
        value={uiLanguage}
        options={[
          {
            label: 'Русский',
            value: 'ru',
          },
          {
            label: 'English',
            value: 'en',
          },
        ]}
        onChange={onUiLanguageChange}
      />
    </SettingRow>
  </div>
);

export default GeneralSettingsTab;
