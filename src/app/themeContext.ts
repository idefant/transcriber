import { createContext, useContext } from 'react';

export type ThemeMode = 'dark' | 'light';

export interface AppThemeContextValue {
  isDarkMode: boolean;
  mode: ThemeMode;
  setIsDarkMode: (value: boolean) => void;
}

export const AppThemeContext = createContext<AppThemeContextValue | undefined>(undefined);

export function useAppTheme() {
  const context = useContext(AppThemeContext);

  if (context === undefined) {
    throw new Error('useAppTheme must be used within AppThemeProvider');
  }

  return context;
}
