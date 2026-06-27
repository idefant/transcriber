import { beforeEach, describe, expect, it } from 'vitest';

import { useUiStore } from './uiStore';

describe('useUiStore', () => {
  beforeEach(() => {
    useUiStore.setState({
      isSettingsModalOpen: false,
      settingsSection: 'general',
    });
  });

  it('opens settings with an explicit section', () => {
    useUiStore.getState().openSettings('about');

    expect(useUiStore.getState().isSettingsModalOpen).toBe(true);
    expect(useUiStore.getState().settingsSection).toBe('about');
  });

  it('preserves the last selected section when reopened without an override', () => {
    useUiStore.getState().setSettingsSection('about');
    useUiStore.getState().closeSettings();
    useUiStore.getState().openSettings();

    expect(useUiStore.getState().isSettingsModalOpen).toBe(true);
    expect(useUiStore.getState().settingsSection).toBe('about');
  });
});
