import { type FC } from 'react';
import { Button, Space, Tooltip, Typography } from 'antd';
import { CopyIcon, LoaderCircleIcon, RotateCcwIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import ErrorDetails from './ErrorDetails';

import styles from './ModelResult.module.scss';

import type { ProcessingDetails } from '#/models/History';

const { Paragraph, Title } = Typography;

interface ModelResultProps {
  canCopy: boolean;
  canRepeat: boolean;
  copyLabel: string;
  details: ProcessingDetails;
  onCopy: () => void;
  onRepeat: () => void;
  repeatLabel: string;
  showBody?: boolean;
  title: string;
}

const ModelResult: FC<ModelResultProps> = ({
  canCopy,
  canRepeat,
  copyLabel,
  details,
  onCopy,
  onRepeat,
  repeatLabel,
  showBody = true,
  title,
}) => {
  const { t } = useTranslation();
  const displayText = details.errorMessage ?? details.text;
  const displayCost =
    typeof details.cost === 'string' &&
    details.cost.length > 0 &&
    details.cost !== '—' &&
    details.cost !== '-'
      ? details.cost
      : undefined;

  return (
    <section className={styles.modelResult}>
      <div className={styles.header}>
        <Title className={styles.title} level={5}>
          {title}
        </Title>
        <Space size={4}>
          <Tooltip title={copyLabel}>
            <Button
              aria-label={copyLabel}
              disabled={!canCopy}
              icon={<CopyIcon size={16} strokeWidth={2} />}
              size="small"
              onClick={onCopy}
            />
          </Tooltip>
          <Tooltip title={repeatLabel}>
            <Button
              aria-label={repeatLabel}
              icon={
                details.isProcessing ? (
                  <LoaderCircleIcon className={styles.spinIcon} size={16} strokeWidth={2} />
                ) : (
                  <RotateCcwIcon size={16} strokeWidth={2} />
                )
              }
              disabled={details.isProcessing || details.status === 'processing' || !canRepeat}
              size="small"
              onClick={onRepeat}
            />
          </Tooltip>
        </Space>
      </div>
      {showBody ? (
        <>
          <dl className={styles.metaList}>
            {details.provider.length > 0 ? (
              <div>
                <dt>{t('history.details.provider')}</dt>
                <dd>{details.provider}</dd>
              </div>
            ) : undefined}
            {details.model.length > 0 ? (
              <div>
                <dt>{t('history.details.model')}</dt>
                <dd>{details.model}</dd>
              </div>
            ) : undefined}
            {details.durationMs === null || details.durationMs === undefined ? undefined : (
              <div>
                <dt>{t('history.details.time')}</dt>
                <dd>{details.duration}</dd>
              </div>
            )}
            {displayCost === undefined ? undefined : (
              <div>
                <dt>{t('history.details.cost')}</dt>
                <dd>{displayCost}</dd>
              </div>
            )}
          </dl>
          <div>
            <Paragraph className={details.status === 'error' ? styles.errorText : styles.text}>
              {displayText}
            </Paragraph>
            {details.status === 'error' && details.errorDetails != null ? (
              <ErrorDetails details={details.errorDetails} />
            ) : undefined}
          </div>
        </>
      ) : undefined}
    </section>
  );
};

export default ModelResult;
