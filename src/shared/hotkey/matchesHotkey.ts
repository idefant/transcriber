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
 * Возвращает true, если событие клавиатуры соответствует переданному распарсенному хоткею. `pressedModifierCodes`
 * хранит значения `event.code` для клавиш-модификаторов, которые в данный момент удерживаются.
 *
 * Для модификаторов, зависящих от стороны, сопоставление строгое:
 * - `'left'` → левая клавиша должна быть нажата, а правая — отпущена
 * - `'right'` → правая клавиша должна быть нажата, а левая — отпущена
 * - `'either'` → должна быть нажата хотя бы одна из сторон
 * - `'none'` → обе стороны должны быть отпущены
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
