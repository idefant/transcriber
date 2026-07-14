import { type FC, useEffect, useMemo, useState } from 'react';
import { Button, Card, DatePicker, Empty, message, Space, Spin, Tooltip } from 'antd';
import clsx from 'clsx';
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
  // preferredDate — выбранная пользователем дата.
  // undefined = автоматически (открыть сегодняшнюю дату, если для неё есть записи)
  // null      = явно закрыто пользователем
  // string    = явно открыто пользователем
  const [preferredDate, setPreferredDate] = useState<string | null | undefined>();
  // selectedRecordId хранит id; сама запись берётся из groups в сторе, поэтому
  // она автоматически отражает обновления, вызванные событиями, без отдельного эффекта синхронизации.
  const [selectedRecordId, setSelectedRecordId] = useState<string>();

  const monthPickerValue = useMemo(() => dayjs(`${selectedMonth}-01`), [selectedMonth]);

  const activeDate = useMemo(() => {
    if (preferredDate === null) return;
    if (preferredDate !== undefined && groups.some((g) => g.date === preferredDate)) {
      return preferredDate;
    }
    // Автоматический режим: открыть сегодняшнюю дату, если для неё есть записи, иначе — ничего.
    const today = dayjs().format('YYYY-MM-DD');
    return groups.some((g) => g.date === today) ? today : undefined;
  }, [groups, preferredDate]);

  const selectedRecord = useMemo(
    () =>
      selectedRecordId === undefined
        ? undefined
        : groups.flatMap((g) => g.records).find((r) => r.id === selectedRecordId),
    [groups, selectedRecordId],
  );

  // Первоначальная загрузка только при монтировании. Изменения месяца обрабатываются явно в setMonth().
  useEffect(() => {
    queueMicrotask(() => {
      void storeLoad(selectedMonth).catch((error: unknown) => {
        void messageApi.error(getErrorMessage(error));
      });
    });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Показываем запись, запрошенную из уведомления об ошибке/предупреждении оверлея. Стор
  // уже переключил месяц и запустил перезагрузку; здесь мы управляем
  // локальным выбором и разворачиваем нужный день. Подписка на стор (а не
  // эффект, производный от рендера) не даёт setState попасть в тело эффекта.
  useEffect(() => {
    const applyPendingOpenRecord = (state: ReturnType<typeof useHistoryStore.getState>) => {
      if (state.pendingOpenRecordId === undefined) {
        return;
      }

      setSelectedRecordId(state.pendingOpenRecordId);
      if (state.pendingOpenDate !== undefined) {
        setPreferredDate(state.pendingOpenDate);
      }
      state.consumePendingOpenRecord();
    };

    applyPendingOpenRecord(useHistoryStore.getState());

    return useHistoryStore.subscribe((state, prev) => {
      if (
        state.pendingOpenRecordId === undefined ||
        state.pendingOpenRecordId === prev.pendingOpenRecordId
      ) {
        return;
      }

      applyPendingOpenRecord(state);
    });
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
      void messageApi.success(t('history.copySuccess'));
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

  // Для действий повтора мы только ожидаем завершения команды (чтобы знать, когда она выполнена).
  // Обновления записи приходят через событие history-updated → historyStore.mergeRecord → обновление groups
  // → selectedRecord автоматически пересчитывается из стора через useMemo.

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

  const isDetailsOpen = selectedRecord !== undefined;

  return (
    <>
      {messageContextHolder}
      <div className={clsx(styles.page, isDetailsOpen && styles.withDetails)}>
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
                onActiveDateChange={(date) => {
                  setPreferredDate(date);
                }}
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
