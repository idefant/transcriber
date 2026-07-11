import { CODE_TO_KEY } from './keyMap';

/** Набор значений `event.code` для всех клавиш-модификаторов (обеих сторон). */
export const MODIFIER_CODES = new Set([
  'ControlLeft',
  'ControlRight',
  'AltLeft',
  'AltRight',
  'ShiftLeft',
  'ShiftRight',
  'MetaLeft',
  'MetaRight',
]);

/**
 * Преобразует событие клавиатуры и коды текущих нажатых модификаторов в строку хоткея.
 * Возвращает `undefined` для нажатий чистых клавиш-модификаторов или неизвестных кодов клавиш.
 *
 * Вывод с учётом стороны:
 * - зажат только левый Ctrl → `"LCtrl"`
 * - зажат только правый Ctrl → `"RCtrl"`
 * - зажаты оба Ctrl → `"Ctrl"` (любая сторона, для обратной совместимости)
 *
 * Та же логика применяется к Alt, Shift и Win/Meta.
 */
export const formatHotkeyFromEvent = (
  event: KeyboardEvent,
  pressedModifierCodes: Set<string>,
): string | undefined => {
  if (MODIFIER_CODES.has(event.code)) {
    return undefined;
  }

  const mainKey = CODE_TO_KEY[event.code];

  if (mainKey === undefined) {
    return undefined;
  }

  const parts: string[] = [];

  const lCtrl = pressedModifierCodes.has('ControlLeft');
  const rCtrl = pressedModifierCodes.has('ControlRight');

  if (lCtrl && rCtrl) parts.push('Ctrl');
  else if (lCtrl) parts.push('LCtrl');
  else if (rCtrl) parts.push('RCtrl');

  const lAlt = pressedModifierCodes.has('AltLeft');
  const rAlt = pressedModifierCodes.has('AltRight');

  if (lAlt && rAlt) parts.push('Alt');
  else if (lAlt) parts.push('LAlt');
  else if (rAlt) parts.push('RAlt');

  const lShift = pressedModifierCodes.has('ShiftLeft');
  const rShift = pressedModifierCodes.has('ShiftRight');

  if (lShift && rShift) parts.push('Shift');
  else if (lShift) parts.push('LShift');
  else if (rShift) parts.push('RShift');

  const lMeta = pressedModifierCodes.has('MetaLeft');
  const rMeta = pressedModifierCodes.has('MetaRight');

  if (lMeta && rMeta) parts.push('Win');
  else if (lMeta) parts.push('LWin');
  else if (rMeta) parts.push('RWin');

  parts.push(mainKey);

  return parts.join('+');
};
