import { createBrowserRouter, Navigate } from 'react-router';

import { RootLayout } from '#/app/RootLayout';
import { AboutPage } from '#/pages/AboutPage';
import { HomePage } from '#/pages/HomePage';
import { NotFoundPage } from '#/pages/NotFoundPage';

export const router = createBrowserRouter([
  {
    children: [
      {
        element: <HomePage />,
        index: true,
      },
      {
        element: <AboutPage />,
        path: 'about',
      },
      {
        element: <Navigate replace to="/" />,
        path: 'home',
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
