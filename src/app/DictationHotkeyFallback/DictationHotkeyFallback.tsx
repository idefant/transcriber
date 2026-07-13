import { type FC, useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';

import * as dictationApi from '#/shared/dictationApi';
import { CODE_TO_KEY, matchesHotkey, MODIFIER_CODES, parseHotkey } from '#/shared/hotkey';
import { isHotkeyCaptureActive } from '#/shared/hotkeyCaptureLock';

import { useSettingsStore } from '#/stores';

const DictationHotkeyFallback: FC = () => {
  const settings = useSettingsStore((s) => s.settings);
  const isSessionActiveRef = useRef(false);
  const isRecordingRef = useRef(false);
  const currentSessionIdRef = useRef<number | null>(null);
  const nextActivationIdRef = useRef(1);
  const activeActivationIdRef = useRef<number | null>(null);
  const pressedModifierCodesRef = useRef(new Set<string>());

  // Отслеживаем состояние сессии диктовки, чтобы управлять перехватом хоткеев отмены и паузы.
  // Отмена перехватывается только пока сессия активна, пауза — только пока идет запись;
  // вне сессии хоткей отмены (как и хоткей паузы) проходит во webview без изменений.
  useEffect(() => {
    const unlistenPromise = listen<{
      active: boolean;
      isRecording: boolean;
      sessionId?: number | null;
    }>('dictation-session', (event) => {
      isSessionActiveRef.current = event.payload.active;
      isRecordingRef.current = event.payload.isRecording;
      currentSessionIdRef.current = event.payload.active ? (event.payload.sessionId ?? null) : null;
    });

    return () => {
      void unlistenPromise.then((unlisten) => {
        unlisten();
        return null;
      });
    };
  }, []);

  useEffect(() => {
    const dictationHotkey = parseHotkey(settings.hotkey);
    const cancelHotkey =
      settings.cancelHotkey.trim().length > 0 ? parseHotkey(settings.cancelHotkey) : undefined;
    // Пауза доступна только в режиме записи по нажатию: в режиме удержания запись
    // и так заканчивается отпусканием хоткея, поэтому пауза не имеет смысла.
    const pauseHotkey =
      settings.triggerMode === 'press' && settings.pauseHotkey.trim().length > 0
        ? parseHotkey(settings.pauseHotkey)
        : undefined;
    const copyLatestHotkey =
      settings.copyLatestHotkey.trim().length > 0
        ? parseHotkey(settings.copyLatestHotkey)
        : undefined;
    const pasteLatestHotkey =
      settings.pasteLatestHotkey.trim().length > 0
        ? parseHotkey(settings.pasteLatestHotkey)
        : undefined;
    const repeatLatestHotkey =
      settings.repeatLatestHotkey.trim().length > 0
        ? parseHotkey(settings.repeatLatestHotkey)
        : undefined;

    if (dictationHotkey === undefined) {
      return;
    }

    const pressedModifierCodes = pressedModifierCodesRef.current;

    const handleKeyDown = (event: KeyboardEvent) => {
      // Обновляем отслеживание нажатых модификаторов перед проверкой совпадения хоткея.
      if (MODIFIER_CODES.has(event.code)) {
        pressedModifierCodes.add(event.code);
        return;
      }

      if (isHotkeyCaptureActive()) {
        return;
      }

      if (matchesHotkey(event, pressedModifierCodes, dictationHotkey)) {
        event.preventDefault();
        event.stopPropagation();

        if (event.repeat) {
          return;
        }

        const activationId = nextActivationIdRef.current;
        nextActivationIdRef.current += 1;
        activeActivationIdRef.current = activationId;
        void dictationApi.notifyDictationShortcutPressed(activationId);
        return;
      }

      // Хоткей отмены перехватывается только когда сессия диктовки активна.
      if (
        cancelHotkey !== undefined &&
        isSessionActiveRef.current &&
        matchesHotkey(event, pressedModifierCodes, cancelHotkey)
      ) {
        event.preventDefault();
        event.stopPropagation();
        void dictationApi.cancelDictation(currentSessionIdRef.current);
        return;
      }

      // Хоткей паузы перехватывается только во время записи: после ее остановки
      // пауза уже недоступна, и клавиша должна работать как обычно.
      if (
        pauseHotkey !== undefined &&
        isRecordingRef.current &&
        matchesHotkey(event, pressedModifierCodes, pauseHotkey)
      ) {
        event.preventDefault();
        event.stopPropagation();
        void dictationApi.togglePauseDictation(currentSessionIdRef.current);
        return;
      }

      if (
        copyLatestHotkey !== undefined &&
        matchesHotkey(event, pressedModifierCodes, copyLatestHotkey)
      ) {
        event.preventDefault();
        event.stopPropagation();
        void dictationApi.copyLatestHistoryText();
        return;
      }

      if (
        pasteLatestHotkey !== undefined &&
        matchesHotkey(event, pressedModifierCodes, pasteLatestHotkey)
      ) {
        event.preventDefault();
        event.stopPropagation();
        void dictationApi.pasteLatestHistoryText();
        return;
      }

      if (
        repeatLatestHotkey !== undefined &&
        matchesHotkey(event, pressedModifierCodes, repeatLatestHotkey)
      ) {
        event.preventDefault();
        event.stopPropagation();
        void dictationApi.repeatLatestHistoryRecord();
        return;
      }
    };

    const handleKeyUp = (event: KeyboardEvent) => {
      // Обновляем отслеживание нажатых модификаторов.
      if (MODIFIER_CODES.has(event.code)) {
        pressedModifierCodes.delete(event.code);
        return;
      }

      const activationId = activeActivationIdRef.current;

      if (activationId === null) {
        return;
      }

      // Отпускание учитывается, только когда отпущена основная клавиша активного хоткея диктовки.
      const eventKey = CODE_TO_KEY[event.code];

      if (eventKey?.toLowerCase() !== dictationHotkey.key.toLowerCase()) {
        return;
      }

      event.preventDefault();
      event.stopPropagation();
      activeActivationIdRef.current = null;
      void dictationApi.notifyDictationShortcutReleased(activationId);
    };

    // Сбрасываем отслеживаемое состояние модификаторов при потере окном фокуса, чтобы устаревшие
    // записи не приводили к ложным совпадениям после возврата пользователя в окно.
    const handleBlur = () => {
      pressedModifierCodes.clear();
    };

    globalThis.addEventListener('keydown', handleKeyDown, { capture: true });
    globalThis.addEventListener('keyup', handleKeyUp, { capture: true });
    globalThis.addEventListener('blur', handleBlur);

    return () => {
      globalThis.removeEventListener('keydown', handleKeyDown, { capture: true });
      globalThis.removeEventListener('keyup', handleKeyUp, { capture: true });
      globalThis.removeEventListener('blur', handleBlur);
    };
  }, [
    settings.cancelHotkey,
    settings.copyLatestHotkey,
    settings.hotkey,
    settings.pasteLatestHotkey,
    settings.pauseHotkey,
    settings.repeatLatestHotkey,
    settings.triggerMode,
  ]);

  return null;
};

export default DictationHotkeyFallback;
