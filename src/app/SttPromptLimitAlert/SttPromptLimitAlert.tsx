import { type FC, type ReactNode, useEffect, useState } from 'react';
import { Alert } from 'antd';
import { useTranslation } from 'react-i18next';

import * as processingApi from '#/shared/processingApi';

import { useDictionaryStore, useProcessing } from '#/stores';

interface SttPromptLimitAnalysisState {
  analysis: processingApi.SttPromptAnalysis | null;
  inputKey: string;
}

interface SttPromptLimitAlertProps {
  action?: ReactNode;
  draftSystemPrompt?: string;
  exceededDescription: string;
}

/** Показывает состояние лимита итогового prompt Speech-to-Text и действие для его исправления. */
const SttPromptLimitAlert: FC<SttPromptLimitAlertProps> = ({
  action,
  draftSystemPrompt,
  exceededDescription,
}) => {
  const { t } = useTranslation();
  const [analysisState, setAnalysisState] = useState<SttPromptLimitAnalysisState | null>(null);
  const dictionaryWords = useDictionaryStore((state) => state.words);
  const { config } = useProcessing();
  const inputKey = JSON.stringify([
    config.stt.modelKey,
    config.stt.systemPrompt,
    config.stt.useCustomPrompt,
    dictionaryWords,
    draftSystemPrompt,
  ]);

  useEffect(() => {
    let cancelled = false;

    void processingApi
      .analyzeSttPrompt(draftSystemPrompt)
      .then((nextAnalysis) => {
        if (!cancelled) setAnalysisState({ analysis: nextAnalysis, inputKey });
        return;
      })
      .catch(() => {
        if (!cancelled) setAnalysisState({ analysis: null, inputKey });
      });

    return () => {
      cancelled = true;
    };
  }, [draftSystemPrompt, inputKey]);

  // Пока новый анализ выполняется, сохраняем предыдущий результат вместо краткого
  // исчезновения уведомления. Устаревший запрос не сможет его перезаписать из-за cleanup эффекта.
  const analysis = analysisState?.analysis ?? null;

  if (analysis === null) {
    return null;
  }

  const isExceeded = analysis.excludedTokenCount > 0;
  const interpolation = {
    count: analysis.tokenCount,
    limit: analysis.limit,
    percent: Math.round(analysis.usagePercent),
  };

  return (
    <Alert
      action={isExceeded ? action : undefined}
      description={isExceeded ? exceededDescription : undefined}
      showIcon
      title={
        isExceeded
          ? t('settings.processing.sttPromptLimitExceeded', interpolation)
          : t('settings.processing.sttPromptLimitWithin', interpolation)
      }
      type={isExceeded ? 'error' : 'success'}
    />
  );
};

export default SttPromptLimitAlert;
