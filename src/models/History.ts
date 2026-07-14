export interface HistoryAudio {
  duration: string;
  durationMs: number;
  path: string;
}

export type HistoryResultStatus = 'error' | 'processing' | 'skipped' | 'success';
export type HistoryRecordStatus = 'error' | 'processing' | 'success';

export interface ProcessingDetails {
  cost?: string | null;
  duration: string;
  durationMs?: number | null;
  errorMessage?: string | null;
  errorDetails?: unknown;
  isProcessing: boolean;
  model: string;
  provider: string;
  resolvedProvider?: string | null;
  status: HistoryResultStatus;
  text: string;
  usage?: unknown;
  settingsSnapshot?: unknown;
}

export interface HistoryRecord {
  audio: HistoryAudio;
  createdAt: string;
  finalText: string;
  id: string;
  postprocessing: ProcessingDetails;
  status: HistoryRecordStatus;
  time: string;
  transcription: ProcessingDetails;
}

export interface HistoryGroup {
  date: string;
  label: string;
  month: string;
  records: HistoryRecord[];
}

export interface HistorySearchResult {
  groups: HistoryGroup[];
  page: number;
  /** Размер страницы приходит с бэкенда, чтобы фронтенд не дублировал его у себя. */
  pageSize: number;
  /** Общее число найденных записей во всей истории, а не на текущей странице. */
  total: number;
}
