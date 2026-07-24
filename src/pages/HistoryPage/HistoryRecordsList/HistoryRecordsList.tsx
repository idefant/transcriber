import { type FC, memo } from 'react';
import { Collapse } from 'antd';

import HistoryRecordRow from './HistoryRecordRow';

import styles from './HistoryRecordsList.module.scss';

import type { HistoryGroup, HistoryRecord } from '#/models/History';

interface HistoryRecordsListProps {
  activeDate?: string;
  groups: HistoryGroup[];
  /** Подстрока, подсвечиваемая в тексте записей. */
  highlightQuery?: string;
  /** В режиме поиска все группы раскрыты и свернуть их нельзя. */
  isSearchMode?: boolean;
  onActiveDateChange: (date: string | null) => void;
  onCopyRecordText: (record: HistoryRecord) => void;
  onDeleteRecord: (record: HistoryRecord) => void;
  onRecordSelect: (record: HistoryRecord) => void;
  onRepeatTranscription: (record: HistoryRecord) => void;
  processingRecordId?: string;
  selectedRecordId?: string;
}

const HistoryRecordsList: FC<HistoryRecordsListProps> = ({
  activeDate,
  groups,
  highlightQuery,
  isSearchMode = false,
  onActiveDateChange,
  onCopyRecordText,
  onDeleteRecord,
  onRecordSelect,
  onRepeatTranscription,
  processingRecordId,
  selectedRecordId,
}) => {
  const collapseItems = groups.map((group) => ({
    children: (
      <div className={styles.records}>
        {group.records.map((record) => (
          <HistoryRecordRow
            key={record.id}
            highlightQuery={highlightQuery}
            isActive={record.id === selectedRecordId}
            isProcessing={
              record.transcription.isProcessing ||
              record.postprocessing.isProcessing ||
              processingRecordId === record.id
            }
            record={record}
            onCopyText={onCopyRecordText}
            onDelete={onDeleteRecord}
            onRepeat={onRepeatTranscription}
            onSelect={onRecordSelect}
          />
        ))}
      </div>
    ),
    key: group.date,
    label: group.label,
    showArrow: !isSearchMode,
  }));

  // В режиме поиска раскрыты все группы. `collapsible="icon"` разрешает сворачивание
  // только кликом по стрелке, а стрелка здесь скрыта — значит свернуть группу нельзя.
  // Вариант `"disabled"` не подошёл бы: он красит заголовок в неактивный цвет.
  return (
    <Collapse
      accordion={!isSearchMode}
      activeKey={isSearchMode ? groups.map((group) => group.date) : activeDate}
      collapsible={isSearchMode ? 'icon' : undefined}
      items={collapseItems}
      onChange={(key) => {
        if (isSearchMode) {
          return;
        }

        const date = Array.isArray(key) ? key.at(0) : key;
        onActiveDateChange(date === '' ? null : (date ?? null));
      }}
    />
  );
};

export default memo(HistoryRecordsList);
