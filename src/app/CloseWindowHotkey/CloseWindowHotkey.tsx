import { type FC, useEffect } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';

const appWindow = getCurrentWindow();

// Ctrl+W с любой стороны, без других модификаторов. `event.code` удерживает комбинацию
// на физической клавише W независимо от активной раскладки клавиатуры.
const isCloseWindowHotkey = (event: KeyboardEvent): boolean =>
  event.code === 'KeyW' && event.ctrlKey && !event.altKey && !event.shiftKey && !event.metaKey;

/**
 * Закрывает главное окно по Ctrl+W, когда оно находится в фокусе. Закрытие скрывает окно в трей,
 * точно так же, как кнопка закрытия в заголовке окна, потому что бэкенд перехватывает `CloseRequested`.
 */
const CloseWindowHotkey: FC = () => {
  useEffect(() => {
    // Фаза всплытия выбрана намеренно. `DictationHotkeyFallback` и `HotkeyInput` слушают на фазе
    // перехвата и вызывают stopPropagation(), как только забирают клавишу, поэтому назначенный
    // пользователем хоткей Ctrl+W и режим захвата хоткея оба побеждают закрытие окна.
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
