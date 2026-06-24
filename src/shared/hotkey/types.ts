export type ModifierSide = 'none' | 'either' | 'left' | 'right';

export interface ParsedHotkey {
  ctrl: ModifierSide;
  alt: ModifierSide;
  shift: ModifierSide;
  meta: ModifierSide;
  key: string;
}
