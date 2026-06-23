let locked = false;

export const lockHotkeyCapture = (): void => {
  locked = true;
};

export const unlockHotkeyCapture = (): void => {
  locked = false;
};

export const isHotkeyCaptureActive = (): boolean => locked;
