const MODIFIER_KEYS = new Set(['Control', 'Alt', 'Shift', 'Meta']);

const CODE_TO_KEY: Record<string, string> = {
  Space: 'Space',
  Enter: 'Enter',
  NumpadEnter: 'Enter',
  Escape: 'Escape',
  Tab: 'Tab',
  Backspace: 'Backspace',
  Delete: 'Delete',
  Insert: 'Insert',
  Home: 'Home',
  End: 'End',
  PageUp: 'PageUp',
  PageDown: 'PageDown',
  ArrowUp: 'ArrowUp',
  ArrowDown: 'ArrowDown',
  ArrowLeft: 'ArrowLeft',
  ArrowRight: 'ArrowRight',
};

for (let i = 1; i <= 24; i++) {
  CODE_TO_KEY[`F${i}`] = `F${i}`;
}

for (let i = 0; i <= 9; i++) {
  CODE_TO_KEY[`Digit${i}`] = String(i);
}

for (const c of 'ABCDEFGHIJKLMNOPQRSTUVWXYZ') {
  CODE_TO_KEY[`Key${c}`] = c;
}

const formatHotkeyFromEvent = (event: KeyboardEvent): string | undefined => {
  if (MODIFIER_KEYS.has(event.key)) {
    return undefined;
  }

  const mainKey = CODE_TO_KEY[event.code];

  if (mainKey === undefined) {
    return undefined;
  }

  const parts: string[] = [];

  if (event.ctrlKey) parts.push('Ctrl');
  if (event.altKey) parts.push('Alt');
  if (event.shiftKey) parts.push('Shift');
  if (event.metaKey) parts.push('Win');

  parts.push(mainKey);

  return parts.join('+');
};

export default formatHotkeyFromEvent;
