import { createBrowserRouter, Navigate } from 'react-router';

import RootLayout from '#/app/RootLayout';
import DictionaryPage from '#/pages/DictionaryPage';
import HistoryPage from '#/pages/HistoryPage';
import NotFoundPage from '#/pages/NotFoundPage';
import { routes } from '#/shared/routes';

export const router = createBrowserRouter([
  {
    children: [
      {
        element: <Navigate replace to={routes.history} />,
        index: true,
      },
      {
        element: <HistoryPage />,
        path: 'history',
      },
      {
        element: <DictionaryPage />,
        path: 'dictionary',
      },
      {
        element: <NotFoundPage />,
        path: '*',
      },
    ],
    element: <RootLayout />,
    path: '/',
  },
]);
