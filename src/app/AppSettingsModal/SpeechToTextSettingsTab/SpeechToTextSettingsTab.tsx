import { type FC } from 'react';

import ProcessingSettingsForm from '../ProcessingSettingsForm';
import type { ProviderConfig } from '../types';

import styles from './SpeechToTextSettingsTab.module.scss';

interface SpeechToTextSettingsTabProps {
  providers: ProviderConfig[];
}

const SpeechToTextSettingsTab: FC<SpeechToTextSettingsTabProps> = ({ providers }) => (
  <div className={styles.settingsTab}>
    <ProcessingSettingsForm providers={providers} />
  </div>
);

export default SpeechToTextSettingsTab;
