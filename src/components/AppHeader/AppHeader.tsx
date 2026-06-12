import { NavLink } from 'react-router';

import { routes } from '#/shared/routes';

import styles from './AppHeader.module.scss';

export function AppHeader() {
  return (
    <header className={styles.header}>
      <nav aria-label="Primary navigation" className={styles.nav}>
        <NavLink className={styles.brand} to={routes.home}>
          Transcriber
        </NavLink>
        <div className={styles.links}>
          <NavLink
            className={({ isActive }) => (isActive ? styles.activeLink : styles.link)}
            to={routes.home}
          >
            Home
          </NavLink>
          <NavLink
            className={({ isActive }) => (isActive ? styles.activeLink : styles.link)}
            to={routes.about}
          >
            About
          </NavLink>
        </div>
      </nav>
    </header>
  );
}
