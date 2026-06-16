import { invoke } from '@tauri-apps/api/core';

import type { AppSettings, AppSettingsInput } from '#/models/Settings';

export const getAppSettings = () => invoke<AppSettings>('get_app_settings');

export const updateAppSettings = (input: AppSettingsInput) =>
  invoke<AppSettings>('update_app_settings', { input });
