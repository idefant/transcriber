import { invoke } from '@tauri-apps/api/core';

export const notifyDictationShortcutPressed = async (activationId: number): Promise<void> => {
  await invoke('dictation_shortcut_pressed', { activationId });
};

export const notifyDictationShortcutReleased = async (activationId: number): Promise<void> => {
  await invoke('dictation_shortcut_released', { activationId });
};

export const setHotkeyCaptureActive = async (active: boolean): Promise<void> => {
  await invoke('set_hotkey_capture_active', { active });
};

export const cancelDictation = async (sessionId?: number | null): Promise<void> => {
  await invoke('cancel_dictation', { sessionId });
};

export const copyLatestHistoryText = async (): Promise<void> => {
  await invoke('copy_latest_history_text');
};

export const pasteLatestHistoryText = async (): Promise<void> => {
  await invoke('paste_latest_history_text');
};

export const repeatLatestHistoryRecord = async (): Promise<void> => {
  await invoke('repeat_latest_history_record');
};
