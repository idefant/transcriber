import { type FC, type ReactNode, useCallback, useEffect, useMemo, useState } from 'react';
import { ConfigProvider, theme as antdTheme } from 'antd';

import { AppThemeContext, type ThemeMode, type ThemePreference } from '#/app/themeContext';

interface AppThemeProviderProps {
  children: ReactNode;
}

const darkModeMediaQuery = '(prefers-color-scheme: dark)';

const getSystemThemeMode = (): ThemeMode => {
  if (!('matchMedia' in globalThis)) {
    return 'light';
  }

  return globalThis.matchMedia(darkModeMediaQuery).matches ? 'dark' : 'light';
};

const AppThemeProvider: FC<AppThemeProviderProps> = ({ children }) => {
  const [themePreference, setThemePreference] = useState<ThemePreference>('light');
  const [systemThemeMode, setSystemThemeMode] = useState<ThemeMode>(() => getSystemThemeMode());
  const isDarkMode =
    themePreference === 'dark' || (themePreference === 'auto' && systemThemeMode === 'dark');
  const mode: ThemeMode = isDarkMode ? 'dark' : 'light';

  const setIsDarkMode = useCallback((value: boolean) => {
    setThemePreference(value ? 'dark' : 'light');
  }, []);

  useEffect(() => {
    if (!('matchMedia' in globalThis)) {
      return;
    }

    const mediaQueryList = globalThis.matchMedia(darkModeMediaQuery);
    const handleChange = (event: MediaQueryListEvent) => {
      setSystemThemeMode(event.matches ? 'dark' : 'light');
    };

    mediaQueryList.addEventListener('change', handleChange);

    return () => {
      mediaQueryList.removeEventListener('change', handleChange);
    };
  }, []);

  const contextValue = useMemo(
    () => ({
      isDarkMode,
      mode,
      setIsDarkMode,
      setThemePreference,
      themePreference,
    }),
    [isDarkMode, mode, setIsDarkMode, themePreference],
  );

  return (
    <AppThemeContext.Provider value={contextValue}>
      <ConfigProvider
        theme={{
          algorithm: isDarkMode ? antdTheme.darkAlgorithm : antdTheme.defaultAlgorithm,
          components: {
            Form: {
              itemMarginBottom: 12,
            },
            Menu: {
              itemMarginInline: 8,
            },
          },
          // token: {
          //   borderRadius: 8,
          //   colorPrimary: '#2f766f',
          // },
        }}
      >
        {children}
      </ConfigProvider>
    </AppThemeContext.Provider>
  );
};

export default AppThemeProvider;
