// Maps KeyboardEvent.code → canonical key name matching Rust parse_main_key output.
export const CODE_TO_KEY: Record<string, string> = {
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
