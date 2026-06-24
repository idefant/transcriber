import { type FC, type KeyboardEvent, type MouseEvent } from 'react';
import { Button, Collapse, Space, Tooltip } from 'antd';
import { CopyIcon, LoaderCircleIcon, RotateCcwIcon, Trash2Icon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import styles from './HistoryRecordsList.module.scss';

import type { HistoryGroup, HistoryRecord } from '#/models/History';

interface HistoryRecordsListProps {
  activeDate?: string;
  groups: HistoryGroup[];
  onActiveDateChange: (date: string | null) => void;
  onCopyRecordText: (record: HistoryRecord) => void;
  onDeleteRecord: (record: HistoryRecord) => void;
  onRecordSelect: (record: HistoryRecord) => void;
  onRepeatTranscription: (record: HistoryRecord) => void;
  processingRecordId?: string;
  selectedRecordId?: string;
}

const stopRecordActionClick = (event: MouseEvent<HTMLElement>) => {
  event.stopPropagation();
};

const hasDisplayText = (record: HistoryRecord) =>
  record.postprocessing.status === 'success' || record.transcription.status === 'success';

const getDisplayText = (record: HistoryRecord) => {
  if (record.postprocessing.status === 'success') {
    return record.postprocessing.text;
  }

  if (record.transcription.status === 'success') {
    return record.transcription.text;
  }

  return record.transcription.errorMessage ?? '';
};

const HistoryRecordsList: FC<HistoryRecordsListProps> = ({
  activeDate,
  groups,
  onActiveDateChange,
  onCopyRecordText,
  onDeleteRecord,
  onRecordSelect,
  onRepeatTranscription,
  processingRecordId,
  selectedRecordId,
}) => {
  const { t } = useTranslation();
  const collapseItems = groups.map((group) => ({
    children: (
      <div className={styles.records}>
        {group.records.map((record) => {
          const displayText = getDisplayText(record);
          const canCopy = hasDisplayText(record);
          const isProcessing =
            record.transcription.isProcessing ||
            record.postprocessing.isProcessing ||
            processingRecordId === record.id;

          return (
            <div
              className={record.id === selectedRecordId ? styles.recordActive : styles.record}
              key={record.id}
              role="button"
              tabIndex={0}
              onClick={() => {
                onRecordSelect(record);
              }}
              onKeyDown={(event: KeyboardEvent<HTMLDivElement>) => {
                if (event.key === 'Enter' || event.key === ' ') {
                  event.preventDefault();
                  onRecordSelect(record);
                }
              }}
            >
              <span className={styles.recordContent}>
                <span className={styles.recordTime}>{record.time}</span>
                <span
                  className={
                    record.transcription.status === 'error' ? styles.recordError : styles.recordText
                  }
                >
                  {displayText}
                </span>
              </span>
              <Space className={styles.recordActions} size={4}>
                <Tooltip title={t('history.records.copyText')}>
                  <Button
                    aria-label={t('history.records.copyText')}
                    icon={<CopyIcon size={16} strokeWidth={2} />}
                    size="small"
                    type="text"
                    disabled={!canCopy}
                    onClick={(event) => {
                      stopRecordActionClick(event);
                      onCopyRecordText(record);
                    }}
                  />
                </Tooltip>
                <Tooltip title={t('history.records.repeat')}>
                  <Button
                    aria-label={t('history.records.repeat')}
                    icon={
                      isProcessing ? (
                        <LoaderCircleIcon className={styles.spinIcon} size={16} strokeWidth={2} />
                      ) : (
                        <RotateCcwIcon size={16} strokeWidth={2} />
                      )
                    }
                    disabled={isProcessing}
                    size="small"
                    type="text"
                    onClick={(event) => {
                      stopRecordActionClick(event);
                      onRepeatTranscription(record);
                    }}
                  />
                </Tooltip>
                <Tooltip title={t('history.records.delete')}>
                  <Button
                    aria-label={t('history.records.delete')}
                    danger
                    icon={<Trash2Icon size={16} strokeWidth={2} />}
                    size="small"
                    type="text"
                    onClick={(event) => {
                      stopRecordActionClick(event);
                      onDeleteRecord(record);
                    }}
                  />
                </Tooltip>
              </Space>
            </div>
          );
        })}
      </div>
    ),
    key: group.date,
    label: group.label,
  }));

  return (
    <Collapse
      accordion
      activeKey={activeDate}
      items={collapseItems}
      onChange={(key) => {
        const date = Array.isArray(key) ? key.at(0) : key;
        onActiveDateChange(date === '' ? null : (date ?? null));
      }}
    />
  );
};

export default HistoryRecordsList;
