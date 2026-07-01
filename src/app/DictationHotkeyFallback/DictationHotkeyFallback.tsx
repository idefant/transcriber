import { type FC, useEffect, useRef } from 'react';
import { listen } from '@tauri-apps/api/event';

import * as dictationApi from '#/shared/dictationApi';
import { CODE_TO_KEY, matchesHotkey, MODIFIER_CODES, parseHotkey } from '#/shared/hotkey';
import { isHotkeyCaptureActive } from '#/shared/hotkeyCaptureLock';

import { useSettingsStore } from '#/stores';

const DictationHotkeyFallback: FC = () => {
  const settings = useSettingsStore((s) => s.settings);
  const isSessionActiveRef = useRef(false);
  const currentSessionIdRef = useRef<number | null>(null);
  const nextActivationIdRef = useRef(1);
  const activeActivationIdRef = useRef<number | null>(null);
  const pressedModifierCodesRef = useRef(new Set<string>());

  // Track dictation session state to gate the cancel hotkey.
  // Cancel is only intercepted while a session is active; outside a session
  // Ctrl+Z (or any other cancel hotkey) passes through to the webview unchanged.
  useEffect(() => {
    const unlistenPromise = listen<{ active: boolean; sessionId?: number | null }>(
      'dictation-session',
      (event) => {
        isSessionActiveRef.current = event.payload.active;
        currentSessionIdRef.current = event.payload.active
          ? (event.payload.sessionId ?? null)
          : null;
      },
    );

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
      // Keep modifier-side tracking up to date before checking hotkey matches.
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

      // Cancel hotkey is only consumed when a dictation session is active.
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
      // Keep modifier-side tracking up to date.
      if (MODIFIER_CODES.has(event.code)) {
        pressedModifierCodes.delete(event.code);
        return;
      }

      const activationId = activeActivationIdRef.current;

      if (activationId === null) {
        return;
      }

      // Release only when the main key of the active dictation hotkey comes up.
      const eventKey = CODE_TO_KEY[event.code];

      if (eventKey?.toLowerCase() !== dictationHotkey.key.toLowerCase()) {
        return;
      }

      event.preventDefault();
      event.stopPropagation();
      activeActivationIdRef.current = null;
      void dictationApi.notifyDictationShortcutReleased(activationId);
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
  }, [
    settings.cancelHotkey,
    settings.copyLatestHotkey,
    settings.hotkey,
    settings.pasteLatestHotkey,
    settings.repeatLatestHotkey,
  ]);

  return null;
};

export default DictationHotkeyFallback;
