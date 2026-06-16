import { RouterProvider } from 'react-router';
import type { FC } from 'react';

import AppThemeProvider from '#/app/AppThemeProvider';
import ProvidersProvider from '#/app/ProvidersProvider';
import { router } from '#/app/router';

const App: FC = () => {
  return (
    <AppThemeProvider>
      <ProvidersProvider>
        <RouterProvider router={router} />
      </ProvidersProvider>
    </AppThemeProvider>
  );
};

export default App;
