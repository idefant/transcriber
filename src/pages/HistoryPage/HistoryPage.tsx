import { type FC, useCallback, useEffect, useMemo, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { Button, Card, DatePicker, Empty, message, Space, Spin, Tooltip } from 'antd';
import dayjs from 'dayjs';
import { ChevronLeftIcon, ChevronRightIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import * as historyApi from '#/shared/historyApi';

import HistoryDetailsPanel from './HistoryDetailsPanel';
import HistoryRecordsList from './HistoryRecordsList';

import styles from './HistoryPage.module.scss';

import type { HistoryGroup, HistoryRecord } from '#/models/History';

const monthFormat = 'YYYY-MM';

const getCurrentMonth = () => dayjs().format(monthFormat);

const shiftMonth = (month: string, monthOffset: number) =>
  dayjs(`${month}-01`).add(monthOffset, 'month').format(monthFormat);

const getErrorMessage = (error: unknown) =>
  error instanceof Error ? error.message : String(error);

const getRecordTextForCopy = (record: HistoryRecord) => {
  if (record.postprocessing.status === 'success') {
    return record.postprocessing.text;
  }

  if (record.transcription.status === 'success') {
    return record.transcription.text;
  }

  return null;
};

const HistoryPage: FC = () => {
  const { t } = useTranslation();
  const [messageApi, messageContextHolder] = message.useMessage();
  const [historyGroups, setHistoryGroups] = useState<HistoryGroup[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [processingRecordId, setProcessingRecordId] = useState<string>();
  const [selectedMonth, setSelectedMonth] = useState(getCurrentMonth);
  const [activeDate, setActiveDate] = useState<string>();
  const [selectedRecord, setSelectedRecord] = useState<HistoryRecord>();

  const monthPickerValue = useMemo(() => dayjs(`${selectedMonth}-01`), [selectedMonth]);

  const filteredGroups = historyGroups;

  const loadHistory = useCallback(async () => {
    setIsLoading(true);

    try {
      const groups = await historyApi.getHistoryGroups(selectedMonth);

      setHistoryGroups(groups);
      setActiveDate((currentDate) =>
        groups.some((group) => group.date === currentDate) ? currentDate : groups[0]?.date,
      );
      setSelectedRecord((currentRecord) => {
        if (currentRecord === undefined) {
          return currentRecord;
        }

        return groups
          .flatMap((group) => group.records)
          .find((record) => record.id === currentRecord.id);
      });
    } catch (error) {
      void messageApi.error(getErrorMessage(error));
    } finally {
      setIsLoading(false);
    }
  }, [messageApi, selectedMonth]);

  useEffect(() => {
    queueMicrotask(() => {
      void loadHistory();
    });
  }, [loadHistory]);

  useEffect(() => {
    let isMounted = true;
    let removeListener: (() => void) | undefined;

    void listen('history-updated', () => {
      if (isMounted) {
        void loadHistory();
      }
    }).then((unlisten) => {
      removeListener = unlisten;

      if (!isMounted) {
        unlisten();
      }

      return null;
    });

    return () => {
      isMounted = false;
      removeListener?.();
    };
  }, [loadHistory]);

  const setMonth = (month: string) => {
    setSelectedMonth(month);
    setActiveDate(undefined);
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

  const updateRecord = (record: HistoryRecord) => {
    setHistoryGroups((currentGroups) =>
      currentGroups.map((group) => ({
        ...group,
        records: group.records.map((currentRecord) =>
          currentRecord.id === record.id ? record : currentRecord,
        ),
      })),
    );
    setSelectedRecord((currentRecord) =>
      currentRecord?.id === record.id ? record : currentRecord,
    );
  };

  const copyText = async (text: string | null) => {
    if (text === null) {
      return;
    }

    try {
      await navigator.clipboard.writeText(text);
    } catch (error) {
      void messageApi.error(getErrorMessage(error));
    }
  };

  const handleDeleteRecord = async (record: HistoryRecord) => {
    try {
      await historyApi.deleteHistoryRecord(record.id);
      setHistoryGroups((currentGroups) =>
        currentGroups
          .map((group) => ({
            ...group,
            records: group.records.filter((currentRecord) => currentRecord.id !== record.id),
          }))
          .filter((group) => group.records.length > 0),
      );
      setSelectedRecord((currentRecord) =>
        currentRecord?.id === record.id ? undefined : currentRecord,
      );
    } catch (error) {
      void messageApi.error(getErrorMessage(error));
    }
  };

  const handleOpenAudio = async (record: HistoryRecord) => {
    try {
      await historyApi.openHistoryAudio(record.id);
    } catch (error) {
      void messageApi.error(getErrorMessage(error));
    }
  };

  const handleRepeatRecord = async (record: HistoryRecord) => {
    setProcessingRecordId(record.id);

    try {
      updateRecord(await historyApi.repeatHistoryRecord(record.id));
    } catch (error) {
      void messageApi.error(getErrorMessage(error));
    } finally {
      setProcessingRecordId(undefined);
    }
  };

  const handleRepeatTranscription = async (record: HistoryRecord) => {
    setProcessingRecordId(record.id);

    try {
      updateRecord(await historyApi.repeatHistoryTranscription(record.id));
    } catch (error) {
      void messageApi.error(getErrorMessage(error));
    } finally {
      setProcessingRecordId(undefined);
    }
  };

  const handleRepeatPostProcessing = async (record: HistoryRecord) => {
    setProcessingRecordId(record.id);

    try {
      updateRecord(await historyApi.repeatHistoryPostProcessing(record.id));
    } catch (error) {
      void messageApi.error(getErrorMessage(error));
    } finally {
      setProcessingRecordId(undefined);
    }
  };

  return (
    <>
      {messageContextHolder}
      <div className={styles.page}>
        <Card className={styles.historyCard}>
          <div className={styles.toolbar}>
            <Space.Compact>
              <Tooltip title={t('history.previousMonth')}>
                <Button
                  aria-label={t('history.previousMonth')}
                  icon={<ChevronLeftIcon size={16} strokeWidth={2} />}
                  onClick={goToPreviousMonth}
                />
              </Tooltip>
              <DatePicker
                allowClear={false}
                className={styles.monthPicker}
                format={monthFormat}
                picker="month"
                placeholder={t('history.month')}
                value={monthPickerValue}
                renderExtraFooter={() => (
                  <Button block size="small" type="text" onClick={goToCurrentMonth}>
                    {t('history.today')}
                  </Button>
                )}
                onChange={handleMonthChange}
              />
              <Tooltip title={t('history.nextMonth')}>
                <Button
                  aria-label={t('history.nextMonth')}
                  icon={<ChevronRightIcon size={16} strokeWidth={2} />}
                  onClick={goToNextMonth}
                />
              </Tooltip>
            </Space.Compact>
          </div>

          <Spin spinning={isLoading}>
            {isLoading ? null : filteredGroups.length > 0 ? (
              <HistoryRecordsList
                activeDate={activeDate}
                groups={filteredGroups}
                processingRecordId={processingRecordId}
                selectedRecordId={selectedRecord?.id}
                onActiveDateChange={setActiveDate}
                onCopyRecordText={(record) => {
                  void copyText(getRecordTextForCopy(record));
                }}
                onDeleteRecord={(record) => {
                  void handleDeleteRecord(record);
                }}
                onRecordSelect={setSelectedRecord}
                onRepeatTranscription={(record) => {
                  void handleRepeatRecord(record);
                }}
              />
            ) : (
              <Empty description={t('history.emptyMonth')} />
            )}
          </Spin>
        </Card>

        <aside className={styles.detailsSlot}>
          {selectedRecord === undefined ? undefined : (
            <HistoryDetailsPanel
              record={selectedRecord}
              onCopyAudioPath={(record) => {
                void copyText(record.audio.path);
              }}
              onCopyPostProcessing={(record) => {
                void copyText(record.postprocessing.text);
              }}
              onCopyTranscription={(record) => {
                void copyText(record.transcription.text);
              }}
              onClose={() => {
                setSelectedRecord(undefined);
              }}
              onOpenAudio={(record) => {
                void handleOpenAudio(record);
              }}
              onRepeatPostProcessing={(record) => {
                void handleRepeatPostProcessing(record);
              }}
              onRepeatTranscription={(record) => {
                void handleRepeatTranscription(record);
              }}
            />
          )}
        </aside>
      </div>
    </>
  );
};

export default HistoryPage;
