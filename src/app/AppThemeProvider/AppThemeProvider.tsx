import { type FC, type ReactNode, useCallback, useEffect, useMemo, useState } from 'react';
import { ConfigProvider, theme as antdTheme } from 'antd';
import enUS from 'antd/locale/en_US';
import ruRU from 'antd/locale/ru_RU';
import dayjs from 'dayjs';

import { useAppSettings } from '#/app/settingsContext';
import { AppThemeContext, type ThemeMode, type ThemePreference } from '#/app/themeContext';

import 'dayjs/locale/ru';

interface AppThemeProviderProps {
  children: ReactNode;
}

const darkModeMediaQuery = '(prefers-color-scheme: dark)';
const appTokenVariableNames = [
  '--app-color-bg-container',
  '--app-color-bg-layout',
  '--app-color-bg-text-hover',
  '--app-color-border',
  '--app-color-border-secondary',
  '--app-color-fill-tertiary',
  '--app-color-primary-bg',
  '--app-color-primary-border',
  '--app-color-text',
  '--app-color-text-secondary',
  '--app-color-text-tertiary',
] as const;

const getSystemThemeMode = (): ThemeMode => {
  if (!('matchMedia' in globalThis)) {
    return 'light';
  }

  return globalThis.matchMedia(darkModeMediaQuery).matches ? 'dark' : 'light';
};

const AppThemeProvider: FC<AppThemeProviderProps> = ({ children }) => {
  const { settings, updateSettings } = useAppSettings();
  const [systemThemeMode, setSystemThemeMode] = useState<ThemeMode>(() => getSystemThemeMode());
  const themePreference = settings.themePreference;
  const antdLocale = settings.effectiveUiLanguage === 'ru' ? ruRU : enUS;
  const isDarkMode =
    themePreference === 'dark' || (themePreference === 'auto' && systemThemeMode === 'dark');
  const mode: ThemeMode = isDarkMode ? 'dark' : 'light';

  const setThemePreference = useCallback(
    (value: ThemePreference) => {
      void updateSettings({ themePreference: value }).catch(() => {
        // The error is stored in AppSettingsContext.
      });
    },
    [updateSettings],
  );

  const setIsDarkMode = useCallback(
    (value: boolean) => {
      void updateSettings({ themePreference: value ? 'dark' : 'light' }).catch(() => {
        // The error is stored in AppSettingsContext.
      });
    },
    [updateSettings],
  );

  useEffect(() => {
    dayjs.locale(settings.effectiveUiLanguage);
  }, [settings.effectiveUiLanguage]);

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
    [isDarkMode, mode, setIsDarkMode, setThemePreference, themePreference],
  );

  return (
    <AppThemeContext.Provider value={contextValue}>
      <ConfigProvider
        locale={antdLocale}
        theme={{
          algorithm: isDarkMode ? antdTheme.darkAlgorithm : antdTheme.defaultAlgorithm,
          components: {
            Form: {
              itemMarginBottom: 12,
            },
            Menu: {
              itemActiveBg: isDarkMode ? 'rgb(255 255 255 / 8%)' : 'rgb(0 0 0 / 4%)',
              itemHoverBg: isDarkMode ? 'rgb(255 255 255 / 8%)' : 'rgb(0 0 0 / 4%)',
              itemMarginInline: 8,
              itemSelectedBg: isDarkMode ? 'rgb(255 255 255 / 12%)' : '#e6f4ff',
            },
          },
          // token: {
          //   borderRadius: 8,
          //   colorPrimary: '#2f766f',
          // },
        }}
      >
        <AppThemeTokenVariables />
        {children}
      </ConfigProvider>
    </AppThemeContext.Provider>
  );
};

const AppThemeTokenVariables: FC = () => {
  const { token } = antdTheme.useToken();

  useEffect(() => {
    const rootStyle = document.documentElement.style;

    rootStyle.setProperty('--app-color-bg-container', token.colorBgContainer);
    rootStyle.setProperty('--app-color-bg-layout', token.colorBgLayout);
    rootStyle.setProperty('--app-color-bg-text-hover', token.colorBgTextHover);
    rootStyle.setProperty('--app-color-border', token.colorBorder);
    rootStyle.setProperty('--app-color-border-secondary', token.colorBorderSecondary);
    rootStyle.setProperty('--app-color-fill-tertiary', token.colorFillTertiary);
    rootStyle.setProperty('--app-color-primary-bg', token.colorPrimaryBg);
    rootStyle.setProperty('--app-color-primary-border', token.colorPrimaryBorder);
    rootStyle.setProperty('--app-color-text', token.colorText);
    rootStyle.setProperty('--app-color-text-secondary', token.colorTextSecondary);
    rootStyle.setProperty('--app-color-text-tertiary', token.colorTextTertiary);

    return () => {
      for (const variableName of appTokenVariableNames) {
        rootStyle.removeProperty(variableName);
      }
    };
  }, [token]);

  return null;
};

export default AppThemeProvider;
