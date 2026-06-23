import { type FC, useEffect, useMemo, useState } from 'react';
import { Button, Card, DatePicker, Empty, message, Space, Spin, Tooltip } from 'antd';
import dayjs from 'dayjs';
import { ChevronLeftIcon, ChevronRightIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import * as historyApi from '#/shared/historyApi';

import HistoryDetailsPanel from './HistoryDetailsPanel';
import HistoryRecordsList from './HistoryRecordsList';

import styles from './HistoryPage.module.scss';

import type { HistoryRecord } from '#/models/History';
import { useHistoryStore } from '#/stores';

const monthFormat = 'YYYY-MM';

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

const getCurrentMonth = () => {
  const now = new Date();
  return `${now.getFullYear().toString()}-${String(now.getMonth() + 1).padStart(2, '0')}`;
};

const HistoryPage: FC = () => {
  const { t } = useTranslation();
  const [messageApi, messageContextHolder] = message.useMessage();

  const groups = useHistoryStore((s) => s.groups);
  const selectedMonth = useHistoryStore((s) => s.selectedMonth);
  const isLoading = useHistoryStore((s) => s.isLoading);
  const storeLoad = useHistoryStore((s) => s.load);
  const storeSetSelectedMonth = useHistoryStore((s) => s.setSelectedMonth);
  const storeRemoveRecord = useHistoryStore((s) => s.removeRecord);

  const [processingRecordId, setProcessingRecordId] = useState<string>();
  // preferredDate is the user-selected date. Effective activeDate falls back to groups[0] when
  // the preferred date is no longer present in groups (e.g. after a month change).
  const [preferredDate, setPreferredDate] = useState<string>();
  // selectedRecordId tracks the id; the full record is resolved from the store's groups so it
  // automatically reflects event-driven updates without a separate sync effect.
  const [selectedRecordId, setSelectedRecordId] = useState<string>();

  const monthPickerValue = useMemo(() => dayjs(`${selectedMonth}-01`), [selectedMonth]);

  const activeDate = useMemo(
    () => (groups.some((g) => g.date === preferredDate) ? preferredDate : groups[0]?.date),
    [groups, preferredDate],
  );

  const selectedRecord = useMemo(
    () =>
      selectedRecordId === undefined
        ? undefined
        : groups.flatMap((g) => g.records).find((r) => r.id === selectedRecordId),
    [groups, selectedRecordId],
  );

  // Initial load on mount only. Month changes are handled explicitly in setMonth().
  useEffect(() => {
    queueMicrotask(() => {
      void storeLoad(selectedMonth).catch((error: unknown) => {
        void messageApi.error(getErrorMessage(error));
      });
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const setMonth = (month: string) => {
    storeSetSelectedMonth(month);
    setPreferredDate(undefined);
    setSelectedRecordId(undefined);
    void storeLoad(month).catch((error: unknown) => {
      void messageApi.error(getErrorMessage(error));
    });
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
      storeRemoveRecord(record.id);
      setSelectedRecordId((current) => (current === record.id ? undefined : current));
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

  // For repeat actions, we only await the command (to know when it's done).
  // Record updates arrive via the history-updated event → historyStore.mergeRecord → groups update
  // → selectedRecord is automatically recomputed from the store via useMemo.

  const handleRepeatRecord = async (record: HistoryRecord) => {
    setProcessingRecordId(record.id);

    try {
      await historyApi.repeatHistoryRecord(record.id);
    } catch (error) {
      void messageApi.error(getErrorMessage(error));
    } finally {
      setProcessingRecordId(undefined);
    }
  };

  const handleRepeatTranscription = async (record: HistoryRecord) => {
    setProcessingRecordId(record.id);

    try {
      await historyApi.repeatHistoryTranscription(record.id);
    } catch (error) {
      void messageApi.error(getErrorMessage(error));
    } finally {
      setProcessingRecordId(undefined);
    }
  };

  const handleRepeatPostProcessing = async (record: HistoryRecord) => {
    setProcessingRecordId(record.id);

    try {
      await historyApi.repeatHistoryPostProcessing(record.id);
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
            {groups.length > 0 ? (
              <HistoryRecordsList
                activeDate={activeDate}
                groups={groups}
                processingRecordId={processingRecordId}
                selectedRecordId={selectedRecord?.id}
                onActiveDateChange={setPreferredDate}
                onCopyRecordText={(record) => {
                  void copyText(getRecordTextForCopy(record));
                }}
                onDeleteRecord={(record) => {
                  void handleDeleteRecord(record);
                }}
                onRecordSelect={(record) => {
                  setSelectedRecordId(record.id);
                }}
                onRepeatTranscription={(record) => {
                  void handleRepeatRecord(record);
                }}
              />
            ) : isLoading ? null : (
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
                setSelectedRecordId(undefined);
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
