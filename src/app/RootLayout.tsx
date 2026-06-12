import { Outlet } from 'react-router';

import { AppHeader } from '#/components/AppHeader';

import styles from './RootLayout.module.scss';

export function RootLayout() {
  return (
    <div className={styles.root}>
      <AppHeader />
      <main className={styles.content}>
        <Outlet />
      </main>
    </div>
  );
}
