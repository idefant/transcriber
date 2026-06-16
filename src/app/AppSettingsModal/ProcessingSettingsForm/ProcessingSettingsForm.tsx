import { type FC, useMemo, useState } from 'react';
import { Empty, Form, Input, InputNumber, Select } from 'antd';

import styles from './ProcessingSettingsForm.module.scss';

import type { ProviderConfig } from '#/models/Provider';

interface ProcessingSettingsFormProps {
  disabled?: boolean;
  providers: ProviderConfig[];
}

const ProcessingSettingsForm: FC<ProcessingSettingsFormProps> = ({
  disabled = false,
  providers,
}) => {
  const [selectedProviderId, setSelectedProviderId] = useState(() => providers[0]?.id ?? '');
  const [selectedModel, setSelectedModel] = useState('');
  const [language, setLanguage] = useState('auto');
  const [prompt, setPrompt] = useState('');
  const [temperature, setTemperature] = useState(0.2);
  const [extraParameters, setExtraParameters] = useState('');

  const selectedProvider = useMemo(
    () => providers.find((provider) => provider.id === selectedProviderId) ?? providers[0],
    [providers, selectedProviderId],
  );
  const modelOptions = (selectedProvider?.favoriteModels ?? []).map((modelName) => ({
    label: modelName,
    value: modelName,
  }));
  const selectedProviderValue = selectedProvider?.id ?? '';
  const selectedModelValue = modelOptions.some((model) => model.value === selectedModel)
    ? selectedModel
    : (modelOptions[0]?.value ?? '');

  if (providers.length === 0) {
    return <Empty description="Сначала добавьте провайдера" image={Empty.PRESENTED_IMAGE_SIMPLE} />;
  }

  return (
    <Form disabled={disabled} layout="vertical">
      <Form.Item label="Провайдер">
        <Select
          value={selectedProviderValue}
          options={providers.map((provider) => ({
            label: provider.name,
            value: provider.id,
          }))}
          onChange={setSelectedProviderId}
        />
      </Form.Item>

      <Form.Item label="Модель">
        <Select
          notFoundContent="Добавьте избранные модели в настройках провайдера"
          value={selectedModelValue}
          options={modelOptions}
          onChange={setSelectedModel}
        />
      </Form.Item>

      <div className={styles.fieldGrid}>
        <Form.Item label="Язык">
          <Select
            value={language}
            options={[
              {
                label: 'Авто',
                value: 'auto',
              },
              {
                label: 'Русский',
                value: 'ru',
              },
              {
                label: 'English',
                value: 'en',
              },
            ]}
            onChange={setLanguage}
          />
        </Form.Item>

        <Form.Item label="Температура">
          <InputNumber
            className={styles.temperatureInput}
            max={2}
            min={0}
            step={0.1}
            value={temperature}
            onChange={(value) => {
              setTemperature(value ?? 0);
            }}
          />
        </Form.Item>
      </div>

      <Form.Item label="Промпт">
        <Input.TextArea
          className={styles.textArea}
          placeholder="Подсказка для модели, словарь терминов или контекст записи"
          value={prompt}
          onChange={(event) => {
            setPrompt(event.target.value);
          }}
        />
      </Form.Item>

      <Form.Item label="Прочее">
        <Input.TextArea
          className={styles.textArea}
          placeholder="Дополнительные параметры в свободном формате"
          value={extraParameters}
          onChange={(event) => {
            setExtraParameters(event.target.value);
          }}
        />
      </Form.Item>
    </Form>
  );
};

export default ProcessingSettingsForm;
