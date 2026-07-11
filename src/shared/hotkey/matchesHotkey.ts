import { CODE_TO_KEY } from './keyMap';
import type { ModifierSide, ParsedHotkey } from './types';

const matchesModifierSide = (
  side: ModifierSide,
  pressedCodes: Set<string>,
  leftCode: string,
  rightCode: string,
): boolean => {
  const leftDown = pressedCodes.has(leftCode);
  const rightDown = pressedCodes.has(rightCode);

  switch (side) {
    case 'none': {
      return !leftDown && !rightDown;
    }
    case 'either': {
      return leftDown || rightDown;
    }
    case 'left': {
      return leftDown && !rightDown;
    }
    case 'right': {
      return !leftDown && rightDown;
    }
  }
};

/**
 * Returns true if the keyboard event matches the given parsed hotkey. `pressedModifierCodes` holds
 * the `event.code` values of the currently held modifier keys.
 *
 * Matching is strict for side-specific modifiers:
 * - `'left'` → left key must be down AND right key must be up
 * - `'right'` → right key must be down AND left key must be up
 * - `'either'` → at least one side must be down
 * - `'none'` → both sides must be up
 */
export const matchesHotkey = (
  event: KeyboardEvent,
  pressedModifierCodes: Set<string>,
  hotkey: ParsedHotkey,
): boolean => {
  const mainKey = CODE_TO_KEY[event.code];

  if (mainKey?.toLowerCase() !== hotkey.key.toLowerCase()) {
    return false;
  }

  return (
    matchesModifierSide(hotkey.ctrl, pressedModifierCodes, 'ControlLeft', 'ControlRight') &&
    matchesModifierSide(hotkey.alt, pressedModifierCodes, 'AltLeft', 'AltRight') &&
    matchesModifierSide(hotkey.shift, pressedModifierCodes, 'ShiftLeft', 'ShiftRight') &&
    matchesModifierSide(hotkey.meta, pressedModifierCodes, 'MetaLeft', 'MetaRight')
  );
};
