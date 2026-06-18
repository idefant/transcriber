import { type FC } from 'react';
import { Switch } from 'antd';

import { useProcessing } from '#/app/processingContext';

import ProcessingSettingsForm from '../ProcessingSettingsForm';
import SettingRow from '../SettingRow';

import PostProcessTestPanel from './PostProcessTestPanel';

import styles from './PostProcessingSettingsTab.module.scss';

const PostProcessingSettingsTab: FC = () => {
  const { config, updatePostProcessConfig } = useProcessing();

  return (
    <div className={styles.settingsTab}>
      <SettingRow
        description="Запускать обработку результата после транскрибации"
        title="Включить постобработку"
      >
        <Switch
          checked={config.postProcess.enabled}
          onChange={(enabled) => {
            void updatePostProcessConfig({ enabled });
          }}
        />
      </SettingRow>

      <ProcessingSettingsForm disabled={!config.postProcess.enabled} task="postProcess" />
      <PostProcessTestPanel />
    </div>
  );
};

export default PostProcessingSettingsTab;
