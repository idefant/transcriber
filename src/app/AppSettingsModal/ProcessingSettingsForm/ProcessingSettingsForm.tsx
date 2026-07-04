import { type FC, useEffect, useMemo } from 'react';
import { Empty, Form, Select, Switch } from 'antd';
import { useTranslation } from 'react-i18next';

import PromptField from './PromptField';

import styles from './ProcessingSettingsForm.module.scss';

import type { ModelTask } from '#/models/Catalog';
import { useAppSettings, useCatalog, useProcessing, useProviders } from '#/stores';

interface ProcessingSettingsFormProps {
  disabled?: boolean;
  task: ModelTask;
}

interface ModelOption {
  label: string;
  value: string;
}

interface RecommendedModelOption extends ModelOption {
  isRecommended: boolean;
}

interface ModelOptionGroup {
  label: string;
  options: ModelOption[];
}

const ProcessingSettingsForm: FC<ProcessingSettingsFormProps> = ({ disabled = false, task }) => {
  const { providers } = useProviders();
  const { settings } = useAppSettings();
  const { config, defaultPrompts, loadDefaultPrompts, updateSttConfig, updatePostProcessConfig } =
    useProcessing();
  const { catalog, isLoading: isCatalogLoading } = useCatalog();
  const { t } = useTranslation();
  const languageOptions = [
    { label: t('settings.processing.languages.auto'), value: 'auto' },
    { label: t('settings.processing.languages.ru'), value: 'ru' },
    { label: t('settings.processing.languages.en'), value: 'en' },
  ];

  useEffect(() => {
    void loadDefaultPrompts().catch(() => {});
  }, [config.stt.language, loadDefaultPrompts, settings.effectiveUiLanguage]);

  const isStt = task === 'stt';
  const currentConfig = isStt ? config.stt : config.postProcess;
  const storedProviderId = currentConfig.providerId;
  const storedModelKey = currentConfig.modelKey;
  const shouldAutofillDefaultSelection = storedProviderId === null && storedModelKey === null;

  // Providers that have at least one curated model for this task
  const compatibleProviders = useMemo(() => {
    const compatibleKinds = new Set(
      catalog.filter((m) => m.task === task).flatMap((m) => m.providerKinds),
    );

    return providers.filter((p) => compatibleKinds.has(p.provider));
  }, [catalog, providers, task]);

  const selectedProvider = useMemo(() => {
    const provider = compatibleProviders.find((p) => p.id === (storedProviderId ?? ''));

    if (provider) {
      return provider;
    }

    return shouldAutofillDefaultSelection ? compatibleProviders[0] : undefined;
  }, [compatibleProviders, shouldAutofillDefaultSelection, storedProviderId]);

  // Curated models for the selected provider
  const modelOptions = useMemo<(ModelOption | ModelOptionGroup)[]>(() => {
    if (!selectedProvider) return [];

    const options: RecommendedModelOption[] = [];

    for (const model of catalog) {
      if (model.task !== task) {
        continue;
      }

      const entry = model.providerEntries.find(
        (item) => item.provider === selectedProvider.provider,
      );

      if (!entry) {
        continue;
      }

      options.push({
        isRecommended: entry.isRecommended,
        label: model.label,
        value: model.key,
      });
    }

    const recommended = options
      .filter((model) => model.isRecommended)
      .map(({ label, value }) => ({ label, value }));
    const unrecommended = options
      .filter((model) => !model.isRecommended)
      .map(({ label, value }) => ({ label, value }));

    if (unrecommended.length === 0) {
      return recommended;
    }

    return [
      ...recommended,
      {
        label: t('settings.processing.unrecommendedModels'),
        options: unrecommended,
      },
    ];
  }, [catalog, selectedProvider, task, t]);

  const selectableModelOptions = useMemo(
    () => modelOptions.flatMap((option) => ('options' in option ? option.options : [option])),
    [modelOptions],
  );

  const providerOptions = compatibleProviders.map((p) => ({ label: p.name, value: p.id }));
  const selectedProviderId = selectedProvider?.id;
  const selectedModelKey = useMemo(() => {
    if (
      storedModelKey &&
      selectableModelOptions.some((option) => option.value === storedModelKey)
    ) {
      return storedModelKey;
    }

    return shouldAutofillDefaultSelection
      ? (selectableModelOptions[0]?.value ?? undefined)
      : undefined;
  }, [selectableModelOptions, shouldAutofillDefaultSelection, storedModelKey]);

  // Auto-persist defaults only for the pristine "nothing selected yet" case.
  useEffect(() => {
    if (!shouldAutofillDefaultSelection || !selectedProviderId || !selectedModelKey) return;

    const update = isStt ? updateSttConfig : updatePostProcessConfig;

    void update({ modelKey: selectedModelKey, providerId: selectedProviderId });
  }, [
    isStt,
    selectedModelKey,
    selectedProviderId,
    shouldAutofillDefaultSelection,
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

  const resetSystemPrompt = () => {
    if (isStt) {
      void updateSttConfig({ systemPrompt: null });
    } else {
      void updatePostProcessConfig({ systemPrompt: null });
    }
  };

  const persistUserPromptTemplate = (userPromptTemplate: string) => {
    void updatePostProcessConfig({ userPromptTemplate });
  };
  const resetUserPromptTemplate = () => {
    void updatePostProcessConfig({ userPromptTemplate: null });
  };

  const defaultSystemPrompt =
    (isStt ? defaultPrompts?.sttSystem : defaultPrompts?.postProcessSystem) ?? '';
  const defaultUserPromptTemplate = defaultPrompts?.postProcessUserTemplate ?? '';
  const isProviderMissing = !disabled && selectedProviderId === undefined;
  const isModelMissing =
    !disabled && selectedProviderId !== undefined && selectedModelKey === undefined;

  if (isCatalogLoading) {
    return null;
  }

  if (compatibleProviders.length === 0) {
    return (
      <Empty
        description={t('settings.processing.noProviders')}
        image={Empty.PRESENTED_IMAGE_SIMPLE}
      />
    );
  }

  return (
    <Form disabled={disabled} layout="vertical">
      <Form.Item
        label={t('settings.processing.provider')}
        validateStatus={isProviderMissing ? 'error' : undefined}
      >
        <Select
          options={providerOptions}
          placeholder={t('settings.processing.providerPlaceholder')}
          value={selectedProviderId}
          onChange={handleProviderChange}
        />
      </Form.Item>

      <Form.Item
        label={t('settings.processing.model')}
        validateStatus={isModelMissing ? 'error' : undefined}
      >
        <Select
          disabled={selectedProviderId === undefined}
          notFoundContent={t('settings.processing.noModels')}
          options={modelOptions}
          placeholder={t('settings.processing.modelPlaceholder')}
          value={selectedModelKey}
          onChange={handleModelChange}
        />
      </Form.Item>

      {isStt && (
        <Form.Item label={t('settings.processing.language')}>
          <Select
            options={languageOptions}
            placeholder={t('settings.processing.languagePlaceholder')}
            value={config.stt.language}
            onChange={handleLanguageChange}
          />
        </Form.Item>
      )}

      <Form.Item>
        <div className={styles.switchRow}>
          <Switch checked={useCustomPrompts} onChange={handleUseCustomPromptsChange} />
          <span>{t('settings.processing.useCustomPrompts')}</span>
        </div>
      </Form.Item>

      {defaultPrompts !== undefined && (
        <>
          <PromptField
            key={`${task}-system-${String(useCustomPrompts)}-${defaultSystemPrompt}`}
            defaultValue={defaultSystemPrompt}
            disabled={disabled}
            enabled={useCustomPrompts}
            label={t('settings.processing.systemPrompt')}
            placeholder={t('settings.processing.systemPromptPlaceholder')}
            resetLabel={t('settings.processing.resetPrompt')}
            storedValue={currentConfig.systemPrompt}
            onPersist={persistSystemPrompt}
            onReset={resetSystemPrompt}
          />

          {!isStt && (
            <PromptField
              key={`post-process-user-${String(useCustomPrompts)}-${defaultUserPromptTemplate}`}
              defaultValue={defaultUserPromptTemplate}
              disabled={disabled}
              enabled={useCustomPrompts}
              hint={t('settings.processing.userPromptTemplateHint')}
              label={t('settings.processing.userPromptTemplate')}
              placeholder={t('settings.processing.userPromptTemplatePlaceholder')}
              resetLabel={t('settings.processing.resetPrompt')}
              storedValue={config.postProcess.userPromptTemplate}
              onPersist={persistUserPromptTemplate}
              onReset={resetUserPromptTemplate}
            />
          )}
        </>
      )}
    </Form>
  );
};

export default ProcessingSettingsForm;
