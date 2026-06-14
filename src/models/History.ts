export interface HistoryAudio {
  duration: string;
  path: string;
}

export interface ProcessingDetails {
  cost: string;
  duration: string;
  isProcessing: boolean;
  model: string;
  provider: string;
  text: string;
}

export interface HistoryRecord {
  audio: HistoryAudio;
  id: string;
  postprocessing: ProcessingDetails;
  time: string;
  transcription: ProcessingDetails;
}

export interface HistoryGroup {
  date: string;
  label: string;
  month: string;
  records: HistoryRecord[];
}
