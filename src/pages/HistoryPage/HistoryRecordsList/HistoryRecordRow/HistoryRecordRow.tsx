import { type FC, type KeyboardEvent, memo, type MouseEvent } from 'react';
import { Button, Space, Tooltip } from 'antd';
import { CopyIcon, LoaderCircleIcon, RotateCcwIcon, Trash2Icon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import HighlightedText from '#/ui/HighlightedText';

import styles from './HistoryRecordRow.module.scss';

import type { HistoryRecord } from '#/models/History';

interface HistoryRecordRowProps {
  /** Подсвечиваемая подстрока в тексте записи. Пусто — текст без подсветки. */
  highlightQuery?: string;
  /** Запись выбрана и открыта в панели деталей. */
  isActive: boolean;
  /** По записи идёт транскрипция, постобработка или их повтор. */
  isProcessing: boolean;
  record: HistoryRecord;
  onCopyText: (record: HistoryRecord) => void;
  onDelete: (record: HistoryRecord) => void;
  onRepeat: (record: HistoryRecord) => void;
  onSelect: (record: HistoryRecord) => void;
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

/**
 * Одна строка списка истории. Обёрнута в {@link memo}, поэтому смена выбранной
 * записи или флага обработки перерисовывает только затронутые строки, а не весь
 * список. Для этого все колбэки (`onSelect`, `onCopyText`, `onDelete`,
 * `onRepeat`) должны приходить со стабильной ссылкой из `HistoryPage` — иначе
 * мемоизация не сработает.
 */
const HistoryRecordRow: FC<HistoryRecordRowProps> = ({
  highlightQuery,
  isActive,
  isProcessing,
  record,
  onCopyText,
  onDelete,
  onRepeat,
  onSelect,
}) => {
  const { t } = useTranslation();
  const displayText = getDisplayText(record);
  const canCopy = hasDisplayText(record);

  return (
    <div
      className={isActive ? styles.recordActive : styles.record}
      role="button"
      tabIndex={0}
      onClick={() => {
        onSelect(record);
      }}
      onKeyDown={(event: KeyboardEvent<HTMLDivElement>) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          onSelect(record);
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
          <HighlightedText query={highlightQuery} text={displayText} />
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
              onCopyText(record);
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
              onRepeat(record);
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
              onDelete(record);
            }}
          />
        </Tooltip>
      </Space>
    </div>
  );
};

export default memo(HistoryRecordRow);
