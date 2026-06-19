import { type FC, type ReactNode } from 'react';
import { Segmented, Select, Switch } from 'antd';
import { MonitorIcon, MoonIcon, SunIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

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

const GeneralSettingsTab: FC<GeneralSettingsTabProps> = ({
  areDictationSoundsEnabled,
  onDictationSoundsEnabledChange,
  onThemePreferenceChange,
  onUiLanguageChange,
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
        description={t('settings.general.dictationSounds.description')}
        title={t('settings.general.dictationSounds.title')}
      >
        <Switch checked={areDictationSoundsEnabled} onChange={onDictationSoundsEnabledChange} />
      </SettingRow>

      <SettingRow
        description={t('settings.general.language.description')}
        title={t('settings.general.language.title')}
      >
        <Select
          className={styles.languageSelect}
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
    </div>
  );
};

export default GeneralSettingsTab;
