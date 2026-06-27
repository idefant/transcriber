import { create } from 'zustand';

import type { SettingsSectionKey } from '#/models/Settings';

interface UiState {
  isSettingsModalOpen: boolean;
  settingsSection: SettingsSectionKey;
  closeSettings: () => void;
  openSettings: (section?: SettingsSectionKey) => void;
  setSettingsSection: (section: SettingsSectionKey) => void;
}

export const useUiStore = create<UiState>((set) => ({
  isSettingsModalOpen: false,
  settingsSection: 'general',

  closeSettings: () => {
    set({ isSettingsModalOpen: false });
  },

  openSettings: (section) => {
    set((state) => ({
      isSettingsModalOpen: true,
      settingsSection: section ?? state.settingsSection,
    }));
  },

  setSettingsSection: (section) => {
    set({ settingsSection: section });
  },
}));
