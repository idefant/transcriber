import type { ModifierSide, ParsedHotkey } from './types';

// Maps hotkey string tokens → canonical key names. Mirrors Rust parse_main_key aliases.
const KEY_ALIASES: Record<string, string> = {
  space: 'Space',
  enter: 'Enter',
  return: 'Enter',
  esc: 'Escape',
  escape: 'Escape',
  tab: 'Tab',
  backspace: 'Backspace',
  delete: 'Delete',
  del: 'Delete',
  insert: 'Insert',
  ins: 'Insert',
  home: 'Home',
  end: 'End',
  pageup: 'PageUp',
  page_up: 'PageUp',
  pagedown: 'PageDown',
  page_down: 'PageDown',
  up: 'ArrowUp',
  arrowup: 'ArrowUp',
  arrow_up: 'ArrowUp',
  down: 'ArrowDown',
  arrowdown: 'ArrowDown',
  arrow_down: 'ArrowDown',
  left: 'ArrowLeft',
  arrowleft: 'ArrowLeft',
  arrow_left: 'ArrowLeft',
  right: 'ArrowRight',
  arrowright: 'ArrowRight',
  arrow_right: 'ArrowRight',
};

const resolveMainKey = (token: string): string | undefined => {
  const lower = token.toLowerCase();

  if (KEY_ALIASES[lower] !== undefined) {
    return KEY_ALIASES[lower];
  }

  const fMatch = /^f(\d{1,2})$/.exec(lower);

  if (fMatch !== null) {
    const n = Number.parseInt(fMatch[1] ?? '', 10);

    if (n >= 1 && n <= 24) {
      return `F${n}`;
    }
  }

  if (/^[a-z0-9]$/.test(lower)) {
    return token.toUpperCase();
  }

  return undefined;
};

/**
 * Parses a hotkey string such as `"Ctrl+Space"` or `"LCtrl+Z"` into a structured object.
 * Returns `undefined` if the string has no valid main key.
 *
 * Modifier semantics:
 * - `"Ctrl"` / `"Alt"` / `"Shift"` / `"Win"` → side `'either'`
 * - `"LCtrl"` / `"LAlt"` / `"LShift"` / `"LWin"` → side `'left'`
 * - `"RCtrl"` / `"RAlt"` / `"RShift"` / `"RWin"` → side `'right'`
 * - absent → side `'none'`
 */
export const parseHotkey = (value: string): ParsedHotkey | undefined => {
  const parts = value
    .split('+')
    .map((p) => p.trim())
    .filter((p) => p.length > 0);

  let ctrl: ModifierSide = 'none';
  let alt: ModifierSide = 'none';
  let shift: ModifierSide = 'none';
  let meta: ModifierSide = 'none';
  let key = '';

  for (const part of parts) {
    const lower = part.toLowerCase();

    switch (lower) {
      case 'ctrl':
      case 'control': {
        ctrl = 'either';
        break;
      }
      case 'lctrl':
      case 'lcontrol': {
        ctrl = 'left';
        break;
      }
      case 'rctrl':
      case 'rcontrol': {
        ctrl = 'right';
        break;
      }
      case 'alt':
      case 'option': {
        alt = 'either';
        break;
      }
      case 'lalt':
      case 'loption': {
        alt = 'left';
        break;
      }
      case 'ralt':
      case 'roption': {
        alt = 'right';
        break;
      }
      case 'shift': {
        shift = 'either';
        break;
      }
      case 'lshift': {
        shift = 'left';
        break;
      }
      case 'rshift': {
        shift = 'right';
        break;
      }
      case 'win':
      case 'windows':
      case 'meta':
      case 'super':
      case 'cmd':
      case 'command': {
        meta = 'either';
        break;
      }
      case 'lwin': {
        meta = 'left';
        break;
      }
      case 'rwin': {
        meta = 'right';
        break;
      }
      default: {
        const resolved = resolveMainKey(part);

        if (resolved !== undefined) {
          key = resolved;
        }
      }
    }
  }

  return key.length > 0 ? { ctrl, alt, shift, meta, key } : undefined;
};
