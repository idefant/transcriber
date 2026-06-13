import { type FC, type ReactNode } from 'react';

import styles from './SettingRow.module.scss';

interface SettingRowProps {
  children: ReactNode;
  description: string;
  title: string;
}

const SettingRow: FC<SettingRowProps> = ({ children, description, title }) => (
  <div className={styles.settingRow}>
    <div className={styles.text}>
      <h3 className={styles.title}>{title}</h3>
      <p className={styles.description}>{description}</p>
    </div>
    <div className={styles.control}>{children}</div>
  </div>
);

export default SettingRow;
