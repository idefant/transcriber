import { type FC } from 'react';
import { Typography } from 'antd';

import styles from './SpeechToTextSettingsTab.module.scss';

const SpeechToTextSettingsTab: FC = () => (
  <div className={styles.settingsTab}>
    <Typography.Paragraph>
      Настройки Speech-to-Text появятся здесь после выбора сценариев распознавания.
    </Typography.Paragraph>
  </div>
);

export default SpeechToTextSettingsTab;
