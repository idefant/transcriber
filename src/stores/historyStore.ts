import { listen } from '@tauri-apps/api/event';
import { create } from 'zustand';

import * as historyApi from '#/shared/historyApi';

import type { HistoryGroup, HistoryRecord } from '#/models/History';

const getCurrentMonth = (): string => {
  const now = new Date();
  return `${now.getFullYear().toString()}-${String(now.getMonth() + 1).padStart(2, '0')}`;
};

interface OpenRecordRequest {
  recordId: string;
  month: string;
  date: string;
}

interface HistoryState {
  groups: HistoryGroup[];
  selectedMonth: string;
  isLoading: boolean;
  // Set when an external trigger (overlay "open record") asks the history page to
  // reveal a specific record. The page consumes it to drive its local selection.
  pendingOpenRecordId?: string;
  pendingOpenDate?: string;
  load: (month?: string, options?: { silent?: boolean }) => Promise<void>;
  setSelectedMonth: (month: string) => void;
  mergeRecord: (record: HistoryRecord) => void;
  removeRecord: (recordId: string) => void;
  openRecord: (request: OpenRecordRequest) => void;
  consumePendingOpenRecord: () => void;
}

export const useHistoryStore = create<HistoryState>((set, get) => ({
  groups: [],
  selectedMonth: getCurrentMonth(),
  isLoading: false,

  load: async (month, options = {}) => {
    const targetMonth = month ?? get().selectedMonth;
    if (!options.silent) {
      set({ isLoading: true });
    }
    try {
      const groups = await historyApi.getHistoryGroups(targetMonth);
      set({ groups });
    } finally {
      if (!options.silent) {
        set({ isLoading: false });
      }
    }
  },

  setSelectedMonth: (month) => {
    set({ selectedMonth: month, groups: [] });
  },

  mergeRecord: (record) => {
    const { groups, selectedMonth } = get();
    const exists = groups.some((g) => g.records.some((r) => r.id === record.id));

    if (exists) {
      set({
        groups: groups.map((group) => ({
          ...group,
          records: group.records.map((r) => (r.id === record.id ? record : r)),
        })),
      });
    } else {
      // New record — reload the current month silently so grouping is done by Rust.
      const recordMonth = record.createdAt.slice(0, 7);
      if (recordMonth === selectedMonth) {
        void get().load(selectedMonth, { silent: true });
      }
    }
  },

  removeRecord: (recordId) => {
    set((state) => ({
      groups: state.groups
        .map((group) => ({
          ...group,
          records: group.records.filter((r) => r.id !== recordId),
        }))
        .filter((group) => group.records.length > 0),
    }));
  },

  openRecord: ({ recordId, month, date }) => {
    if (month !== get().selectedMonth) {
      get().setSelectedMonth(month);
    }
    void get().load(month);
    set({ pendingOpenRecordId: recordId, pendingOpenDate: date });
  },

  consumePendingOpenRecord: () => {
    set({ pendingOpenRecordId: undefined, pendingOpenDate: undefined });
  },
}));

// Set up the Tauri event subscription. Call once from App.tsx on mount.
// The payload is HistoryRecord when a record is updated, null when deleted.
export const initHistoryEventSubscription = () =>
  listen<HistoryRecord | null>('history-updated', (event) => {
    const record = event.payload;
    if (record === null) {
      const { load, selectedMonth } = useHistoryStore.getState();
      void load(selectedMonth, { silent: true });
    } else {
      useHistoryStore.getState().mergeRecord(record);
    }
  });
