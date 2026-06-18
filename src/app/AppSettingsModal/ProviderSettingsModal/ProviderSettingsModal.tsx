import { type FC, useEffect, useMemo } from 'react';
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

const getProviderLabel = (provider: ProviderKind) =>
  providerOptions.find(({ value }) => value === provider)?.label ?? 'Custom';

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
  const [form] = Form.useForm<ProviderSettingsFormValues>();
  const selectedProvider =
    (Form.useWatch('provider', form) as ProviderKind | undefined) ?? 'openai';
  const areAdvancedSettingsEnabled = Form.useWatch('areAdvancedSettingsEnabled', form) ?? false;
  const canUseAdvancedSettings = selectedProvider !== 'custom';
  const isApiKeyRequired = !editingProvider?.hasApiKey;
  const apiKeyPlaceholder =
    editingProvider?.hasApiKey === true
      ? `Ключ уже сохранен (${editingProvider.keyPreview})`
      : 'Введите API key';
  const tokenPlaceholder =
    editingProvider?.hasApiKey === true
      ? `Токен уже сохранен (${editingProvider.keyPreview})`
      : 'Введите токен';

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
  }, [editingProvider, form, open]);

  // Curated models for the selected provider kind — derived, no state needed
  const catalogRows = useMemo<CatalogRow[]>(
    () =>
      catalog
        .filter((model) => model.providerKinds.includes(selectedProvider))
        .map((model) => ({
          apiId: '',
          key: model.key,
          label: model.label,
          supported: null as boolean | null,
        })),
    [catalog, selectedProvider],
  );

  // When modelRows arrive, match them against our catalog rows
  const resolvedCatalogRows = useMemo<CatalogRow[]>(() => {
    if (!isModelListVisible || modelRows.length === 0) {
      return catalogRows;
    }

    const availableIds = new Set(modelRows.map((m) => m.name));

    return catalogRows.map((row) => {
      // Check if the model key exists in the catalog for this provider
      const isInCatalog = catalog.some((m) => m.key === row.key);

      if (!isInCatalog) {
        return { ...row, supported: false };
      }

      // We don't have the per-provider api_id on the frontend (only providerKinds).
      // We match by checking if any model name in the response contains the model key
      // or matches common naming conventions.
      const isSupported = [...availableIds].some(
        (id) =>
          id.toLowerCase().includes(row.key.toLowerCase()) ||
          row.key.toLowerCase().includes(id.toLowerCase().split('/').pop() ?? id),
      );

      return { ...row, supported: isSupported };
    });
  }, [catalog, catalogRows, isModelListVisible, modelRows]);

  const modelColumns = useMemo<TableColumnsType<CatalogRow>>(
    () => [
      {
        dataIndex: 'label',
        title: 'Модель',
      },
      {
        align: 'right',
        render: (_, row) => {
          if (row.supported === null) {
            return null;
          }

          return row.supported ? (
            <Tag color="success">Поддерживается</Tag>
          ) : (
            <Tag color="default">Не поддерживается</Tag>
          );
        },
        title: '',
        width: 180,
      },
    ],
    [],
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
          <Form.Item label="Провайдер" name="provider">
            <Radio.Group className={styles.providerRadioGroup} buttonStyle="solid">
              {providerOptions.map((providerOption) => (
                <Radio.Button key={providerOption.value} value={providerOption.value}>
                  {providerOption.label}
                </Radio.Button>
              ))}
            </Radio.Group>
          </Form.Item>

          <Form.Item label="Название" name="name">
            <Input placeholder={getProviderLabel(selectedProvider)} />
          </Form.Item>

          {selectedProvider === 'custom' ? (
            <>
              <Form.Item label="URL" name="baseUrl" rules={[{ required: true }]}>
                <Input placeholder="https://api.example.com/v1" />
              </Form.Item>
              <Form.Item label="Токен" name="apiKey" rules={[{ required: isApiKeyRequired }]}>
                <Input.Password placeholder={tokenPlaceholder} />
              </Form.Item>
              <Form.Item label="Заголовки запроса" name="headers">
                <Input.TextArea
                  className={styles.headersInput}
                  placeholder="X-Api-Gateway: transcriber&#10;X-Workspace: default"
                />
              </Form.Item>
            </>
          ) : (
            <>
              <Form.Item label="Ключ" name="apiKey" rules={[{ required: isApiKeyRequired }]}>
                <Input.Password placeholder={apiKeyPlaceholder} />
              </Form.Item>
              <Form.Item>
                <div className={styles.advancedToggle}>
                  <Form.Item name="areAdvancedSettingsEnabled" noStyle valuePropName="checked">
                    <Switch disabled={!canUseAdvancedSettings} />
                  </Form.Item>
                  <span>Дополнительные параметры</span>
                </div>
              </Form.Item>
              {areAdvancedSettingsEnabled && (
                <>
                  <Form.Item label="Custom URL" name="baseUrl">
                    <Input placeholder="https://api.example.com/v1" />
                  </Form.Item>
                  <Form.Item label="Дополнительные заголовки" name="headers">
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
            Проверить валидность конфигурации
          </Button>
          <Button
            icon={<SparklesIcon size={18} strokeWidth={2} />}
            loading={isLoadingModels}
            onClick={() => {
              void handleLoadModels();
            }}
          >
            {isModelListVisible ? 'Скрыть список моделей' : 'Показать список моделей'}
          </Button>
        </Space>

        {catalogRows.length > 0 && (
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
