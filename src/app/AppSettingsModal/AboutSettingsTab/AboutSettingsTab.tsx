import { type FC, useEffect, useState } from 'react';
import { getVersion } from '@tauri-apps/api/app';
import { Button, message, Progress, Switch, Tag } from 'antd';
import { DownloadIcon, RefreshCwIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import ResetAppDataButton from '#/app/ResetAppDataButton';
import { triggerRustTelemetryFailure } from '#/shared/telemetry';

import SettingRow from '../SettingRow';

import ReleaseNotes from './ReleaseNotes';
import TelemetryReactCrash from './TelemetryReactCrash';

import styles from './AboutSettingsTab.module.scss';

import { useAppSettings, useUpdaterStore } from '#/stores';

const isCanary = import.meta.env.VITE_APP_CHANNEL === 'canary';
const getErrorMessage = (error: unknown) =>
  error instanceof Error ? error.message : String(error);

const triggerRustTelemetryFailureSafely = () => {
  void triggerRustTelemetryFailure().catch(() => {});
};

const AboutSettingsTab: FC = () => {
  const { t } = useTranslation();
  const [messageApi, messageContextHolder] = message.useMessage();
  const { settings, updateSettings } = useAppSettings();
  const availableUpdate = useUpdaterStore((s) => s.availableUpdate);
  const checkForUpdates = useUpdaterStore((s) => s.checkForUpdates);
  const downloadAndInstall = useUpdaterStore((s) => s.downloadAndInstall);
  const installProgress = useUpdaterStore((s) => s.installProgress);
  const isChecking = useUpdaterStore((s) => s.isChecking);
  const isInstalling = useUpdaterStore((s) => s.isInstalling);
  const lastCheckedAt = useUpdaterStore((s) => s.lastCheckedAt);
  const [version, setVersion] = useState('');
  const [isReactTelemetryCrashActive, setIsReactTelemetryCrashActive] = useState(false);

  useEffect(() => {
    void getVersion().then(setVersion);
  }, []);

  useEffect(() => {
    void checkForUpdates(settings.isOfferUnstableVersionsEnabled).catch((error: unknown) => {
      void messageApi.error(getErrorMessage(error));
    });
  }, [checkForUpdates, messageApi, settings.isOfferUnstableVersionsEnabled]);

  const handleCheckForUpdates = async () => {
    try {
      await checkForUpdates(settings.isOfferUnstableVersionsEnabled);
    } catch (error) {
      void messageApi.error(getErrorMessage(error));
    }
  };

  const handleInstall = async () => {
    try {
      await downloadAndInstall();
    } catch (error) {
      void messageApi.error(getErrorMessage(error));
    }
  };

  const handleUpdateNotificationsChange = (value: boolean) => {
    void updateSettings({ isUpdateNotificationsEnabled: value });
  };

  const handleOfferUnstableChange = (value: boolean) => {
    void updateSettings({ isOfferUnstableVersionsEnabled: value });
  };

  const downloadPercent = installProgress?.total
    ? Math.round((installProgress.downloaded / installProgress.total) * 100)
    : undefined;
  const releaseNotes = availableUpdate?.notes?.trim() ?? '';

  return (
    <>
      {messageContextHolder}
      <div className={styles.settingsList}>
        <SettingRow
          description={t('settings.about.version.description')}
          title={t('settings.about.version.title')}
        >
          <div className={styles.versionRow}>
            <span className={styles.version}>{version || '…'}</span>
            {isCanary && (
              <Tag color="gold" variant="outlined">
                {t('settings.about.channel.canary')}
              </Tag>
            )}
          </div>
        </SettingRow>

        <SettingRow
          description={t('settings.about.updateNotifications.description')}
          title={t('settings.about.updateNotifications.title')}
        >
          <Switch
            checked={settings.isUpdateNotificationsEnabled}
            onChange={handleUpdateNotificationsChange}
          />
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
            <Button
              loading={isChecking}
              icon={<RefreshCwIcon size={14} strokeWidth={2} />}
              onClick={() => void handleCheckForUpdates()}
            >
              {t('settings.about.checkForUpdates')}
            </Button>
          )}

          {availableUpdate !== null && !isInstalling && (
            <Button
              color="green"
              variant="solid"
              icon={<DownloadIcon size={14} strokeWidth={2} />}
              onClick={() => void handleInstall()}
            >
              {t('settings.about.installUpdate', { version: availableUpdate.version })}
            </Button>
          )}

          {lastCheckedAt !== null && availableUpdate === null && !isChecking && !isInstalling && (
            <span className={styles.noUpdate}>{t('settings.about.noUpdate')}</span>
          )}

          {isInstalling && (
            <div className={styles.installingBlock}>
              <span>{t('settings.about.installing')}</span>
              {downloadPercent !== undefined && <Progress percent={downloadPercent} size="small" />}
            </div>
          )}

          {availableUpdate !== null && releaseNotes.length > 0 && (
            <ReleaseNotes notes={releaseNotes} version={availableUpdate.version} />
          )}
        </div>

        <SettingRow
          description={t('maintenance.reset.aboutDescription')}
          title={t('maintenance.reset.aboutTitle')}
        >
          <ResetAppDataButton />
        </SettingRow>

        <SettingRow
          description={t('settings.about.telemetryTest.react.description')}
          title={t('settings.about.telemetryTest.react.title')}
        >
          <Button
            danger
            onClick={() => {
              setIsReactTelemetryCrashActive(true);
            }}
          >
            {t('settings.about.telemetryTest.react.action')}
          </Button>
        </SettingRow>

        <SettingRow
          description={t('settings.about.telemetryTest.rust.description')}
          title={t('settings.about.telemetryTest.rust.title')}
        >
          <Button danger onClick={triggerRustTelemetryFailureSafely}>
            {t('settings.about.telemetryTest.rust.action')}
          </Button>
        </SettingRow>
      </div>
      <TelemetryReactCrash isActive={isReactTelemetryCrashActive} />
    </>
  );
};

export default AboutSettingsTab;
