import { type FC, useState } from 'react';
import { Switch } from 'antd';

import ProcessingSettingsForm from '../ProcessingSettingsForm';
import SettingRow from '../SettingRow';
import type { ProviderConfig } from '../types';

import styles from './PostProcessingSettingsTab.module.scss';

interface PostProcessingSettingsTabProps {
  providers: ProviderConfig[];
}

const PostProcessingSettingsTab: FC<PostProcessingSettingsTabProps> = ({ providers }) => {
  const [isPostProcessingEnabled, setIsPostProcessingEnabled] = useState(false);

  return (
    <div className={styles.settingsTab}>
      <SettingRow
        description="Запускать обработку результата после транскрибации"
        title="Включить постобработку"
      >
        <Switch checked={isPostProcessingEnabled} onChange={setIsPostProcessingEnabled} />
      </SettingRow>

      <ProcessingSettingsForm disabled={!isPostProcessingEnabled} providers={providers} />
    </div>
  );
};

export default PostProcessingSettingsTab;
