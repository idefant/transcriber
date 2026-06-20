import { type FC, useCallback, useEffect, useMemo } from 'react';
import {
  Button,
  Form,
  Input,
  Modal,
  Radio,
  Space,
  Switch,
  Table,
  type TableColumnsType,
  Tag,
} from 'antd';
import { CheckCircleIcon, SparklesIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import { providerOptions } from '../constants';

import styles from './ProviderSettingsModal.module.scss';

import type { CuratedModelInfo } from '#/models/Catalog';
import type {
  ModelInfo,
  ProviderConfig,
  ProviderConnectionInput,
  ProviderInput,
  ProviderKind,
} from '#/models/Provider';

interface ProviderSettingsFormValues {
  apiKey?: string;
  areAdvancedSettingsEnabled?: boolean;
  baseUrl?: string;
  headers?: string;
  name?: string;
  provider: ProviderKind;
}

interface CatalogRow {
  apiId: string;
  key: string;
  label: string;
  supported: boolean | null;
}

interface ProviderSettingsModalProps {
  catalog: CuratedModelInfo[];
  editingProvider?: ProviderConfig;
  isLoadingModels: boolean;
  isModelListVisible: boolean;
  isSaving: boolean;
  isValidating: boolean;
  modelRows: ModelInfo[];
  okText: string;
  open: boolean;
  title: string;
  onCancel: () => void;
  onLoadModels: (input: ProviderConnectionInput) => Promise<void>;
  onModelListHide: () => void;
  onSubmit: (input: ProviderInput) => Promise<void>;
  onValidate: (input: ProviderConnectionInput) => Promise<void>;
}

const ProviderSettingsModal: FC<ProviderSettingsModalProps> = ({
  catalog,
  editingProvider,
  isLoadingModels,
  isModelListVisible,
  isSaving,
  isValidating,
  modelRows,
  okText,
  open,
  title,
  onCancel,
  onLoadModels,
  onModelListHide,
  onSubmit,
  onValidate,
}) => {
  const { t } = useTranslation();
  const [form] = Form.useForm<ProviderSettingsFormValues>();
  const selectedProvider =
    (Form.useWatch('provider', form) as ProviderKind | undefined) ?? 'openai';
  const areAdvancedSettingsEnabled = Form.useWatch('areAdvancedSettingsEnabled', form) ?? false;
  const canUseAdvancedSettings = selectedProvider !== 'custom';
  const isApiKeyRequired = !editingProvider?.hasApiKey;
  const getProviderLabel = useCallback(
    (provider: ProviderKind) => {
      const option = providerOptions.find(({ value }) => value === provider);

      if (option?.value === 'custom') {
        return t('common.custom');
      }

      return option?.label ?? t('common.custom');
    },
    [t],
  );
  const apiKeyPlaceholder =
    editingProvider?.hasApiKey === true
      ? t('settings.providers.modal.keySaved', { preview: editingProvider.keyPreview })
      : t('settings.providers.modal.keyPlaceholder');
  const tokenPlaceholder =
    editingProvider?.hasApiKey === true
      ? t('settings.providers.modal.tokenSaved', { preview: editingProvider.keyPreview })
      : t('settings.providers.modal.tokenPlaceholder');

  useEffect(() => {
    if (!open) {
      return;
    }

    form.setFieldsValue({
      apiKey: '',
      areAdvancedSettingsEnabled: editingProvider?.useAdvancedSettings ?? false,
      baseUrl: editingProvider?.baseUrl ?? '',
      headers: editingProvider?.headers ?? '',
      name:
        editingProvider === undefined ||
        editingProvider.name === getProviderLabel(editingProvider.provider)
          ? ''
          : editingProvider.name,
      provider: editingProvider?.provider ?? 'openai',
    });
  }, [editingProvider, form, getProviderLabel, open]);

  // Curated models for the selected provider kind — derived, no state needed
  const catalogRows = useMemo<CatalogRow[]>(
    () =>
      catalog
        .map((model) => {
          const providerEntry = model.providerEntries.find(
            (entry) => entry.provider === selectedProvider,
          );

          if (!providerEntry) {
            return;
          }

          return {
            apiId: providerEntry.apiId,
            key: model.key,
            label: model.label,
            supported: null as boolean | null,
          };
        })
        .filter((model): model is CatalogRow => model !== undefined),
    [catalog, selectedProvider],
  );

  // When modelRows arrive, match them against our catalog rows
  const resolvedCatalogRows = useMemo<CatalogRow[]>(() => {
    if (!isModelListVisible || modelRows.length === 0) {
      return catalogRows;
    }

    const availableIds = new Set(modelRows.map((model) => model.name.toLowerCase()));

    return catalogRows.map((row) => {
      const isSupported = availableIds.has(row.apiId.toLowerCase());

      return { ...row, supported: isSupported };
    });
  }, [catalogRows, isModelListVisible, modelRows]);

  const modelColumns = useMemo<TableColumnsType<CatalogRow>>(
    () => [
      {
        dataIndex: 'label',
        title: t('settings.providers.modal.model'),
      },
      {
        align: 'right',
        render: (_, row) => {
          if (row.supported === null) {
            return null;
          }

          return row.supported ? (
            <Tag color="success">{t('settings.providers.modal.supported')}</Tag>
          ) : (
            <Tag color="default">{t('settings.providers.modal.unsupported')}</Tag>
          );
        },
        title: '',
        width: 180,
      },
    ],
    [t],
  );

  const buildConnectionInput = async (): Promise<ProviderConnectionInput> => {
    const values = await form.validateFields();
    const isCustomProvider = values.provider === 'custom';
    const shouldUseAdvancedSettings =
      isCustomProvider || values.areAdvancedSettingsEnabled === true;

    return {
      apiKey: values.apiKey,
      baseUrl: shouldUseAdvancedSettings ? values.baseUrl : undefined,
      headers: shouldUseAdvancedSettings ? values.headers : undefined,
      provider: values.provider,
      providerId: editingProvider?.id,
      useAdvancedSettings: shouldUseAdvancedSettings,
    };
  };

  const handleValidate = async () => {
    await onValidate(await buildConnectionInput());
  };

  const handleLoadModels = async () => {
    if (isModelListVisible) {
      onModelListHide();
      return;
    }

    await onLoadModels(await buildConnectionInput());
  };

  const handleSubmit = async () => {
    await form.validateFields();

    const values = form.getFieldsValue(true) as ProviderSettingsFormValues;

    await onSubmit({
      apiKey: values.apiKey,
      baseUrl: values.baseUrl,
      headers: values.headers,
      name: values.name,
      provider: values.provider,
      useAdvancedSettings:
        values.provider === 'custom' || values.areAdvancedSettingsEnabled === true,
    });
  };

  return (
    <Modal
      confirmLoading={isSaving}
      okText={okText}
      open={open}
      title={title}
      width={760}
      onCancel={onCancel}
      onOk={() => {
        void handleSubmit();
      }}
    >
      <div className={styles.providerCard}>
        <Form form={form} layout="vertical">
          <Form.Item label={t('settings.providers.modal.provider')} name="provider">
            <Radio.Group
              className={styles.providerRadioGroup}
              buttonStyle="solid"
              onChange={() => {
                onModelListHide();
              }}
            >
              {providerOptions.map((providerOption) => (
                <Radio.Button key={providerOption.value} value={providerOption.value}>
                  {providerOption.value === 'custom' ? t('common.custom') : providerOption.label}
                </Radio.Button>
              ))}
            </Radio.Group>
          </Form.Item>

          <Form.Item label={t('settings.providers.modal.name')} name="name">
            <Input placeholder={getProviderLabel(selectedProvider)} />
          </Form.Item>

          {selectedProvider === 'custom' ? (
            <>
              <Form.Item label="URL" name="baseUrl" rules={[{ required: true }]}>
                <Input placeholder="https://api.example.com/v1" />
              </Form.Item>
              <Form.Item
                label={t('settings.providers.modal.token')}
                name="apiKey"
                rules={[{ required: isApiKeyRequired }]}
              >
                <Input.Password placeholder={tokenPlaceholder} />
              </Form.Item>
              <Form.Item label={t('settings.providers.modal.headers')} name="headers">
                <Input.TextArea
                  className={styles.headersInput}
                  placeholder="X-Api-Gateway: transcriber&#10;X-Workspace: default"
                />
              </Form.Item>
            </>
          ) : (
            <>
              <Form.Item
                label={t('settings.providers.modal.key')}
                name="apiKey"
                rules={[{ required: isApiKeyRequired }]}
              >
                <Input.Password placeholder={apiKeyPlaceholder} />
              </Form.Item>
              <Form.Item>
                <div className={styles.advancedToggle}>
                  <Form.Item name="areAdvancedSettingsEnabled" noStyle valuePropName="checked">
                    <Switch disabled={!canUseAdvancedSettings} />
                  </Form.Item>
                  <span>{t('settings.providers.modal.advanced')}</span>
                </div>
              </Form.Item>
              {areAdvancedSettingsEnabled && (
                <>
                  <Form.Item label={t('settings.providers.modal.customUrl')} name="baseUrl">
                    <Input placeholder="https://api.example.com/v1" />
                  </Form.Item>
                  <Form.Item label={t('settings.providers.modal.extraHeaders')} name="headers">
                    <Input.TextArea
                      className={styles.headersInput}
                      placeholder="X-Api-Gateway: transcriber&#10;Authorization: Bearer custom-token"
                    />
                  </Form.Item>
                </>
              )}
            </>
          )}
        </Form>

        <Space className={styles.modelActions}>
          <Button
            icon={<CheckCircleIcon size={18} strokeWidth={2} />}
            loading={isValidating}
            onClick={() => {
              void handleValidate();
            }}
          >
            {t('settings.providers.modal.validate')}
          </Button>
          <Button
            icon={<SparklesIcon size={18} strokeWidth={2} />}
            loading={isLoadingModels}
            onClick={() => {
              void handleLoadModels();
            }}
          >
            {isModelListVisible
              ? t('settings.providers.modal.hideModels')
              : t('settings.providers.modal.showModels')}
          </Button>
        </Space>

        {isModelListVisible && catalogRows.length > 0 && (
          <div className={styles.modelList}>
            <Table
              columns={modelColumns}
              dataSource={resolvedCatalogRows}
              pagination={false}
              rowKey="key"
              scroll={{ y: 280 }}
              size="small"
            />
          </div>
        )}
      </div>
    </Modal>
  );
};

export default ProviderSettingsModal;
