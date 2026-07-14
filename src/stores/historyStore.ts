import { listen } from '@tauri-apps/api/event';
import { create } from 'zustand';

import * as historyApi from '#/shared/historyApi';

import type { HistoryGroup, HistoryRecord } from '#/models/History';

const getCurrentMonth = (): string => {
  const now = new Date();
  return `${now.getFullYear().toString()}-${String(now.getMonth() + 1).padStart(2, '0')}`;
};

/** Trigram-индекс FTS5 не ищет последовательности короче трёх символов. */
const searchMinQueryLength = 3;
const defaultSearchPageSize = 100;

// Номер последнего запущенного поиска. Быстрый набор текста порождает несколько
// параллельных запросов, и ответ на устаревший не должен затирать более свежий.
let searchRequestSeq = 0;

const replaceRecordInGroups = (groups: HistoryGroup[], record: HistoryRecord): HistoryGroup[] =>
  groups.map((group) => ({
    ...group,
    records: group.records.map((r) => (r.id === record.id ? record : r)),
  }));

const removeRecordFromGroups = (groups: HistoryGroup[], recordId: string): HistoryGroup[] =>
  groups
    .map((group) => ({
      ...group,
      records: group.records.filter((r) => r.id !== recordId),
    }))
    .filter((group) => group.records.length > 0);

const hasRecord = (groups: HistoryGroup[], recordId: string): boolean =>
  groups.some((group) => group.records.some((r) => r.id === recordId));

interface OpenRecordRequest {
  recordId: string;
  month: string;
  date: string;
}

/** `month` — помесячный просмотр истории, `search` — поиск по всей истории. */
export type HistoryViewMode = 'month' | 'search';

interface HistoryState {
  groups: HistoryGroup[];
  selectedMonth: string;
  isLoading: boolean;
  viewMode: HistoryViewMode;
  /** Сырой текст поля ввода. Живёт в сторе, чтобы пережить уход со страницы истории. */
  searchInput: string;
  /** Запрос, по которому реально выполнен поиск. По нему же подсвечиваются совпадения. */
  searchQuery: string;
  searchGroups: HistoryGroup[];
  searchPage: number;
  searchPageSize: number;
  searchTotal: number;
  isSearchLoading: boolean;
  // Устанавливается, когда внешний триггер (оверлей «открыть запись») просит страницу истории
  // показать конкретную запись. Страница использует это значение для управления своим локальным выбором.
  pendingOpenRecordId?: string;
  pendingOpenDate?: string;
  load: (month?: string, options?: { silent?: boolean }) => Promise<void>;
  setSelectedMonth: (month: string) => void;
  mergeRecord: (record: HistoryRecord) => void;
  removeRecord: (recordId: string) => void;
  openRecord: (request: OpenRecordRequest) => void;
  consumePendingOpenRecord: () => void;
  openSearch: () => void;
  closeSearch: () => void;
  setSearchInput: (value: string) => void;
  runSearch: (query: string, page: number) => Promise<void>;
}

const emptySearchState = {
  searchInput: '',
  searchQuery: '',
  searchGroups: [] as HistoryGroup[],
  searchPage: 1,
  searchTotal: 0,
  isSearchLoading: false,
};

export const useHistoryStore = create<HistoryState>((set, get) => ({
  groups: [],
  selectedMonth: getCurrentMonth(),
  isLoading: false,
  viewMode: 'month',
  searchPageSize: defaultSearchPageSize,
  ...emptySearchState,

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
    const { groups, searchGroups, selectedMonth } = get();

    // Найденные записи обновляются на месте, но новая диктовка в готовую выдачу
    // поиска не добавляется: она не обязана удовлетворять текущему запросу.
    if (hasRecord(searchGroups, record.id)) {
      set({ searchGroups: replaceRecordInGroups(searchGroups, record) });
    }

    if (hasRecord(groups, record.id)) {
      set({ groups: replaceRecordInGroups(groups, record) });
      return;
    }

    // Новая запись — тихо перезагружаем текущий месяц, чтобы группировку выполнил Rust.
    const recordMonth = record.createdAt.slice(0, 7);
    if (recordMonth === selectedMonth) {
      void get().load(selectedMonth, { silent: true });
    }
  },

  removeRecord: (recordId) => {
    set((state) => ({
      groups: removeRecordFromGroups(state.groups, recordId),
      searchGroups: removeRecordFromGroups(state.searchGroups, recordId),
      searchTotal: hasRecord(state.searchGroups, recordId)
        ? Math.max(0, state.searchTotal - 1)
        : state.searchTotal,
    }));
  },

  openRecord: ({ recordId, month, date }) => {
    if (month !== get().selectedMonth) {
      get().setSelectedMonth(month);
    }
    void get().load(month);
    set({
      viewMode: 'month',
      ...emptySearchState,
      pendingOpenRecordId: recordId,
      pendingOpenDate: date,
    });
  },

  consumePendingOpenRecord: () => {
    set({ pendingOpenRecordId: undefined, pendingOpenDate: undefined });
  },

  openSearch: () => {
    set({ viewMode: 'search', ...emptySearchState });
  },

  closeSearch: () => {
    searchRequestSeq += 1;
    set({ viewMode: 'month', ...emptySearchState });
  },

  setSearchInput: (value) => {
    set({ searchInput: value });
  },

  runSearch: async (query, page) => {
    const trimmed = query.trim();
    searchRequestSeq += 1;
    const requestId = searchRequestSeq;

    if (trimmed.length < searchMinQueryLength) {
      set({
        searchQuery: '',
        searchGroups: [],
        searchPage: 1,
        searchTotal: 0,
        isSearchLoading: false,
      });
      return;
    }

    set({ isSearchLoading: true });

    try {
      const result = await historyApi.searchHistoryRecords(trimmed, page);

      if (requestId !== searchRequestSeq) {
        return;
      }

      set({
        searchQuery: trimmed,
        searchGroups: result.groups,
        searchPage: result.page,
        searchPageSize: result.pageSize,
        searchTotal: result.total,
      });
    } finally {
      if (requestId === searchRequestSeq) {
        set({ isSearchLoading: false });
      }
    }
  },
}));

// Настраивает подписку на событие Tauri. Вызывается один раз из App.tsx при монтировании.
// В payload передаётся HistoryRecord, если запись обновлена, и null, если удалена.
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
