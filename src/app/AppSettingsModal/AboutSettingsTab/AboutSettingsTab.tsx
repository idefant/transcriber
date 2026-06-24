import { type FC, useEffect, useState } from 'react';
import { getVersion } from '@tauri-apps/api/app';
import { useTranslation } from 'react-i18next';

import SettingRow from '../SettingRow';

import styles from './AboutSettingsTab.module.scss';

const AboutSettingsTab: FC = () => {
  const { t } = useTranslation();
  const [version, setVersion] = useState('');

  useEffect(() => {
    void getVersion().then(setVersion);
  }, []);

  return (
    <div className={styles.settingsList}>
      <SettingRow
        description={t('settings.about.version.description')}
        title={t('settings.about.version.title')}
      >
        <span className={styles.version}>{version || '…'}</span>
      </SettingRow>
    </div>
  );
};

export default AboutSettingsTab;
