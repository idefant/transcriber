import { setHotkeyCaptureActive } from './dictationApi';

let activeCaptures = 0;
let syncVersion = 0;
let nativeSync = Promise.resolve<null>(null);

// Смена поля вызывает cleanup и новый захват за один проход React: отправляем только итог.
const syncNativeCapture = (): void => {
  const version = ++syncVersion;
  queueMicrotask(() => {
    nativeSync = nativeSync
      .then(async () => {
        if (version !== syncVersion) return null;
        await setHotkeyCaptureActive(activeCaptures > 0);
        return null;
      })
      .catch((error: unknown) => {
        console.error(error);
        return null;
      });
  });
};

/** Добавляет владельца блокировки; каждое получение должно завершиться одним освобождением. */
export const lockHotkeyCapture = (): void => {
  activeCaptures += 1;
  syncNativeCapture();
};

/** Освобождает одного владельца, сохраняя блокировку остальных полей. */
export const unlockHotkeyCapture = (): void => {
  activeCaptures = Math.max(0, activeCaptures - 1);
  syncNativeCapture();
};

/** Блокировка действует, пока хотя бы одно поле записывает или редактирует сочетание. */
export const isHotkeyCaptureActive = (): boolean => activeCaptures > 0;
