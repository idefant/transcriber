import styles from './AboutPage.module.scss';

const setupItems = [
  'React Router',
  'TypeScript aliases',
  'ESLint import order',
  'Prettier',
  'Stylelint',
  'Husky',
];

export function AboutPage() {
  return (
    <section className={styles.page}>
      <div className={styles.header}>
        <p className={styles.eyebrow}>Project setup</p>
        <h1 className={styles.title}>The app shell is ready for feature work.</h1>
      </div>
      <ul className={styles.list}>
        {setupItems.map((item) => (
          <li className={styles.item} key={item}>
            {item}
          </li>
        ))}
      </ul>
    </section>
  );
}
