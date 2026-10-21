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

/** Ставит текущую запись на паузу или продолжает её. Вне записи ничего не делает. */
export const togglePauseDictation = async (sessionId?: number | null): Promise<void> => {
  await invoke('toggle_pause_dictation', { sessionId });
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

/** Идентифицирует одну вставку, чтобы запоздалое завершение не разблокировало следующую. */
export interface SyntheticPasteEvent {
  operationId: number;
  active: boolean;
}

/** Вызывается после синхронной блокировки DOM-хоткеев, до разрешения синтетического ввода. */
export const acknowledgeSyntheticPaste = async (operationId: number): Promise<void> => {
  await invoke('acknowledge_synthetic_paste', { operationId });
};

/** Проверяет ручной ввод общим парсером backend, не меняя сохранённые настройки. */
export const normalizeHotkey = async (value: string): Promise<string> =>
  invoke<string>('normalize_hotkey', { value });
