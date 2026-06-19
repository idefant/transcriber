import { type FC } from 'react';
import { Switch } from 'antd';
import { useTranslation } from 'react-i18next';

import { useProcessing } from '#/app/processingContext';

import ProcessingSettingsForm from '../ProcessingSettingsForm';
import SettingRow from '../SettingRow';

import PostProcessTestPanel from './PostProcessTestPanel';

import styles from './PostProcessingSettingsTab.module.scss';

const PostProcessingSettingsTab: FC = () => {
  const { config, updatePostProcessConfig } = useProcessing();
  const { t } = useTranslation();

  return (
    <div className={styles.settingsTab}>
      <SettingRow
        description={t('settings.postProcessing.enabled.description')}
        title={t('settings.postProcessing.enabled.title')}
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
