import { type FC, useMemo, useState } from 'react';
import { Button, Card, DatePicker, Empty, Space, Tooltip } from 'antd';
import dayjs from 'dayjs';
import { ChevronLeftIcon, ChevronRightIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import HistoryDetailsPanel from './HistoryDetailsPanel';
import HistoryRecordsList from './HistoryRecordsList';

import styles from './HistoryPage.module.scss';

import { historyGroups } from '#/mocks/history';
import type { HistoryRecord } from '#/models/History';

const monthFormat = 'YYYY-MM';

const getCurrentMonth = () => dayjs().format(monthFormat);

const getFirstGroupByMonth = (month: string) =>
  historyGroups.find((group) => group.month === month);

const shiftMonth = (month: string, monthOffset: number) =>
  dayjs(`${month}-01`).add(monthOffset, 'month').format(monthFormat);

const HistoryPage: FC = () => {
  const { t } = useTranslation();
  const [selectedMonth, setSelectedMonth] = useState(getCurrentMonth);
  const [activeDate, setActiveDate] = useState(getFirstGroupByMonth(selectedMonth)?.date);
  const [selectedRecord, setSelectedRecord] = useState<HistoryRecord>();

  const monthPickerValue = useMemo(() => dayjs(`${selectedMonth}-01`), [selectedMonth]);

  const filteredGroups = useMemo(() => {
    return historyGroups.filter((group) => group.month === selectedMonth);
  }, [selectedMonth]);

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

  return (
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

        {filteredGroups.length > 0 ? (
          <HistoryRecordsList
            activeDate={activeDate}
            groups={filteredGroups}
            selectedRecordId={selectedRecord?.id}
            onActiveDateChange={setActiveDate}
            onRecordSelect={setSelectedRecord}
          />
        ) : (
          <Empty description={t('history.emptyMonth')} />
        )}
      </Card>

      <aside className={styles.detailsSlot}>
        {selectedRecord === undefined ? undefined : (
          <HistoryDetailsPanel
            record={selectedRecord}
            onClose={() => {
              setSelectedRecord(undefined);
            }}
          />
        )}
      </aside>
    </div>
  );
};

export default HistoryPage;
