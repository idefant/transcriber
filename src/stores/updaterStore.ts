import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { create } from 'zustand';

import type { UpdateInfo, UpdateProgress } from '#/shared/updaterApi';
import * as updaterApi from '#/shared/updaterApi';

let latestCheckRequestId = 0;

interface UpdaterState {
  availableUpdate: UpdateInfo | null;
  installProgress: UpdateProgress | null;
  isChecking: boolean;
  isInstalling: boolean;
  lastCheckedAt: number | null;
  checkForUpdates: (offerUnstable: boolean) => Promise<UpdateInfo | null>;
  downloadAndInstall: () => Promise<void>;
}

export const useUpdaterStore = create<UpdaterState>((set, get) => ({
  availableUpdate: null,
  installProgress: null,
  isChecking: false,
  isInstalling: false,
  lastCheckedAt: null,

  checkForUpdates: async (offerUnstable) => {
    const requestId = ++latestCheckRequestId;

    set({ isChecking: true });

    try {
      const info = await updaterApi.checkForUpdate(offerUnstable);

      if (requestId !== latestCheckRequestId) {
        return get().availableUpdate;
      }

      set({
        availableUpdate: info,
        lastCheckedAt: Date.now(),
      });

      return info;
    } finally {
      if (requestId === latestCheckRequestId) {
        set({ isChecking: false });
      }
    }
  },

  downloadAndInstall: async () => {
    if (get().isInstalling) {
      return;
    }

    set({
      installProgress: null,
      isInstalling: true,
    });

    let unlisten: UnlistenFn | undefined;

    try {
      unlisten = await listen<UpdateProgress>('updater://progress', (event) => {
        set({ installProgress: event.payload });
      });

      await updaterApi.downloadAndInstallUpdate();
    } catch (error) {
      set({
        installProgress: null,
        isInstalling: false,
      });

      throw error;
    } finally {
      unlisten?.();
    }
  },
}));
