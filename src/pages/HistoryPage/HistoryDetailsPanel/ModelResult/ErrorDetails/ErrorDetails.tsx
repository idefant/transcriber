import { type FC, useState } from 'react';
import { Button } from 'antd';
import { ChevronDownIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import styles from './ErrorDetails.module.scss';

interface ErrorDetailsProps {
  details: unknown;
}

const hasDisplayableDetails = (details: unknown): boolean => {
  if (details == null) {
    return false;
  }

  if (typeof details === 'string') {
    return details.trim().length > 0;
  }

  if (Array.isArray(details)) {
    return details.length > 0;
  }

  if (typeof details === 'object') {
    return Object.keys(details).length > 0;
  }

  return true;
};

const formatDetails = (details: unknown): string => {
  if (typeof details === 'string') {
    return details;
  }

  try {
    return JSON.stringify(details, null, 2);
  } catch {
    return String(details);
  }
};

const ErrorDetails: FC<ErrorDetailsProps> = ({ details }) => {
  const { t } = useTranslation();
  const [isOpen, setIsOpen] = useState(false);
  const formattedDetails = hasDisplayableDetails(details) ? formatDetails(details).trim() : '';

  if (formattedDetails.length === 0) {
    return null;
  }

  return (
    <div className={styles.errorDetails}>
      <Button
        className={styles.toggle}
        icon={
          <ChevronDownIcon
            className={isOpen ? styles.chevronOpen : styles.chevron}
            size={16}
            strokeWidth={2}
          />
        }
        size="small"
        type="text"
        onClick={() => {
          setIsOpen((value) => !value);
        }}
      >
        {isOpen ? t('history.details.hideErrorDetails') : t('history.details.showErrorDetails')}
      </Button>
      {isOpen ? <pre className={styles.json}>{formattedDetails}</pre> : undefined}
    </div>
  );
};

export default ErrorDetails;
