import { Link } from 'react-router';
import type { FC } from 'react';

import Button from '#/ui/Button';

import styles from './HomePage.module.scss';

const HomePage: FC = () => {
  return (
    <section className={styles.page}>
      <div className={styles.intro}>
        <p className={styles.eyebrow}>React + TypeScript + Vite</p>
        <h1 className={styles.title}>A clean starter for building the transcriber app.</h1>
        <p className={styles.description}>
          Routing, aliases, SCSS modules, strict linting, formatting, and pre-commit checks are
          wired from the first commit.
        </p>
        <Button as={Link} to="/about">
          View setup
        </Button>
      </div>
    </section>
  );
};

export default HomePage;
