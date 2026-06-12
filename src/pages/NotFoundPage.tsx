import { Link } from 'react-router';

import { Button } from '#/ui/Button';

import styles from './NotFoundPage.module.scss';

export function NotFoundPage() {
  return (
    <section className={styles.page}>
      <p className={styles.code}>404</p>
      <h1 className={styles.title}>Page not found</h1>
      <Button as={Link} to="/">
        Back home
      </Button>
    </section>
  );
}
