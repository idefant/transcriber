import { type FC, useEffect, useMemo, useState } from 'react';
import { Empty, Form, Select, Switch } from 'antd';
import { sortBy } from 'lodash-es';
import { useTranslation } from 'react-i18next';

import * as providersApi from '#/shared/providersApi';

import PromptField from './PromptField';

import styles from './ProcessingSettingsForm.module.scss';

import type { ModelTask } from '#/models/Catalog';
import type { OpenRouterProviderOption } from '#/models/Provider';
import { useAppSettings, useCatalog, useProcessing, useProviders } from '#/stores';

const AUTO_OPENROUTER_PROVIDER = '';

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

  // Провайдеры, у которых есть хотя бы одна подобранная модель для этой задачи
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

  // Подобранные модели для выбранного провайдера
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

  // Автоматически сохранять значения по умолчанию только для «чистого» случая, когда ещё ничего не выбрано.
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

  const isOpenRouterSelected = !isStt && selectedProvider?.provider === 'openrouter';
  const openrouterApiModelId = useMemo(() => {
    if (!isOpenRouterSelected || !selectedModelKey) return;

    return catalog
      .find((model) => model.key === selectedModelKey)
      ?.providerEntries.find((entry) => entry.provider === 'openrouter')?.apiId;
  }, [catalog, isOpenRouterSelected, selectedModelKey]);

  const [openrouterProviderOptions, setOpenrouterProviderOptions] = useState<
    OpenRouterProviderOption[]
  >([]);
  const [isLoadingOpenrouterProviders, setIsLoadingOpenrouterProviders] = useState(false);

  const sortedOpenrouterProviderOptions = useMemo(
    () => sortBy(openrouterProviderOptions, 'label'),
    [openrouterProviderOptions],
  );

  // Список апстрим-провайдеров зависит от конкретной модели OpenRouter, поэтому запрашивается заново при её смене.
  useEffect(() => {
    let cancelled = false;

    const load = async () => {
      if (!selectedProviderId || !openrouterApiModelId) {
        if (!cancelled) setOpenrouterProviderOptions([]);
        return;
      }

      if (!cancelled) setIsLoadingOpenrouterProviders(true);

      try {
        const options = await providersApi.listOpenRouterModelProviders(
          selectedProviderId,
          openrouterApiModelId,
        );

        if (!cancelled) setOpenrouterProviderOptions(options);
      } catch {
        if (!cancelled) setOpenrouterProviderOptions([]);
      } finally {
        if (!cancelled) setIsLoadingOpenrouterProviders(false);
      }
    };

    queueMicrotask(() => {
      void load();
    });

    return () => {
      cancelled = true;
    };
  }, [openrouterApiModelId, selectedProviderId]);

  const handleProviderChange = (providerId: string) => {
    if (isStt) {
      void updateSttConfig({ modelKey: null, providerId });
    } else {
      void updatePostProcessConfig({ modelKey: null, openrouterProvider: null, providerId });
    }
  };

  const handleModelChange = (modelKey: string) => {
    if (isStt) {
      void updateSttConfig({ modelKey });
    } else {
      void updatePostProcessConfig({ modelKey, openrouterProvider: null });
    }
  };

  const handleOpenrouterProviderChange = (value: string) => {
    void updatePostProcessConfig({
      openrouterProvider: value === AUTO_OPENROUTER_PROVIDER ? null : value,
    });
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

      {isOpenRouterSelected && selectedModelKey !== undefined && (
        <Form.Item label={t('settings.processing.openrouterProvider')}>
          <Select
            loading={isLoadingOpenrouterProviders}
            options={[
              {
                label: t('settings.processing.openrouterProviderAuto'),
                value: AUTO_OPENROUTER_PROVIDER,
              },
              ...sortedOpenrouterProviderOptions,
            ]}
            showSearch={{
              filterOption: (input, option) =>
                (option?.label ?? '').toLowerCase().includes(input.toLowerCase()),
            }}
            value={config.postProcess.openrouterProvider ?? AUTO_OPENROUTER_PROVIDER}
            onChange={handleOpenrouterProviderChange}
          />
        </Form.Item>
      )}

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
