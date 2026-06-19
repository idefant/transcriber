import { Link } from 'react-router';
import type { FC } from 'react';
import { useTranslation } from 'react-i18next';

import { routes } from '#/shared/routes';
import Button from '#/ui/Button';

import styles from './NotFoundPage.module.scss';

const NotFoundPage: FC = () => {
  const { t } = useTranslation();

  return (
    <section className={styles.page}>
      <p className={styles.code}>404</p>
      <h1 className={styles.title}>{t('notFound.title')}</h1>
      <Button as={Link} to={routes.history}>
        {t('notFound.backHome')}
      </Button>
    </section>
  );
};

export default NotFoundPage;
