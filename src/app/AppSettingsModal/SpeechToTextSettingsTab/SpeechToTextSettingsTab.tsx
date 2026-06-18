import { type FC } from 'react';

import ProcessingSettingsForm from '../ProcessingSettingsForm';

import SttTestPanel from './SttTestPanel';

import styles from './SpeechToTextSettingsTab.module.scss';

const SpeechToTextSettingsTab: FC = () => (
  <div className={styles.settingsTab}>
    <ProcessingSettingsForm task="stt" />
    <SttTestPanel />
  </div>
);

export default SpeechToTextSettingsTab;
