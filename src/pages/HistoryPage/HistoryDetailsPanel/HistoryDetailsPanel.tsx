import { type FC } from 'react';
import { Button, Card, Space, Tooltip, Typography } from 'antd';
import { ClipboardCopyIcon, FolderOpenIcon, XIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import { useProcessing } from '#/app/processingContext';

import ModelResult from './ModelResult';

import styles from './HistoryDetailsPanel.module.scss';

import type { HistoryRecord } from '#/models/History';

const { Text, Title } = Typography;

interface HistoryDetailsPanelProps {
  onCopyAudioPath: (record: HistoryRecord) => void;
  onCopyPostProcessing: (record: HistoryRecord) => void;
  onCopyTranscription: (record: HistoryRecord) => void;
  onClose: () => void;
  onOpenAudio: (record: HistoryRecord) => void;
  onRepeatPostProcessing: (record: HistoryRecord) => void;
  onRepeatTranscription: (record: HistoryRecord) => void;
  record: HistoryRecord;
}

const HistoryDetailsPanel: FC<HistoryDetailsPanelProps> = ({
  onClose,
  onCopyAudioPath,
  onCopyPostProcessing,
  onCopyTranscription,
  onOpenAudio,
  onRepeatPostProcessing,
  onRepeatTranscription,
  record,
}) => {
  const { t } = useTranslation();
  const { config } = useProcessing();
  const canRepeatTranscription = true;
  const canRepeatPostProcessing =
    config.postProcess.enabled && record.transcription.status === 'success';
  const shouldShowPostProcessing =
    record.postprocessing.status !== 'skipped' || canRepeatPostProcessing;
  const shouldShowPostProcessingBody = record.postprocessing.status !== 'skipped';

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
              onClick={() => {
                onCopyAudioPath(record);
              }}
            />
          </Tooltip>
          <Tooltip title={t('history.details.openInExplorer')}>
            <Button
              aria-label={t('history.details.openInExplorer')}
              icon={<FolderOpenIcon size={16} strokeWidth={2} />}
              size="small"
              onClick={() => {
                onOpenAudio(record);
              }}
            />
          </Tooltip>
        </Space>
      </section>

      <ModelResult
        canCopy={record.transcription.status === 'success'}
        canRepeat={canRepeatTranscription}
        copyLabel={t('history.details.copyTranscription')}
        details={record.transcription}
        onCopy={() => {
          onCopyTranscription(record);
        }}
        onRepeat={() => {
          onRepeatTranscription(record);
        }}
        repeatLabel={t('history.details.repeatTranscription')}
        title={t('history.details.transcription')}
      />
      {shouldShowPostProcessing ? (
        <ModelResult
          canCopy={record.postprocessing.status === 'success'}
          canRepeat={canRepeatPostProcessing}
          copyLabel={t('history.details.copyPostProcessing')}
          details={record.postprocessing}
          showBody={shouldShowPostProcessingBody}
          onCopy={() => {
            onCopyPostProcessing(record);
          }}
          onRepeat={() => {
            onRepeatPostProcessing(record);
          }}
          repeatLabel={t('history.details.repeatPostProcessing')}
          title={t('history.details.postProcessing')}
        />
      ) : undefined}
    </Card>
  );
};

export default HistoryDetailsPanel;
