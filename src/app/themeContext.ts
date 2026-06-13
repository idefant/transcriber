import { createContext, useContext } from 'react';

export type ThemeMode = 'dark' | 'light';
export type ThemePreference = 'auto' | ThemeMode;

export interface AppThemeContextValue {
  isDarkMode: boolean;
  mode: ThemeMode;
  setIsDarkMode: (value: boolean) => void;
  setThemePreference: (value: ThemePreference) => void;
  themePreference: ThemePreference;
}

export const AppThemeContext = createContext<AppThemeContextValue | undefined>(undefined);

export function useAppTheme() {
  const context = useContext(AppThemeContext);

  if (context === undefined) {
    throw new Error('useAppTheme must be used within AppThemeProvider');
  }

  return context;
}
