import { type FC } from 'react';
import { Typography } from 'antd';

import styles from './PostProcessingSettingsTab.module.scss';

const PostProcessingSettingsTab: FC = () => (
  <div className={styles.settingsTab}>
    <Typography.Paragraph>
      Настройки постобработки появятся здесь после подключения первого сценария.
    </Typography.Paragraph>
  </div>
);

export default PostProcessingSettingsTab;
