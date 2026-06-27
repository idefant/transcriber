import { Button, type NotificationArgsProps } from 'antd';
import type { TFunction } from 'i18next';
import { DownloadIcon } from 'lucide-react';

import type { UpdateInfo } from '#/shared/updaterApi';

interface CreateUpdateNotificationArgsInput {
  info: UpdateInfo;
  onDownload: () => void;
  t: TFunction;
}

export const updateNotificationKey = 'update-available';

export const createUpdateNotificationArgs = ({
  info,
  onDownload,
  t,
}: CreateUpdateNotificationArgsInput): NotificationArgsProps => ({
  actions: (
    <Button icon={<DownloadIcon size={14} strokeWidth={2} />} type="primary" onClick={onDownload}>
      {t('settings.about.download')}
    </Button>
  ),
  description: info.notes ?? undefined,
  duration: 10,
  key: updateNotificationKey,
  pauseOnHover: true,
  placement: 'bottomRight',
  showProgress: true,
  title: t('settings.about.updateAvailable', { version: info.version }),
});
