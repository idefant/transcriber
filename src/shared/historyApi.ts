import { invoke } from '@tauri-apps/api/core';

import type { HistoryGroup, HistoryRecord } from '#/models/History';

export const getHistoryGroups = (month?: string) =>
  invoke<HistoryGroup[]>('get_history_groups', { month });

export const deleteHistoryRecord = async (recordId: string): Promise<void> => {
  await invoke('delete_history_record', { recordId });
};

export const openHistoryAudio = async (recordId: string): Promise<void> => {
  await invoke('open_history_audio', { recordId });
};

export const openHistoryRecord = async (recordId: string): Promise<void> => {
  await invoke('open_history_record', { recordId });
};

export const repeatHistoryTranscription = (recordId: string) =>
  invoke<HistoryRecord>('repeat_history_transcription', { recordId });

export const repeatHistoryRecord = (recordId: string) =>
  invoke<HistoryRecord>('repeat_history_record', { recordId });

export const repeatHistoryPostProcessing = (recordId: string) =>
  invoke<HistoryRecord>('repeat_history_post_processing', { recordId });
