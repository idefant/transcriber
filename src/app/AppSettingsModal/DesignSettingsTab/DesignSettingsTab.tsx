import { type FC, type ReactNode } from 'react';
import { Segmented, Select } from 'antd';
import { MonitorIcon, MoonIcon, SunIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import SettingRow from '../SettingRow';

import styles from './DesignSettingsTab.module.scss';

import type {
  OverlayScreenMode,
  OverlayVariant,
  ThemePreference,
  UiLanguage,
} from '#/models/Settings';

interface DesignSettingsTabProps {
  onOverlayScreenModeChange: (value: OverlayScreenMode) => void;
  onOverlayVariantChange: (value: OverlayVariant) => void;
  onThemePreferenceChange: (value: ThemePreference) => void;
  onUiLanguageChange: (value: UiLanguage) => void;
  overlayScreenMode: OverlayScreenMode;
  overlayVariant: OverlayVariant;
  themePreference: ThemePreference;
  uiLanguage: UiLanguage;
}

const DesignSettingsTab: FC<DesignSettingsTabProps> = ({
  onOverlayScreenModeChange,
  onOverlayVariantChange,
  onThemePreferenceChange,
  onUiLanguageChange,
  overlayScreenMode,
  overlayVariant,
  themePreference,
  uiLanguage,
}) => {
  const { t } = useTranslation();
  const themeOptions: { icon: ReactNode; label: string; value: ThemePreference }[] = [
    {
      icon: <SunIcon size={15} strokeWidth={2} />,
      label: t('settings.design.theme.light'),
      value: 'light',
    },
    {
      icon: <MoonIcon size={15} strokeWidth={2} />,
      label: t('settings.design.theme.dark'),
      value: 'dark',
    },
    {
      icon: <MonitorIcon size={15} strokeWidth={2} />,
      label: t('settings.design.theme.auto'),
      value: 'auto',
    },
  ];

  return (
    <div className={styles.settingsList}>
      <SettingRow
        description={t('settings.design.theme.description')}
        title={t('settings.design.theme.title')}
      >
        <Segmented<ThemePreference>
          className={styles.themePicker}
          options={themeOptions}
          value={themePreference}
          onChange={onThemePreferenceChange}
        />
      </SettingRow>

      <SettingRow
        description={t('settings.design.language.description')}
        title={t('settings.design.language.title')}
      >
        <Select
          className={styles.languageSelect}
          placeholder={t('settings.design.language.placeholder')}
          value={uiLanguage}
          options={[
            {
              label: t('settings.design.language.system'),
              value: 'system',
            },
            {
              label: t('settings.design.language.ru'),
              value: 'ru',
            },
            {
              label: t('settings.design.language.en'),
              value: 'en',
            },
          ]}
          onChange={onUiLanguageChange}
        />
      </SettingRow>

      <SettingRow
        description={t('settings.design.overlayVariant.description')}
        title={t('settings.design.overlayVariant.title')}
      >
        <Select
          className={styles.overlaySelect}
          placeholder={t('settings.design.overlayVariant.placeholder')}
          value={overlayVariant}
          options={[
            {
              label: t('settings.design.overlayVariant.center'),
              value: 'center',
            },
            {
              label: t('settings.design.overlayVariant.bottom'),
              value: 'bottom',
            },
          ]}
          onChange={onOverlayVariantChange}
        />
      </SettingRow>

      <SettingRow
        description={t('settings.design.overlayScreenMode.description')}
        title={t('settings.design.overlayScreenMode.title')}
      >
        <Select
          className={styles.overlaySelect}
          placeholder={t('settings.design.overlayScreenMode.placeholder')}
          value={overlayScreenMode}
          options={[
            {
              label: t('settings.design.overlayScreenMode.all'),
              value: 'all',
            },
            {
              label: t('settings.design.overlayScreenMode.cursor'),
              value: 'cursor',
            },
          ]}
          onChange={onOverlayScreenModeChange}
        />
      </SettingRow>
    </div>
  );
};

export default DesignSettingsTab;
