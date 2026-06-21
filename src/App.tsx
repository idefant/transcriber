import { RouterProvider } from 'react-router';
import type { FC } from 'react';

import AppSettingsProvider from '#/app/AppSettingsProvider';
import AppThemeProvider from '#/app/AppThemeProvider';
import DictationHotkeyFallback from '#/app/DictationHotkeyFallback';
import I18nProvider from '#/app/I18nProvider';
import ProcessingProvider from '#/app/ProcessingProvider';
import ProvidersProvider from '#/app/ProvidersProvider';
import { router } from '#/app/router';

const App: FC = () => {
  return (
    <AppSettingsProvider>
      <DictationHotkeyFallback />
      <I18nProvider>
        <AppThemeProvider>
          <ProvidersProvider>
            <ProcessingProvider>
              <RouterProvider router={router} />
            </ProcessingProvider>
          </ProvidersProvider>
        </AppThemeProvider>
      </I18nProvider>
    </AppSettingsProvider>
  );
};

export default App;
