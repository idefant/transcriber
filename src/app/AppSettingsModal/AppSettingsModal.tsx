import { type FC, useMemo, useState } from 'react';
import {
  Button,
  Form,
  Input,
  List,
  Menu,
  type MenuProps,
  Modal,
  Radio,
  Segmented,
  Select,
  Space,
  Switch,
  Table,
  type TableColumnsType,
  Typography,
} from 'antd';
import {
  BookOpenIcon,
  CheckCircleIcon,
  KeyboardIcon,
  PlusIcon,
  SettingsIcon,
  SlidersHorizontalIcon,
  SparklesIcon,
  StarIcon,
  WandSparklesIcon,
} from 'lucide-react';

import { useAppTheme } from '#/app/themeContext';

import styles from './AppSettingsModal.module.scss';

type SettingsSectionKey = 'general' | 'hotkeys' | 'providers' | 'speechToText' | 'postProcessing';
type ProviderKind = 'custom' | 'grok' | 'openai' | 'openrouter';
type TriggerMode = 'hold' | 'press';
type UiLanguage = 'en' | 'ru';

interface AppSettingsModalProps {
  open: boolean;
  onClose: () => void;
}

interface ProviderConfig {
  id: string;
  keyPreview: string;
  name: string;
  provider: Exclude<ProviderKind, 'custom'> | 'custom';
}

interface ProviderOption {
  label: string;
  value: ProviderKind;
}

interface ModelInfo {
  description: string;
  name: string;
}

const settingsMenuItems: MenuProps['items'] = [
  {
    icon: <SettingsIcon size={18} strokeWidth={2} />,
    key: 'general',
    label: 'Основное',
  },
  {
    icon: <KeyboardIcon size={18} strokeWidth={2} />,
    key: 'hotkeys',
    label: 'Хоткеи',
  },
  {
    icon: <BookOpenIcon size={18} strokeWidth={2} />,
    key: 'providers',
    label: 'Провайдеры',
  },
  {
    icon: <SlidersHorizontalIcon size={18} strokeWidth={2} />,
    key: 'speechToText',
    label: 'Speech-to-Text',
  },
  {
    icon: <WandSparklesIcon size={18} strokeWidth={2} />,
    key: 'postProcessing',
    label: 'Постобработка',
  },
];

const providerOptions: ProviderOption[] = [
  {
    label: 'OpenAI',
    value: 'openai',
  },
  {
    label: 'Grok',
    value: 'grok',
  },
  {
    label: 'OpenRouter',
    value: 'openrouter',
  },
  {
    label: 'Custom (совместимый с OpenAI)',
    value: 'custom',
  },
];

const providerModels: Record<ProviderKind, ModelInfo[]> = {
  custom: [
    {
      description: 'Модель будет загружена из пользовательского OpenAI-compatible endpoint.',
      name: 'custom-model',
    },
  ],
  grok: [
    {
      description: 'Быстрая модель для черновой транскрибации и коротких аудио.',
      name: 'grok-stt-beta',
    },
    {
      description: 'Модель для более точного распознавания речи в длинных записях.',
      name: 'grok-stt-large',
    },
  ],
  openai: [
    {
      description: 'Универсальная модель распознавания речи.',
      name: 'gpt-4o-transcribe',
    },
    {
      description: 'Лёгкая модель для быстрых транскрибаций.',
      name: 'gpt-4o-mini-transcribe',
    },
  ],
  openrouter: [
    {
      description: 'Маршрутизируемая модель распознавания речи через OpenRouter.',
      name: 'openrouter/auto-stt',
    },
    {
      description: 'Резервная модель для аудио с шумом.',
      name: 'openrouter/stt-balanced',
    },
  ],
};

const defaultProviders: ProviderConfig[] = [
  {
    id: 'openai-default',
    keyPreview: 'sk-...42f9',
    name: 'OpenAI Gateway',
    provider: 'openai',
  },
];

const AppSettingsModal: FC<AppSettingsModalProps> = ({ open, onClose }) => {
  const [activeSection, setActiveSection] = useState<SettingsSectionKey>('general');
  const [uiLanguage, setUiLanguage] = useState<UiLanguage>('ru');
  const [hotkey, setHotkey] = useState('Ctrl + Shift + Space');
  const [triggerMode, setTriggerMode] = useState<TriggerMode>('press');
  const [providers, setProviders] = useState<ProviderConfig[]>(defaultProviders);
  const [isProviderModalOpen, setIsProviderModalOpen] = useState(false);
  const [selectedProvider, setSelectedProvider] = useState<ProviderKind>('openai');
  const [areAdvancedSettingsEnabled, setAreAdvancedSettingsEnabled] = useState(false);
  const [isModelListVisible, setIsModelListVisible] = useState(false);
  const [favoriteModels, setFavoriteModels] = useState<Set<string>>(() => new Set());
  const { isDarkMode, setIsDarkMode } = useAppTheme();

  const selectedProviderLabel = providerOptions.find(
    ({ value }) => value === selectedProvider,
  )?.label;
  const canUseAdvancedSettings = selectedProvider !== 'custom';
  const modelRows = providerModels[selectedProvider];

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
                setFavoriteModels((currentFavorites) => {
                  const nextFavorites = new Set(currentFavorites);

                  if (nextFavorites.has(model.name)) {
                    nextFavorites.delete(model.name);
                    return nextFavorites;
                  }

                  nextFavorites.add(model.name);
                  return nextFavorites;
                });
              }}
            />
          );
        },
        title: 'Избранное',
        width: 120,
      },
    ],
    [favoriteModels],
  );

  const handleAddProvider = () => {
    const providerLabel = selectedProviderLabel ?? 'Custom';

    setProviders((currentProviders) => [
      ...currentProviders,
      {
        id: `${selectedProvider}-${Date.now()}`,
        keyPreview: '••••••••',
        name: providerLabel,
        provider: selectedProvider,
      },
    ]);
    setIsProviderModalOpen(false);
  };

  const renderActiveSection = () => {
    switch (activeSection) {
      case 'general': {
        return (
          <div className={styles.section}>
            <Typography.Title level={4}>Основное</Typography.Title>
            <Form layout="vertical">
              <Form.Item label="Тема">
                <Segmented
                  options={[
                    {
                      label: 'Светлая',
                      value: 'light',
                    },
                    {
                      label: 'Темная',
                      value: 'dark',
                    },
                  ]}
                  value={isDarkMode ? 'dark' : 'light'}
                  onChange={(value) => {
                    setIsDarkMode(value === 'dark');
                  }}
                />
              </Form.Item>
              <Form.Item label="Язык UI">
                <Select
                  value={uiLanguage}
                  options={[
                    {
                      label: 'Русский',
                      value: 'ru',
                    },
                    {
                      label: 'English',
                      value: 'en',
                    },
                  ]}
                  onChange={setUiLanguage}
                />
              </Form.Item>
            </Form>
          </div>
        );
      }

      case 'hotkeys': {
        return (
          <div className={styles.section}>
            <Typography.Title level={4}>Хоткеи</Typography.Title>
            <Form layout="vertical">
              <Form.Item label="Хоткей для старта записи">
                <Input
                  value={hotkey}
                  onChange={(event) => {
                    setHotkey(event.target.value);
                  }}
                />
              </Form.Item>
              <Form.Item label="Режим запуска">
                <Radio.Group
                  value={triggerMode}
                  onChange={(event) => {
                    setTriggerMode(event.target.value as TriggerMode);
                  }}
                >
                  <Radio.Button value="press">По нажатию</Radio.Button>
                  <Radio.Button value="hold">По зажатию комбинации</Radio.Button>
                </Radio.Group>
              </Form.Item>
            </Form>
          </div>
        );
      }

      case 'providers': {
        return (
          <div className={styles.section}>
            <Typography.Title level={4}>Провайдеры</Typography.Title>
            <Typography.Paragraph>
              Здесь будут общие настройки подключенных провайдеров распознавания и обработки.
            </Typography.Paragraph>
          </div>
        );
      }

      case 'speechToText': {
        return (
          <div className={styles.section}>
            <Typography.Title level={4}>Speech-to-Text</Typography.Title>
            <List
              bordered
              dataSource={providers}
              renderItem={(provider) => (
                <List.Item>
                  <List.Item.Meta
                    title={provider.name}
                    description={`Провайдер: ${provider.provider}; ключ: ${provider.keyPreview}`}
                  />
                </List.Item>
              )}
            />
            <div className={styles.providerActions}>
              <Button
                icon={<PlusIcon size={18} strokeWidth={2} />}
                type="primary"
                onClick={() => {
                  setIsProviderModalOpen(true);
                }}
              >
                Добавить провайдера
              </Button>
            </div>
          </div>
        );
      }

      case 'postProcessing': {
        return (
          <div className={styles.section}>
            <Typography.Title level={4}>Постобработка</Typography.Title>
            <Typography.Paragraph>
              Настройки постобработки появятся здесь после подключения первого сценария.
            </Typography.Paragraph>
          </div>
        );
      }
    }
  };

  return (
    <>
      <Modal footer={null} open={open} title="Настройки" width={920} onCancel={onClose}>
        <div className={styles.modalBody}>
          <Menu
            className={styles.settingsMenu}
            items={settingsMenuItems}
            mode="inline"
            selectedKeys={[activeSection]}
            onClick={({ key }) => {
              setActiveSection(key as SettingsSectionKey);
            }}
          />
          <div className={styles.panel}>{renderActiveSection()}</div>
        </div>
      </Modal>

      <Modal
        okText="Добавить"
        open={isProviderModalOpen}
        title="Добавить провайдера"
        width={760}
        onCancel={() => {
          setIsProviderModalOpen(false);
        }}
        onOk={handleAddProvider}
      >
        <div className={styles.providerCard}>
          <Form layout="vertical">
            <Form.Item label="Провайдер">
              <Select
                value={selectedProvider}
                options={providerOptions}
                onChange={(value) => {
                  setSelectedProvider(value);
                  setAreAdvancedSettingsEnabled(false);
                  setIsModelListVisible(false);
                }}
              />
            </Form.Item>

            {selectedProvider === 'custom' ? (
              <div className={styles.fieldGrid}>
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
              </div>
            ) : (
              <div className={styles.fieldGrid}>
                <Form.Item label="Ключ">
                  <Input.Password placeholder="Введите API key" />
                </Form.Item>
                <Form.Item label="Дополнительные параметры">
                  <Switch
                    checked={areAdvancedSettingsEnabled}
                    disabled={!canUseAdvancedSettings}
                    onChange={setAreAdvancedSettingsEnabled}
                  />
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
              </div>
            )}
          </Form>

          <Space className={styles.modelActions}>
            <Button icon={<CheckCircleIcon size={18} strokeWidth={2} />}>
              Проверить валидность конфигурации
            </Button>
            <Button
              icon={<SparklesIcon size={18} strokeWidth={2} />}
              onClick={() => {
                setIsModelListVisible((isVisible) => !isVisible);
              }}
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
    </>
  );
};

export default AppSettingsModal;
