import { Link } from 'react-router';
import type { FC } from 'react';

import { routes } from '#/shared/routes';
import Button from '#/ui/Button';

import styles from './NotFoundPage.module.scss';

const NotFoundPage: FC = () => {
  return (
    <section className={styles.page}>
      <p className={styles.code}>404</p>
      <h1 className={styles.title}>Page not found</h1>
      <Button as={Link} to={routes.history}>
        Back home
      </Button>
    </section>
  );
};

export default NotFoundPage;
