import { invoke } from '@tauri-apps/api/core';

export interface StartupStatus {
  /** Данные в каталоге записаны более новой версией приложения, чем эта. */
  dataTooNew: boolean;
}

export const getStartupStatus = () => invoke<StartupStatus>('get_startup_status');

/**
 * Переносит все данные приложения в резервную папку и перезапускает приложение
 * с чистого состояния. Обычно не завершается на стороне вызывающего кода:
 * приложение перезапускается до разрешения промиса.
 */
export const resetAppData = () => invoke('reset_app_data');
