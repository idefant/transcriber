import { type FC, type ReactNode, useMemo, useState } from 'react';
import { ConfigProvider, theme as antdTheme } from 'antd';

import { AppThemeContext, type ThemeMode } from '#/app/themeContext';

interface AppThemeProviderProps {
  children: ReactNode;
}

const AppThemeProvider: FC<AppThemeProviderProps> = ({ children }) => {
  const [isDarkMode, setIsDarkMode] = useState(false);
  const mode: ThemeMode = isDarkMode ? 'dark' : 'light';

  const contextValue = useMemo(
    () => ({
      isDarkMode,
      mode,
      setIsDarkMode,
    }),
    [isDarkMode, mode],
  );

  return (
    <AppThemeContext.Provider value={contextValue}>
      <ConfigProvider
        theme={{
          algorithm: isDarkMode ? antdTheme.darkAlgorithm : antdTheme.defaultAlgorithm,
          components: {
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
