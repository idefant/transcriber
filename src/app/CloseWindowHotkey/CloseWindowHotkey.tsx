import { type FC, useEffect } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';

const appWindow = getCurrentWindow();

// Ctrl+W on either side, with no other modifier. `event.code` keeps the shortcut on the
// physical W key regardless of the active keyboard layout.
const isCloseWindowHotkey = (event: KeyboardEvent): boolean =>
  event.code === 'KeyW' && event.ctrlKey && !event.altKey && !event.shiftKey && !event.metaKey;

/**
 * Closes the main window on Ctrl+W while it has focus. Closing hides the window into the tray,
 * exactly like the titlebar close button, because the backend intercepts `CloseRequested`.
 */
const CloseWindowHotkey: FC = () => {
  useEffect(() => {
    // Bubble phase on purpose. `DictationHotkeyFallback` and `HotkeyInput` listen in the capture
    // phase and call stopPropagation() once they claim a key, so a user-assigned Ctrl+W hotkey and
    // the hotkey capture mode both win over closing the window.
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.repeat || !isCloseWindowHotkey(event)) {
        return;
      }

      event.preventDefault();
      void appWindow.close();
    };

    globalThis.addEventListener('keydown', handleKeyDown);

    return () => {
      globalThis.removeEventListener('keydown', handleKeyDown);
    };
  }, []);

  return null;
};

export default CloseWindowHotkey;
