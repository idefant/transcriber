import { type FC, type ReactNode } from 'react';
import clsx from 'clsx';

import styles from './SettingsSection.module.scss';

interface SettingsSectionProps {
  children: ReactNode;
  /** Плотный шаг для секций с полями формы: они компактнее строк настроек. */
  compact?: boolean;
  title: string;
}

/**
 * Группа настроек с подзаголовком внутри вкладки. Заголовок секции — `h2`,
 * уровнем выше `h3` из `SettingRow`, поэтому вкладка остаётся с корректной
 * иерархией заголовков.
 */
const SettingsSection: FC<SettingsSectionProps> = ({ children, compact = false, title }) => (
  <section className={clsx(styles.section, compact && styles.compact)}>
    <h2 className={styles.title}>{title}</h2>
    <div className={styles.body}>{children}</div>
  </section>
);

export default SettingsSection;
