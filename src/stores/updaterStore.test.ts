import { beforeEach, describe, expect, it, vi } from 'vitest';

const { checkForUpdateMock, downloadAndInstallUpdateMock, listenMock } = vi.hoisted(() => ({
  checkForUpdateMock: vi.fn(),
  downloadAndInstallUpdateMock: vi.fn(),
  listenMock: vi.fn(),
}));

vi.mock('#/shared/updaterApi', () => ({
  checkForUpdate: checkForUpdateMock,
  downloadAndInstallUpdate: downloadAndInstallUpdateMock,
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: listenMock,
}));

import { useUpdaterStore } from './updaterStore';

describe('useUpdaterStore', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useUpdaterStore.setState({
      availableUpdate: null,
      installProgress: null,
      isChecking: false,
      isInstalling: false,
      lastCheckedAt: null,
    });
  });

  it('stores the latest available update after a successful check', async () => {
    const info = { notes: 'Release notes', version: '1.2.3' };
    checkForUpdateMock.mockResolvedValue(info);

    const result = await useUpdaterStore.getState().checkForUpdates(true);

    expect(result).toEqual(info);
    expect(checkForUpdateMock).toHaveBeenCalledWith(true);
    expect(useUpdaterStore.getState().availableUpdate).toEqual(info);
    expect(useUpdaterStore.getState().isChecking).toBe(false);
    expect(useUpdaterStore.getState().lastCheckedAt).toEqual(expect.any(Number));
  });

  it('ignores stale update check results when a newer request finishes first', async () => {
    let resolveFirst: ((value: { notes?: string; version: string } | null) => void) | undefined;
    const firstCheck = new Promise<{ notes?: string; version: string } | null>((resolve) => {
      resolveFirst = resolve;
    });
    const latestInfo = { notes: 'Latest', version: '2.0.0' };

    checkForUpdateMock.mockReturnValueOnce(firstCheck).mockResolvedValueOnce(latestInfo);

    const firstPromise = useUpdaterStore.getState().checkForUpdates(false);
    const secondPromise = useUpdaterStore.getState().checkForUpdates(true);

    await secondPromise;
    resolveFirst?.({ notes: 'Stale', version: '1.5.0' });
    await firstPromise;

    expect(useUpdaterStore.getState().availableUpdate).toEqual(latestInfo);
    expect(useUpdaterStore.getState().isChecking).toBe(false);
  });

  it('tracks download progress and resets install state when installation fails', async () => {
    const unlistenMock = vi.fn();
    listenMock.mockImplementation(
      (
        _eventName: string,
        handler: (event: { payload: { downloaded: number; total: number } }) => void,
      ) => {
        handler({ payload: { downloaded: 50, total: 100 } });
        return Promise.resolve(unlistenMock);
      },
    );
    downloadAndInstallUpdateMock.mockRejectedValue(new Error('boom'));

    await expect(useUpdaterStore.getState().downloadAndInstall()).rejects.toThrow('boom');

    expect(listenMock).toHaveBeenCalledWith('updater://progress', expect.any(Function));
    expect(unlistenMock).toHaveBeenCalled();
    expect(useUpdaterStore.getState().installProgress).toBeNull();
    expect(useUpdaterStore.getState().isInstalling).toBe(false);
  });
});
