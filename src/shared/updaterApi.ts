import { invoke } from '@tauri-apps/api/core';

export interface UpdateInfo {
  version: string;
  notes?: string;
}

export interface UpdateProgress {
  downloaded: number;
  total?: number;
}

export const checkForUpdate = (offerUnstable: boolean) =>
  invoke<UpdateInfo | null>('check_for_update', { offerUnstable });

export const downloadAndInstallUpdate = () => invoke<undefined>('download_and_install_update');
