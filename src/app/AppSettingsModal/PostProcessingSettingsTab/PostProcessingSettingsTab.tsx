import { type FC } from 'react';
import { Switch } from 'antd';
import { useTranslation } from 'react-i18next';

import ProcessingSettingsForm from '../ProcessingSettingsForm';
import SettingRow from '../SettingRow';
import SettingsSection from '../SettingsSection';

import PostProcessTestPanel from './PostProcessTestPanel';

import styles from './PostProcessingSettingsTab.module.scss';

import { useProcessing } from '#/stores';

const PostProcessingSettingsTab: FC = () => {
  const { config, updatePostProcessConfig } = useProcessing();
  const { t } = useTranslation();

  return (
    <div className={styles.sectionList}>
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

      {config.postProcess.enabled && (
        <>
          <ProcessingSettingsForm task="postProcess" />

          <SettingsSection compact title={t('settings.tests.title')}>
            <PostProcessTestPanel />
          </SettingsSection>
        </>
      )}
    </div>
  );
};

export default PostProcessingSettingsTab;
