import { type FC, type KeyboardEvent, type MouseEvent, useMemo, useState } from 'react';
import { Button, Card, Collapse, DatePicker, Empty, Space, Tooltip, Typography } from 'antd';
import dayjs from 'dayjs';
import {
  ChevronLeftIcon,
  ChevronRightIcon,
  ClipboardCopyIcon,
  CopyIcon,
  FolderOpenIcon,
  LoaderCircleIcon,
  RotateCcwIcon,
  Trash2Icon,
  XIcon,
} from 'lucide-react';

import type { HistoryGroup, HistoryRecord } from './types';
import type { ProcessingDetails } from './types';

import styles from './HistoryPage.module.scss';

const { Paragraph, Text, Title } = Typography;
const monthFormat = 'YYYY-MM';

const historyGroups: HistoryGroup[] = [
  {
    date: '2026-06-13',
    label: '13 июня 2026',
    month: '2026-06',
    records: [
      {
        id: '13-1042',
        audio: {
          duration: '00:02:34',
          path: String.raw`C:\Users\inik3\Recordings\meeting-2026-06-13-1042.wav`,
        },
        postprocessing: {
          cost: '$0.0041',
          duration: '1.8 сек',
          isProcessing: false,
          model: 'gpt-4.1-mini',
          provider: 'OpenAI',
          text: 'Итоговая версия заметки: согласовать дизайн панели истории и подготовить моковые состояния.',
        },
        time: '10:42',
        transcription: {
          cost: '$0.0126',
          duration: '6.4 сек',
          isProcessing: true,
          model: 'whisper-large-v3',
          provider: 'Groq',
          text: 'Нужно согласовать внешний вид панели истории и подготовить несколько моковых состояний.',
        },
      },
      {
        id: '13-0915',
        audio: {
          duration: '00:00:58',
          path: String.raw`C:\Users\inik3\Recordings\quick-note-2026-06-13-0915.wav`,
        },
        postprocessing: {
          cost: '$0.0013',
          duration: '0.9 сек',
          isProcessing: false,
          model: 'gpt-4.1-mini',
          provider: 'OpenAI',
          text: 'Добавить проверку состояния кнопки повтора во время обработки.',
        },
        time: '09:15',
        transcription: {
          cost: '$0.0038',
          duration: '2.1 сек',
          isProcessing: false,
          model: 'whisper-1',
          provider: 'OpenAI',
          text: 'Добавить проверку состояния кнопки повтора во время обработки.',
        },
      },
    ],
  },
  {
    date: '2026-06-12',
    label: '12 июня 2026',
    month: '2026-06',
    records: [
      {
        id: '12-1810',
        audio: {
          duration: '00:04:17',
          path: String.raw`C:\Users\inik3\Recordings\planning-2026-06-12-1810.wav`,
        },
        postprocessing: {
          cost: '$0.0068',
          duration: '2.5 сек',
          isProcessing: false,
          model: 'gpt-4.1-mini',
          provider: 'OpenRouter',
          text: 'План: собрать страницу истории, правую панель деталей и моковые действия для элементов списка.',
        },
        time: '18:10',
        transcription: {
          cost: '$0.0184',
          duration: '8.7 сек',
          isProcessing: false,
          model: 'whisper-large-v3',
          provider: 'Groq',
          text: 'Собрать страницу истории, правую панель деталей и моковые действия для элементов списка.',
        },
      },
    ],
  },
  {
    date: '2026-05-30',
    label: '30 мая 2026',
    month: '2026-05',
    records: [
      {
        id: '30-1428',
        audio: {
          duration: '00:01:46',
          path: String.raw`C:\Users\inik3\Recordings\dictionary-2026-05-30-1428.wav`,
        },
        postprocessing: {
          cost: '$0.0029',
          duration: '1.2 сек',
          isProcessing: false,
          model: 'gpt-4.1-mini',
          provider: 'OpenAI',
          text: 'Словарь должен добавлять слова по Enter и через кнопку рядом с полем ввода.',
        },
        time: '14:28',
        transcription: {
          cost: '$0.0072',
          duration: '3.9 сек',
          isProcessing: false,
          model: 'whisper-1',
          provider: 'OpenAI',
          text: 'Словарь должен добавлять слова по Enter и через кнопку рядом с полем ввода.',
        },
      },
    ],
  },
];

const getCurrentMonth = () => dayjs().format(monthFormat);

const getFirstGroupByMonth = (month: string) =>
  historyGroups.find((group) => group.month === month);

const shiftMonth = (month: string, monthOffset: number) =>
  dayjs(`${month}-01`).add(monthOffset, 'month').format(monthFormat);

const stopRecordActionClick = (event: MouseEvent<HTMLElement>) => {
  event.stopPropagation();
};

const HistoryPage: FC = () => {
  const [selectedMonth, setSelectedMonth] = useState(getCurrentMonth);
  const [activeDate, setActiveDate] = useState(getFirstGroupByMonth(selectedMonth)?.date);
  const [selectedRecord, setSelectedRecord] = useState<HistoryRecord>();

  const monthPickerValue = useMemo(() => dayjs(`${selectedMonth}-01`), [selectedMonth]);

  const filteredGroups = useMemo(() => {
    return historyGroups.filter((group) => group.month === selectedMonth);
  }, [selectedMonth]);

  const collapseItems = filteredGroups.map((group) => ({
    children: (
      <div className={styles.records}>
        {group.records.map((record) => (
          <div
            className={record.id === selectedRecord?.id ? styles.recordActive : styles.record}
            key={record.id}
            role="button"
            tabIndex={0}
            onClick={() => {
              setSelectedRecord(record);
            }}
            onKeyDown={(event: KeyboardEvent<HTMLDivElement>) => {
              if (event.key === 'Enter' || event.key === ' ') {
                event.preventDefault();
                setSelectedRecord(record);
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

  const setMonth = (month: string) => {
    const firstGroup = getFirstGroupByMonth(month);
    setSelectedMonth(month);
    setActiveDate(firstGroup?.date);
    setSelectedRecord(undefined);
  };

  const handleMonthChange = (_: unknown, dateString: string | null) => {
    if (dateString === null || dateString.length === 0) {
      return;
    }

    setMonth(dateString);
  };

  const goToPreviousMonth = () => {
    setMonth(shiftMonth(selectedMonth, -1));
  };

  const goToNextMonth = () => {
    setMonth(shiftMonth(selectedMonth, 1));
  };

  const goToCurrentMonth = () => {
    setMonth(getCurrentMonth());
  };

  const renderProcessingSection = (
    title: string,
    details: ProcessingDetails,
    repeatLabel: string,
    copyLabel: string,
  ) => (
    <section className={styles.detailsSection}>
      <div className={styles.sectionHeader}>
        <Title className={styles.sectionTitle} level={5}>
          {title}
        </Title>
        <Space size={4}>
          <Tooltip title={copyLabel}>
            <Button
              aria-label={copyLabel}
              icon={<CopyIcon size={16} strokeWidth={2} />}
              size="small"
            />
          </Tooltip>
          <Tooltip title={repeatLabel}>
            <Button
              aria-label={repeatLabel}
              icon={
                details.isProcessing ? (
                  <LoaderCircleIcon className={styles.spinIcon} size={16} strokeWidth={2} />
                ) : (
                  <RotateCcwIcon size={16} strokeWidth={2} />
                )
              }
              disabled={details.isProcessing}
              size="small"
            />
          </Tooltip>
        </Space>
      </div>
      <dl className={styles.metaList}>
        <div>
          <dt>Провайдер</dt>
          <dd>{details.provider}</dd>
        </div>
        <div>
          <dt>Модель</dt>
          <dd>{details.model}</dd>
        </div>
        <div>
          <dt>Время</dt>
          <dd>{details.duration}</dd>
        </div>
        <div>
          <dt>Стоимость</dt>
          <dd>{details.cost}</dd>
        </div>
      </dl>
      <Paragraph className={styles.transcriptText}>{details.text}</Paragraph>
    </section>
  );

  return (
    <div className={styles.page}>
      <Card className={styles.historyCard}>
        <div className={styles.toolbar}>
          <Space.Compact>
            <Tooltip title="Предыдущий месяц">
              <Button
                aria-label="Предыдущий месяц"
                icon={<ChevronLeftIcon size={16} strokeWidth={2} />}
                onClick={goToPreviousMonth}
              />
            </Tooltip>
            <DatePicker
              allowClear={false}
              className={styles.monthPicker}
              format={monthFormat}
              picker="month"
              placeholder="Месяц"
              value={monthPickerValue}
              renderExtraFooter={() => (
                <Button block size="small" type="text" onClick={goToCurrentMonth}>
                  Сегодня
                </Button>
              )}
              onChange={handleMonthChange}
            />
            <Tooltip title="Следующий месяц">
              <Button
                aria-label="Следующий месяц"
                icon={<ChevronRightIcon size={16} strokeWidth={2} />}
                onClick={goToNextMonth}
              />
            </Tooltip>
          </Space.Compact>
        </div>

        {collapseItems.length > 0 ? (
          <Collapse
            accordion
            activeKey={activeDate}
            items={collapseItems}
            onChange={(key) => {
              setActiveDate(Array.isArray(key) ? key[0] : key);
            }}
          />
        ) : (
          <Empty description="За выбранный месяц записей нет" />
        )}
      </Card>

      <aside className={styles.detailsSlot}>
        {selectedRecord === undefined ? undefined : (
          <Card className={styles.detailsPanel}>
            <div className={styles.detailsHeader}>
              <Title className={styles.detailsTitle} level={5}>
                Детали записи
              </Title>
              <Tooltip title="Закрыть панель">
                <Button
                  aria-label="Закрыть панель"
                  icon={<XIcon size={18} strokeWidth={2} />}
                  type="text"
                  onClick={() => {
                    setSelectedRecord(undefined);
                  }}
                />
              </Tooltip>
            </div>

            <section className={styles.audioSection}>
              <Title className={styles.sectionTitle} level={5}>
                Аудио
              </Title>
              <Text className={styles.audioDuration}>{selectedRecord.audio.duration}</Text>
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

            {renderProcessingSection(
              'Транскрибация',
              selectedRecord.transcription,
              'Повторить транскрибацию',
              'Скопировать транскрибацию',
            )}
            {renderProcessingSection(
              'Постобработка',
              selectedRecord.postprocessing,
              'Повторить постобработку',
              'Скопировать постобработку',
            )}
          </Card>
        )}
      </aside>
    </div>
  );
};

export default HistoryPage;
