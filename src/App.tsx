import { RouterProvider } from 'react-router';
import type { FC } from 'react';

import AppThemeProvider from '#/app/AppThemeProvider';
import { router } from '#/app/router';

const App: FC = () => {
  return (
    <AppThemeProvider>
      <RouterProvider router={router} />
    </AppThemeProvider>
  );
};

export default App;
