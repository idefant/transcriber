import { type FC, useState } from 'react';
import { Alert, Button, Input, Typography } from 'antd';
import { WandSparklesIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import { useProcessing } from '#/app/processingContext';
import * as processingApi from '#/shared/processingApi';

import styles from './PostProcessTestPanel.module.scss';

const PostProcessTestPanel: FC = () => {
  const { config } = useProcessing();
  const { t } = useTranslation();
  const [inputText, setInputText] = useState('');
  const [isRunning, setIsRunning] = useState(false);
  const [result, setResult] = useState<string>();
  const [error, setError] = useState<string>();
  const [elapsedMs, setElapsedMs] = useState(0);

  const canRun = Boolean(
    config.postProcess.providerId && config.postProcess.modelKey && inputText.trim(),
  );
  const formatElapsed = (value: number) =>
    value < 1000
      ? t('common.milliseconds', { value })
      : t('common.seconds', { value: (value / 1000).toFixed(1) });

  const handleRun = async () => {
    if (!config.postProcess.providerId || !config.postProcess.modelKey) return;

    setIsRunning(true);
    setResult(undefined);
    setError(undefined);

    const startedAt = performance.now();

    try {
      const text = await processingApi.runPostProcessTest({ text: inputText });

      setElapsedMs(Math.round(performance.now() - startedAt));
      setResult(text);
    } catch (unknownError) {
      setError(unknownError instanceof Error ? unknownError.message : String(unknownError));
    } finally {
      setIsRunning(false);
    }
  };

  return (
    <div className={styles.panel}>
      <Typography.Text strong>{t('settings.tests.title')}</Typography.Text>

      <Input.TextArea
        className={styles.inputText}
        placeholder={t('settings.tests.inputPlaceholder')}
        value={inputText}
        onChange={(event) => {
          setInputText(event.target.value);
        }}
        autoSize={{ minRows: 2, maxRows: 12 }}
      />

      <div>
        <Button
          disabled={!canRun}
          icon={<WandSparklesIcon size={16} strokeWidth={2} />}
          loading={isRunning}
          type="primary"
          onClick={() => {
            void handleRun();
          }}
        >
          {t('settings.tests.run')}
        </Button>
      </div>

      {(!config.postProcess.providerId || !config.postProcess.modelKey) && (
        <Alert showIcon title={t('settings.processing.selectProviderAndModel')} type="warning" />
      )}

      {error !== undefined && <Alert showIcon title={error} type="error" />}

      {result !== undefined && (
        <div className={styles.result}>
          <Typography.Text type="secondary">
            {t('settings.tests.result', { elapsed: formatElapsed(elapsedMs) })}
          </Typography.Text>
          <Typography.Paragraph className={styles.resultText}>{result}</Typography.Paragraph>
        </div>
      )}
    </div>
  );
};

export default PostProcessTestPanel;
