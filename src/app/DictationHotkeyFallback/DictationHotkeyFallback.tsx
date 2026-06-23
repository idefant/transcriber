import { type FC, useEffect, useRef } from 'react';

import * as dictationApi from '#/shared/dictationApi';
import { isHotkeyCaptureActive } from '#/shared/hotkeyCaptureLock';

import { useSettingsStore } from '#/stores';

interface ParsedHotkey {
  alt: boolean;
  ctrl: boolean;
  key: string;
  meta: boolean;
  shift: boolean;
}

const keyAliases: Record<string, string> = {
  arrowdown: 'arrowdown',
  arrowleft: 'arrowleft',
  arrowright: 'arrowright',
  arrowup: 'arrowup',
  backspace: 'backspace',
  delete: 'delete',
  end: 'end',
  enter: 'enter',
  escape: 'escape',
  home: 'home',
  insert: 'insert',
  pagedown: 'pagedown',
  pageup: 'pageup',
  space: ' ',
  tab: 'tab',
};

const metaAliases = new Set(['win', 'windows', 'meta', 'super', 'cmd', 'command']);

const parseHotkey = (value: string): ParsedHotkey | undefined => {
  const parts = value
    .split('+')
    .map((part) => part.trim())
    .filter((part) => part.length > 0);

  const hotkey: ParsedHotkey = {
    alt: false,
    ctrl: false,
    key: '',
    meta: false,
    shift: false,
  };

  for (const part of parts) {
    const normalized = part.toLowerCase();

    switch (normalized) {
      case 'ctrl':
      case 'control': {
        hotkey.ctrl = true;
        break;
      }
      case 'alt':
      case 'option': {
        hotkey.alt = true;
        break;
      }
      case 'shift': {
        hotkey.shift = true;
        break;
      }
      default: {
        if (metaAliases.has(normalized)) {
          hotkey.meta = true;
        } else {
          hotkey.key = keyAliases[normalized] ?? normalized;
        }
      }
    }
  }

  return hotkey.key.length > 0 ? hotkey : undefined;
};

const eventMatchesHotkey = (event: KeyboardEvent, hotkey: ParsedHotkey) =>
  event.ctrlKey === hotkey.ctrl &&
  event.altKey === hotkey.alt &&
  event.shiftKey === hotkey.shift &&
  event.metaKey === hotkey.meta &&
  event.key.toLowerCase() === hotkey.key;

const DictationHotkeyFallback: FC = () => {
  const settings = useSettingsStore((s) => s.settings);
  const isShortcutActiveRef = useRef(false);

  useEffect(() => {
    const hotkey = parseHotkey(settings.hotkey);

    if (hotkey === undefined) {
      return;
    }

    const handleKeyDown = (event: KeyboardEvent) => {
      if (isHotkeyCaptureActive() || !eventMatchesHotkey(event, hotkey)) {
        return;
      }

      event.preventDefault();
      event.stopPropagation();

      if (event.repeat) {
        return;
      }

      isShortcutActiveRef.current = true;
      void dictationApi.notifyDictationShortcutPressed();
    };

    const handleKeyUp = (event: KeyboardEvent) => {
      if (!isShortcutActiveRef.current || event.key.toLowerCase() !== hotkey.key) {
        return;
      }

      event.preventDefault();
      event.stopPropagation();
      isShortcutActiveRef.current = false;
      void dictationApi.notifyDictationShortcutReleased();
    };

    globalThis.addEventListener('keydown', handleKeyDown, { capture: true });
    globalThis.addEventListener('keyup', handleKeyUp, { capture: true });

    return () => {
      globalThis.removeEventListener('keydown', handleKeyDown, { capture: true });
      globalThis.removeEventListener('keyup', handleKeyUp, { capture: true });
    };
  }, [settings.hotkey]);

  return null;
};

export default DictationHotkeyFallback;
