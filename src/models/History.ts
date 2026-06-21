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
  isProcessing: boolean;
  model: string;
  provider: string;
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
