import { RouterProvider } from 'react-router';
import type { FC } from 'react';

import AppSettingsProvider from '#/app/AppSettingsProvider';
import AppThemeProvider from '#/app/AppThemeProvider';
import ProcessingProvider from '#/app/ProcessingProvider';
import ProvidersProvider from '#/app/ProvidersProvider';
import { router } from '#/app/router';

const App: FC = () => {
  return (
    <AppSettingsProvider>
      <AppThemeProvider>
        <ProvidersProvider>
          <ProcessingProvider>
            <RouterProvider router={router} />
          </ProcessingProvider>
        </ProvidersProvider>
      </AppThemeProvider>
    </AppSettingsProvider>
  );
};

export default App;
