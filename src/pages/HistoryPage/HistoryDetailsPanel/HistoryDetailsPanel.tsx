import { type FC } from 'react';
import { Button, Card, Space, Tooltip, Typography } from 'antd';
import { ClipboardCopyIcon, FolderOpenIcon, XIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import ModelResult from './ModelResult';

import styles from './HistoryDetailsPanel.module.scss';

import type { HistoryRecord } from '#/models/History';

const { Text, Title } = Typography;

interface HistoryDetailsPanelProps {
  onClose: () => void;
  record: HistoryRecord;
}

const HistoryDetailsPanel: FC<HistoryDetailsPanelProps> = ({ onClose, record }) => {
  const { t } = useTranslation();

  return (
    <Card className={styles.panel}>
      <div className={styles.header}>
        <Title className={styles.title} level={5}>
          {t('history.details.title')}
        </Title>
        <Tooltip title={t('history.details.close')}>
          <Button
            aria-label={t('history.details.close')}
            icon={<XIcon size={18} strokeWidth={2} />}
            type="text"
            onClick={onClose}
          />
        </Tooltip>
      </div>

      <section className={styles.audioSection}>
        <Title className={styles.sectionTitle} level={5}>
          {t('history.details.audio')}
        </Title>
        <Text className={styles.audioDuration}>{record.audio.duration}</Text>
        <Space className={styles.audioActions} size={4}>
          <Tooltip title={t('history.details.copyPath')}>
            <Button
              aria-label={t('history.details.copyPath')}
              icon={<ClipboardCopyIcon size={16} strokeWidth={2} />}
              size="small"
            />
          </Tooltip>
          <Tooltip title={t('history.details.openInExplorer')}>
            <Button
              aria-label={t('history.details.openInExplorer')}
              icon={<FolderOpenIcon size={16} strokeWidth={2} />}
              size="small"
            />
          </Tooltip>
        </Space>
      </section>

      <ModelResult
        copyLabel={t('history.details.copyTranscription')}
        details={record.transcription}
        repeatLabel={t('history.details.repeatTranscription')}
        title={t('history.details.transcription')}
      />
      <ModelResult
        copyLabel={t('history.details.copyPostProcessing')}
        details={record.postprocessing}
        repeatLabel={t('history.details.repeatPostProcessing')}
        title={t('history.details.postProcessing')}
      />
    </Card>
  );
};

export default HistoryDetailsPanel;
