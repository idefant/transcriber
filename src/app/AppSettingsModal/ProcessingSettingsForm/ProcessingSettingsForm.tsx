import { type FC, useEffect, useMemo, useState } from 'react';
import { Empty, Form, Select, Switch } from 'antd';

import { useProcessing } from '#/app/processingContext';
import { useProviders } from '#/app/providersContext';
import * as catalogApi from '#/shared/catalogApi';
import * as processingApi from '#/shared/processingApi';

import PromptField from './PromptField';

import styles from './ProcessingSettingsForm.module.scss';

import type { CuratedModelInfo, ModelTask } from '#/models/Catalog';
import type { DefaultPrompts } from '#/models/Processing';

interface ProcessingSettingsFormProps {
  disabled?: boolean;
  task: ModelTask;
}

const LANGUAGE_OPTIONS = [
  { label: 'Авто', value: 'auto' },
  { label: 'Русский', value: 'ru' },
  { label: 'English', value: 'en' },
];

const ProcessingSettingsForm: FC<ProcessingSettingsFormProps> = ({ disabled = false, task }) => {
  const { providers } = useProviders();
  const { config, updateSttConfig, updatePostProcessConfig } = useProcessing();
  const [catalog, setCatalog] = useState<CuratedModelInfo[]>([]);
  const [defaultPrompts, setDefaultPrompts] = useState<DefaultPrompts>();

  useEffect(() => {
    catalogApi
      .getModelCatalog()
      .then(setCatalog)
      .catch(() => {});
  }, []);

  useEffect(() => {
    processingApi
      .getDefaultPrompts()
      .then(setDefaultPrompts)
      .catch(() => {});
  }, []);

  const isStt = task === 'stt';
  const currentConfig = isStt ? config.stt : config.postProcess;
  const selectedModelKey = currentConfig.modelKey ?? '';

  // Providers that have at least one curated model for this task
  const compatibleProviders = useMemo(() => {
    const compatibleKinds = new Set(
      catalog.filter((m) => m.task === task).flatMap((m) => m.providerKinds),
    );

    return providers.filter((p) => compatibleKinds.has(p.provider));
  }, [catalog, providers, task]);

  const selectedProvider =
    compatibleProviders.find((p) => p.id === (currentConfig.providerId ?? '')) ??
    compatibleProviders[0];

  // Curated models for the selected provider
  const modelOptions = useMemo(() => {
    if (!selectedProvider) return [];

    return catalog
      .filter((m) => m.task === task && m.providerKinds.includes(selectedProvider.provider))
      .map((m) => ({ label: m.label, value: m.key }));
  }, [catalog, selectedProvider, task]);

  const providerOptions = compatibleProviders.map((p) => ({ label: p.name, value: p.id }));

  const effectiveProviderId = selectedProvider?.id ?? '';
  const effectiveModelKey = modelOptions.some((o) => o.value === selectedModelKey)
    ? selectedModelKey
    : (modelOptions[0]?.value ?? '');

  // Primitive refs to avoid object identity issues in deps
  const storedProviderId = currentConfig.providerId;
  const storedModelKey = currentConfig.modelKey;

  // Auto-persist defaults so that SttTestPanel / PostProcessTestPanel see non-null values
  // even when the user has never explicitly made a selection (first launch).
  useEffect(() => {
    if (!effectiveProviderId || !effectiveModelKey) return;
    if (storedProviderId && storedModelKey) return;

    const update = isStt ? updateSttConfig : updatePostProcessConfig;

    void update({
      ...(storedProviderId ? {} : { providerId: effectiveProviderId }),
      ...(storedModelKey ? {} : { modelKey: effectiveModelKey }),
    });
  }, [
    effectiveModelKey,
    effectiveProviderId,
    isStt,
    storedModelKey,
    storedProviderId,
    updatePostProcessConfig,
    updateSttConfig,
  ]);

  const handleProviderChange = (providerId: string) => {
    const update = isStt ? updateSttConfig : updatePostProcessConfig;

    void update({ modelKey: null, providerId });
  };

  const handleModelChange = (modelKey: string) => {
    const update = isStt ? updateSttConfig : updatePostProcessConfig;

    void update({ modelKey });
  };

  const handleLanguageChange = (language: string) => {
    void updateSttConfig({ language });
  };

  const useCustomPrompts = isStt ? config.stt.useCustomPrompt : config.postProcess.useCustomPrompts;

  const handleUseCustomPromptsChange = (checked: boolean) => {
    if (isStt) {
      void updateSttConfig({ useCustomPrompt: checked });
    } else {
      void updatePostProcessConfig({ useCustomPrompts: checked });
    }
  };

  const persistSystemPrompt = (systemPrompt: string) => {
    if (isStt) {
      void updateSttConfig({ systemPrompt });
    } else {
      void updatePostProcessConfig({ systemPrompt });
    }
  };

  const persistUserPromptTemplate = (userPromptTemplate: string) => {
    void updatePostProcessConfig({ userPromptTemplate });
  };

  const defaultSystemPrompt =
    (isStt ? defaultPrompts?.sttSystem : defaultPrompts?.postProcessSystem) ?? '';

  if (compatibleProviders.length === 0) {
    return <Empty description="Сначала добавьте провайдера" image={Empty.PRESENTED_IMAGE_SIMPLE} />;
  }

  return (
    <Form disabled={disabled} layout="vertical">
      <Form.Item label="Провайдер">
        <Select
          options={providerOptions}
          value={effectiveProviderId}
          onChange={handleProviderChange}
        />
      </Form.Item>

      <Form.Item label="Модель">
        <Select
          notFoundContent="Нет доступных моделей для этого провайдера"
          options={modelOptions}
          value={effectiveModelKey}
          onChange={handleModelChange}
        />
      </Form.Item>

      {isStt && (
        <Form.Item label="Язык">
          <Select
            options={LANGUAGE_OPTIONS}
            value={config.stt.language}
            onChange={handleLanguageChange}
          />
        </Form.Item>
      )}

      <Form.Item>
        <div className={styles.switchRow}>
          <Switch checked={useCustomPrompts} onChange={handleUseCustomPromptsChange} />
          <span>Использовать кастомные промпты</span>
        </div>
      </Form.Item>

      <PromptField
        defaultValue={defaultSystemPrompt}
        disabled={disabled}
        enabled={useCustomPrompts}
        label="Системный промпт"
        storedValue={currentConfig.systemPrompt}
        onPersist={persistSystemPrompt}
      />

      {!isStt && (
        <PromptField
          defaultValue={defaultPrompts?.postProcessUserTemplate ?? ''}
          disabled={disabled}
          enabled={useCustomPrompts}
          hint="Оставьте пустым, если не хотите использовать пользовательский шаблон."
          label="Шаблон пользовательского промпта"
          storedValue={config.postProcess.userPromptTemplate}
          onPersist={persistUserPromptTemplate}
        />
      )}
    </Form>
  );
};

export default ProcessingSettingsForm;
