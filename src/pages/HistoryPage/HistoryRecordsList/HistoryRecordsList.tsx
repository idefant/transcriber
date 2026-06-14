import { type FC, type KeyboardEvent, type MouseEvent } from 'react';
import { Button, Collapse, Space, Tooltip } from 'antd';
import { CopyIcon, LoaderCircleIcon, RotateCcwIcon, Trash2Icon } from 'lucide-react';

import styles from './HistoryRecordsList.module.scss';

import type { HistoryGroup, HistoryRecord } from '#/models/History';

interface HistoryRecordsListProps {
  activeDate?: string;
  groups: HistoryGroup[];
  onActiveDateChange: (date?: string) => void;
  onRecordSelect: (record: HistoryRecord) => void;
  selectedRecordId?: string;
}

const stopRecordActionClick = (event: MouseEvent<HTMLElement>) => {
  event.stopPropagation();
};

const HistoryRecordsList: FC<HistoryRecordsListProps> = ({
  activeDate,
  groups,
  onActiveDateChange,
  onRecordSelect,
  selectedRecordId,
}) => {
  const collapseItems = groups.map((group) => ({
    children: (
      <div className={styles.records}>
        {group.records.map((record) => (
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
              <span className={styles.recordText}>{record.transcription.text}</span>
            </span>
            <Space className={styles.recordActions} size={4}>
              <Tooltip title="Скопировать текст">
                <Button
                  aria-label="Скопировать текст"
                  icon={<CopyIcon size={16} strokeWidth={2} />}
                  size="small"
                  type="text"
                  onClick={stopRecordActionClick}
                />
              </Tooltip>
              <Tooltip title="Повторить">
                <Button
                  aria-label="Повторить"
                  icon={
                    record.transcription.isProcessing ? (
                      <LoaderCircleIcon className={styles.spinIcon} size={16} strokeWidth={2} />
                    ) : (
                      <RotateCcwIcon size={16} strokeWidth={2} />
                    )
                  }
                  disabled={record.transcription.isProcessing}
                  size="small"
                  type="text"
                  onClick={stopRecordActionClick}
                />
              </Tooltip>
              <Tooltip title="Удалить">
                <Button
                  aria-label="Удалить"
                  danger
                  icon={<Trash2Icon size={16} strokeWidth={2} />}
                  size="small"
                  type="text"
                  onClick={stopRecordActionClick}
                />
              </Tooltip>
            </Space>
          </div>
        ))}
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
        onActiveDateChange(Array.isArray(key) ? key[0] : key);
      }}
    />
  );
};

export default HistoryRecordsList;
