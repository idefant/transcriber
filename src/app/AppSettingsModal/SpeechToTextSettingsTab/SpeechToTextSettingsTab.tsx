import { type FC } from 'react';

import ProcessingSettingsForm from '../ProcessingSettingsForm';

import styles from './SpeechToTextSettingsTab.module.scss';

import type { ProviderConfig } from '#/models/Provider';

interface SpeechToTextSettingsTabProps {
  providers: ProviderConfig[];
}

const SpeechToTextSettingsTab: FC<SpeechToTextSettingsTabProps> = ({ providers }) => (
  <div className={styles.settingsTab}>
    <ProcessingSettingsForm providers={providers} />
  </div>
);

export default SpeechToTextSettingsTab;
