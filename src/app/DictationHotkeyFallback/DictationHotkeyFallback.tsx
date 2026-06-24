import { type FC, useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';

import * as dictationApi from '#/shared/dictationApi';
import { CODE_TO_KEY, matchesHotkey, MODIFIER_CODES, parseHotkey } from '#/shared/hotkey';
import { isHotkeyCaptureActive } from '#/shared/hotkeyCaptureLock';

import { useSettingsStore } from '#/stores';

const DictationHotkeyFallback: FC = () => {
  const settings = useSettingsStore((s) => s.settings);
  const isShortcutActiveRef = useRef(false);
  const isSessionActiveRef = useRef(false);
  const pressedModifierCodesRef = useRef(new Set<string>());

  // Track dictation session state to gate the cancel hotkey.
  // Cancel is only intercepted while a session is active; outside a session
  // Ctrl+Z (or any other cancel hotkey) passes through to the webview unchanged.
  useEffect(() => {
    const unlistenPromise = listen<{ active: boolean }>('dictation-session', (event) => {
      isSessionActiveRef.current = event.payload.active;
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

    if (dictationHotkey === undefined) {
      return;
    }

    const pressedModifierCodes = pressedModifierCodesRef.current;

    const handleKeyDown = (event: KeyboardEvent) => {
      // Keep modifier-side tracking up to date before checking hotkey matches.
      if (MODIFIER_CODES.has(event.code)) {
        pressedModifierCodes.add(event.code);
        return;
      }

      if (isHotkeyCaptureActive()) {
        return;
      }

      // Cancel hotkey — only consumed when a dictation session is active.
      if (
        cancelHotkey !== undefined &&
        isSessionActiveRef.current &&
        matchesHotkey(event, pressedModifierCodes, cancelHotkey)
      ) {
        event.preventDefault();
        event.stopPropagation();
        void dictationApi.cancelDictation();
        return;
      }

      // Dictation hotkey.
      if (matchesHotkey(event, pressedModifierCodes, dictationHotkey)) {
        event.preventDefault();
        event.stopPropagation();

        if (event.repeat) {
          return;
        }

        isShortcutActiveRef.current = true;
        void dictationApi.notifyDictationShortcutPressed();
      }
    };

    const handleKeyUp = (event: KeyboardEvent) => {
      // Keep modifier-side tracking up to date.
      if (MODIFIER_CODES.has(event.code)) {
        pressedModifierCodes.delete(event.code);
        return;
      }

      if (!isShortcutActiveRef.current) {
        return;
      }

      // Release only when the main key of the active dictation hotkey comes up.
      const eventKey = CODE_TO_KEY[event.code];

      if (eventKey?.toLowerCase() !== dictationHotkey.key.toLowerCase()) {
        return;
      }

      event.preventDefault();
      event.stopPropagation();
      isShortcutActiveRef.current = false;
      void dictationApi.notifyDictationShortcutReleased();
    };

    // Clear tracked modifier state when the window loses focus so stale entries
    // don't cause false matches after the user switches back.
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
  }, [settings.hotkey, settings.cancelHotkey]);

  return null;
};

export default DictationHotkeyFallback;
