import { type ComponentProps, type FC } from 'react';
import { useTranslation } from 'react-i18next';
import Markdown, { type Components, type ExtraProps } from 'react-markdown';
import remarkGfm from 'remark-gfm';

import styles from './ReleaseNotes.module.scss';

interface ReleaseNotesProps {
  notes: string;
  version: string;
}

type MarkdownLinkProps = ComponentProps<'a'> & ExtraProps;

// Обычный <a href> увёл бы всё окно Tauri из приложения по ссылке.
// Плагин opener не зарегистрирован, поэтому ссылки остаются неактивными и только показывают свой URL.
const MarkdownLink: FC<MarkdownLinkProps> = ({ children, href }) => (
  <a
    href={href}
    rel="noreferrer"
    title={href}
    onClick={(event) => {
      event.preventDefault();
    }}
  >
    {children}
  </a>
);

const markdownComponents: Components = {
  a: MarkdownLink,
};

const remarkPlugins = [remarkGfm];

const ReleaseNotes: FC<ReleaseNotesProps> = ({ notes, version }) => {
  const { t } = useTranslation();

  return (
    <section className={styles.releaseNotes}>
      <h3 className={styles.title}>{t('settings.about.releaseNotesTitle', { version })}</h3>
      <div className={styles.body}>
        <Markdown components={markdownComponents} remarkPlugins={remarkPlugins}>
          {notes}
        </Markdown>
      </div>
    </section>
  );
};

export default ReleaseNotes;
