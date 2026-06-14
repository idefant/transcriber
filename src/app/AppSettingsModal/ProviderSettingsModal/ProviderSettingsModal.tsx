import { type FC, useMemo } from 'react';
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
} from 'antd';
import { CheckCircleIcon, SparklesIcon, StarIcon } from 'lucide-react';

import { providerOptions } from '../constants';

import styles from './ProviderSettingsModal.module.scss';

import type { ModelInfo, ProviderKind } from '#/models/Provider';

interface ProviderSettingsModalProps {
  areAdvancedSettingsEnabled: boolean;
  canUseAdvancedSettings: boolean;
  favoriteModels: Set<string>;
  isModelListVisible: boolean;
  modelRows: ModelInfo[];
  okText: string;
  open: boolean;
  selectedProvider: ProviderKind;
  title: string;
  onAdvancedSettingsEnabledChange: (value: boolean) => void;
  onCancel: () => void;
  onFavoriteModelToggle: (modelName: string) => void;
  onModelListVisibleToggle: () => void;
  onProviderChange: (value: ProviderKind) => void;
  onSubmit: () => void;
}

const ProviderSettingsModal: FC<ProviderSettingsModalProps> = ({
  areAdvancedSettingsEnabled,
  canUseAdvancedSettings,
  favoriteModels,
  isModelListVisible,
  modelRows,
  okText,
  open,
  selectedProvider,
  title,
  onAdvancedSettingsEnabledChange,
  onCancel,
  onFavoriteModelToggle,
  onModelListVisibleToggle,
  onProviderChange,
  onSubmit,
}) => {
  const modelColumns = useMemo<TableColumnsType<ModelInfo>>(
    () => [
      {
        dataIndex: 'name',
        title: 'Модель',
      },
      {
        dataIndex: 'description',
        title: 'Описание',
      },
      {
        render: (_, model) => {
          const isFavorite = favoriteModels.has(model.name);

          return (
            <Button
              aria-label={isFavorite ? 'Убрать из избранного' : 'Добавить в избранное'}
              icon={
                <StarIcon fill={isFavorite ? 'currentColor' : 'none'} size={18} strokeWidth={2} />
              }
              type={isFavorite ? 'primary' : 'text'}
              onClick={() => {
                onFavoriteModelToggle(model.name);
              }}
            />
          );
        },
        title: 'Избранное',
        width: 120,
      },
    ],
    [favoriteModels, onFavoriteModelToggle],
  );

  return (
    <Modal
      okText={okText}
      open={open}
      title={title}
      width={760}
      onCancel={onCancel}
      onOk={onSubmit}
    >
      <div className={styles.providerCard}>
        <Form layout="vertical">
          <Form.Item label="Провайдер">
            <Radio.Group
              className={styles.providerRadioGroup}
              value={selectedProvider}
              buttonStyle="solid"
              onChange={(event) => {
                onProviderChange(event.target.value as ProviderKind);
              }}
            >
              {providerOptions.map((providerOption) => (
                <Radio.Button key={providerOption.value} value={providerOption.value}>
                  {providerOption.label}
                </Radio.Button>
              ))}
            </Radio.Group>
          </Form.Item>

          {selectedProvider === 'custom' ? (
            <>
              <Form.Item label="URL">
                <Input placeholder="https://api.example.com/v1" />
              </Form.Item>
              <Form.Item label="Токен">
                <Input.Password placeholder="Введите токен" />
              </Form.Item>
              <Form.Item label="Заголовки запроса">
                <Input.TextArea
                  className={styles.headersInput}
                  placeholder="X-Api-Gateway: transcriber&#10;X-Workspace: default"
                />
              </Form.Item>
            </>
          ) : (
            <>
              <Form.Item label="Ключ">
                <Input.Password placeholder="Введите API key" />
              </Form.Item>
              <Form.Item>
                <div className={styles.advancedToggle}>
                  <Switch
                    checked={areAdvancedSettingsEnabled}
                    disabled={!canUseAdvancedSettings}
                    onChange={onAdvancedSettingsEnabledChange}
                  />
                  <span>Дополнительные параметры</span>
                </div>
              </Form.Item>
              {areAdvancedSettingsEnabled && (
                <>
                  <Form.Item label="Custom URL">
                    <Input placeholder="https://api.example.com/v1" />
                  </Form.Item>
                  <Form.Item label="Дополнительные заголовки">
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
          <Button icon={<CheckCircleIcon size={18} strokeWidth={2} />}>
            Проверить валидность конфигурации
          </Button>
          <Button
            icon={<SparklesIcon size={18} strokeWidth={2} />}
            onClick={onModelListVisibleToggle}
          >
            Показать список моделей
          </Button>
        </Space>

        {isModelListVisible && (
          <Table
            columns={modelColumns}
            dataSource={modelRows}
            pagination={false}
            rowKey="name"
            size="small"
          />
        )}
      </div>
    </Modal>
  );
};

export default ProviderSettingsModal;
