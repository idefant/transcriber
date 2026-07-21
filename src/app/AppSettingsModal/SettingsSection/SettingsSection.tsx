import { type FC, type ReactNode } from 'react';

import styles from './SettingsSection.module.scss';

interface SettingsSectionProps {
  children: ReactNode;
  title: string;
}

/**
 * Группа настроек с подзаголовком внутри вкладки. Заголовок секции — `h2`,
 * уровнем выше `h3` из `SettingRow`, поэтому вкладка остаётся с корректной
 * иерархией заголовков.
 */
const SettingsSection: FC<SettingsSectionProps> = ({ children, title }) => (
  <section className={styles.section}>
    <h2 className={styles.title}>{title}</h2>
    <div className={styles.body}>{children}</div>
  </section>
);

export default SettingsSection;
