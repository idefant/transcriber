import { invoke } from '@tauri-apps/api/core';

export const notifyDictationShortcutPressed = async (): Promise<void> => {
  await invoke('dictation_shortcut_pressed');
};

export const notifyDictationShortcutReleased = async (): Promise<void> => {
  await invoke('dictation_shortcut_released');
};
