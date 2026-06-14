import { type FC } from 'react';
import { Button, Space, Tooltip, Typography } from 'antd';
import { CopyIcon, LoaderCircleIcon, RotateCcwIcon } from 'lucide-react';

import styles from './ModelResult.module.scss';

import type { ProcessingDetails } from '#/models/History';

const { Paragraph, Title } = Typography;

interface ModelResultProps {
  copyLabel: string;
  details: ProcessingDetails;
  repeatLabel: string;
  title: string;
}

const ModelResult: FC<ModelResultProps> = ({ copyLabel, details, repeatLabel, title }) => (
  <section className={styles.modelResult}>
    <div className={styles.header}>
      <Title className={styles.title} level={5}>
        {title}
      </Title>
      <Space size={4}>
        <Tooltip title={copyLabel}>
          <Button
            aria-label={copyLabel}
            icon={<CopyIcon size={16} strokeWidth={2} />}
            size="small"
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
            disabled={details.isProcessing}
            size="small"
          />
        </Tooltip>
      </Space>
    </div>
    <dl className={styles.metaList}>
      <div>
        <dt>Провайдер</dt>
        <dd>{details.provider}</dd>
      </div>
      <div>
        <dt>Модель</dt>
        <dd>{details.model}</dd>
      </div>
      <div>
        <dt>Время</dt>
        <dd>{details.duration}</dd>
      </div>
      <div>
        <dt>Стоимость</dt>
        <dd>{details.cost}</dd>
      </div>
    </dl>
    <Paragraph className={styles.text}>{details.text}</Paragraph>
  </section>
);

export default ModelResult;
