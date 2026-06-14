import { type FC } from 'react';
import { Button, Card, Space, Tooltip, Typography } from 'antd';
import { ClipboardCopyIcon, FolderOpenIcon, XIcon } from 'lucide-react';

import ModelResult from './ModelResult';

import styles from './HistoryDetailsPanel.module.scss';

import type { HistoryRecord } from '#/models/History';

const { Text, Title } = Typography;

interface HistoryDetailsPanelProps {
  onClose: () => void;
  record: HistoryRecord;
}

const HistoryDetailsPanel: FC<HistoryDetailsPanelProps> = ({ onClose, record }) => (
  <Card className={styles.panel}>
    <div className={styles.header}>
      <Title className={styles.title} level={5}>
        Детали записи
      </Title>
      <Tooltip title="Закрыть панель">
        <Button
          aria-label="Закрыть панель"
          icon={<XIcon size={18} strokeWidth={2} />}
          type="text"
          onClick={onClose}
        />
      </Tooltip>
    </div>

    <section className={styles.audioSection}>
      <Title className={styles.sectionTitle} level={5}>
        Аудио
      </Title>
      <Text className={styles.audioDuration}>{record.audio.duration}</Text>
      <Space className={styles.audioActions} size={4}>
        <Tooltip title="Скопировать путь">
          <Button
            aria-label="Скопировать путь"
            icon={<ClipboardCopyIcon size={16} strokeWidth={2} />}
            size="small"
          />
        </Tooltip>
        <Tooltip title="Открыть в проводнике">
          <Button
            aria-label="Открыть в проводнике"
            icon={<FolderOpenIcon size={16} strokeWidth={2} />}
            size="small"
          />
        </Tooltip>
      </Space>
    </section>

    <ModelResult
      copyLabel="Скопировать транскрибацию"
      details={record.transcription}
      repeatLabel="Повторить транскрибацию"
      title="Транскрибация"
    />
    <ModelResult
      copyLabel="Скопировать постобработку"
      details={record.postprocessing}
      repeatLabel="Повторить постобработку"
      title="Постобработка"
    />
  </Card>
);

export default HistoryDetailsPanel;
