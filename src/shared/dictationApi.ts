import { invoke } from '@tauri-apps/api/core';

export const notifyDictationShortcutPressed = async (): Promise<void> => {
  await invoke('dictation_shortcut_pressed');
};

export const notifyDictationShortcutReleased = async (): Promise<void> => {
  await invoke('dictation_shortcut_released');
};

export const setHotkeyCaptureActive = async (active: boolean): Promise<void> => {
  await invoke('set_hotkey_capture_active', { active });
};
