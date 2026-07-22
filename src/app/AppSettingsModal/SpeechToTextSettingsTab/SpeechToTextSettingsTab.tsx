import { type FC } from 'react';
import { useTranslation } from 'react-i18next';

import ProcessingSettingsForm from '../ProcessingSettingsForm';
import SettingsSection from '../SettingsSection';

import SttTestPanel from './SttTestPanel';

import styles from './SpeechToTextSettingsTab.module.scss';

const SpeechToTextSettingsTab: FC = () => {
  const { t } = useTranslation();

  return (
    <div className={styles.sectionList}>
      <ProcessingSettingsForm task="stt" />

      <SettingsSection compact title={t('settings.tests.title')}>
        <SttTestPanel />
      </SettingsSection>
    </div>
  );
};

export default SpeechToTextSettingsTab;
