import { type FC, useEffect, useState } from 'react';
import { getVersion } from '@tauri-apps/api/app';
import { listen } from '@tauri-apps/api/event';
import { Button, Progress, Switch, Tag } from 'antd';
import { useTranslation } from 'react-i18next';

import type { UpdateProgress } from '#/shared/updaterApi';
import * as updaterApi from '#/shared/updaterApi';

import SettingRow from '../SettingRow';

import styles from './AboutSettingsTab.module.scss';

import { useAppSettings } from '#/stores';

const isCanary = import.meta.env.VITE_APP_CHANNEL === 'canary';

const AboutSettingsTab: FC = () => {
  const { t } = useTranslation();
  const { settings, updateSettings } = useAppSettings();
  const [version, setVersion] = useState('');
  const [isChecking, setIsChecking] = useState(false);
  const [hasChecked, setHasChecked] = useState(false);
  const [pendingVersion, setPendingVersion] = useState<string | null>(null);
  const [isInstalling, setIsInstalling] = useState(false);
  const [installProgress, setInstallProgress] = useState<UpdateProgress | null>(null);

  useEffect(() => {
    void getVersion().then(setVersion);
  }, []);

  const handleCheckForUpdates = async () => {
    setIsChecking(true);
    setPendingVersion(null);
    try {
      const info = await updaterApi.checkForUpdate(settings.isOfferUnstableVersionsEnabled);
      setPendingVersion(info?.version ?? null);
    } finally {
      setIsChecking(false);
      setHasChecked(true);
    }
  };

  const handleInstall = async () => {
    setIsInstalling(true);
    setInstallProgress(null);
    const unlisten = await listen<UpdateProgress>('updater://progress', (event) => {
      setInstallProgress(event.payload);
    });
    try {
      await updaterApi.downloadAndInstallUpdate();
    } catch {
      setIsInstalling(false);
      setInstallProgress(null);
    } finally {
      unlisten();
    }
  };

  const handleOfferUnstableChange = (value: boolean) => {
    void updateSettings({ isOfferUnstableVersionsEnabled: value });
  };

  const downloadPercent = installProgress?.total
    ? Math.round((installProgress.downloaded / installProgress.total) * 100)
    : undefined;

  return (
    <div className={styles.settingsList}>
      <SettingRow
        description={t('settings.about.version.description')}
        title={t('settings.about.version.title')}
      >
        <div className={styles.versionRow}>
          <span className={styles.version}>{version || '…'}</span>
          {isCanary && <Tag color="gold">{t('settings.about.channel.canary')}</Tag>}
        </div>
      </SettingRow>

      <SettingRow
        description={t('settings.about.offerUnstable.description')}
        title={t('settings.about.offerUnstable.title')}
      >
        <Switch
          checked={settings.isOfferUnstableVersionsEnabled}
          onChange={handleOfferUnstableChange}
        />
      </SettingRow>

      <div className={styles.updateRow}>
        {!isInstalling && (
          <Button loading={isChecking} type="primary" onClick={() => void handleCheckForUpdates()}>
            {t('settings.about.checkForUpdates')}
          </Button>
        )}

        {!isChecking && hasChecked && pendingVersion !== null && !isInstalling && (
          <div className={styles.updateActions}>
            <span className={styles.updateAvailable}>
              {t('settings.about.updateAvailable', { version: pendingVersion })}
            </span>
            <Button type="default" onClick={() => void handleInstall()}>
              {t('settings.about.installUpdate', { version: pendingVersion })}
            </Button>
          </div>
        )}

        {!isChecking && hasChecked && pendingVersion === null && !isInstalling && (
          <span className={styles.noUpdate}>{t('settings.about.noUpdate')}</span>
        )}

        {isInstalling && (
          <div className={styles.installingBlock}>
            <span>{t('settings.about.installing')}</span>
            {downloadPercent !== undefined && <Progress percent={downloadPercent} size="small" />}
          </div>
        )}
      </div>
    </div>
  );
};

export default AboutSettingsTab;
