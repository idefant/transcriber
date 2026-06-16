import { RouterProvider } from 'react-router';
import type { FC } from 'react';

import AppSettingsProvider from '#/app/AppSettingsProvider';
import AppThemeProvider from '#/app/AppThemeProvider';
import ProvidersProvider from '#/app/ProvidersProvider';
import { router } from '#/app/router';

const App: FC = () => {
  return (
    <AppSettingsProvider>
      <AppThemeProvider>
        <ProvidersProvider>
          <RouterProvider router={router} />
        </ProvidersProvider>
      </AppThemeProvider>
    </AppSettingsProvider>
  );
};

export default App;
