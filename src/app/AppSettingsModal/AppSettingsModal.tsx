import { type FC, useState } from 'react';
import { ConfigProvider, Menu, type MenuProps, message, Modal } from 'antd';
import {
  BookOpenIcon,
  KeyboardIcon,
  SettingsIcon,
  SlidersHorizontalIcon,
  WandSparklesIcon,
} from 'lucide-react';

import { useProviders } from '#/app/providersContext';

import GeneralSettingsTab from './GeneralSettingsTab';
import HotkeysSettingsTab from './HotkeysSettingsTab';
import PostProcessingSettingsTab from './PostProcessingSettingsTab';
import ProviderSettingsModal from './ProviderSettingsModal';
import ProvidersSettingsTab from './ProvidersSettingsTab';
import SpeechToTextSettingsTab from './SpeechToTextSettingsTab';

import styles from './AppSettingsModal.module.scss';

import type {
  ModelInfo,
  ProviderConfig,
  ProviderConnectionInput,
  ProviderInput,
} from '#/models/Provider';
import type { SettingsSectionKey, TriggerMode, UiLanguage } from '#/models/Settings';

interface AppSettingsModalProps {
  open: boolean;
  onClose: () => void;
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

const getErrorMessage = (error: unknown) =>
  error instanceof Error ? error.message : String(error);

const sortModelsByFavorite = (models: ModelInfo[], favoriteModels: Set<string>) =>
  models.toSorted((firstModel, secondModel) => {
    const isFirstFavorite = favoriteModels.has(firstModel.name);
    const isSecondFavorite = favoriteModels.has(secondModel.name);

    if (isFirstFavorite === isSecondFavorite) {
      return firstModel.name.localeCompare(secondModel.name);
    }

    return isFirstFavorite ? -1 : 1;
  });

const AppSettingsModal: FC<AppSettingsModalProps> = ({ open, onClose }) => {
  const [messageApi, messageContextHolder] = message.useMessage();
  const {
    createProvider,
    deleteProvider,
    listProviderModels,
    providers,
    toggleFavoriteModel,
    updateProvider,
    validateProviderConfig,
  } = useProviders();
  const [activeSection, setActiveSection] = useState<SettingsSectionKey>('general');
  const [uiLanguage, setUiLanguage] = useState<UiLanguage>('ru');
  const [areDictationSoundsEnabled, setAreDictationSoundsEnabled] = useState(true);
  const [hotkey, setHotkey] = useState('Ctrl + Shift + Space');
  const [triggerMode, setTriggerMode] = useState<TriggerMode>('press');
  const [isProviderModalOpen, setIsProviderModalOpen] = useState(false);
  const [editingProvider, setEditingProvider] = useState<ProviderConfig>();
  const [isModelListVisible, setIsModelListVisible] = useState(false);
  const [favoriteModels, setFavoriteModels] = useState<Set<string>>(() => new Set<string>());
  const [modelRows, setModelRows] = useState<ModelInfo[]>([]);
  const [isSavingProvider, setIsSavingProvider] = useState(false);
  const [isValidatingProvider, setIsValidatingProvider] = useState(false);
  const [isLoadingModels, setIsLoadingModels] = useState(false);
  const isEditingProvider = editingProvider !== undefined;

  const handleOpenProviderModal = (provider?: ProviderConfig) => {
    setEditingProvider(provider);
    setFavoriteModels(new Set(provider?.favoriteModels));
    setModelRows([]);
    setIsModelListVisible(false);
    setIsProviderModalOpen(true);
  };

  const handleCloseProviderModal = () => {
    setIsProviderModalOpen(false);
  };

  const handleDeleteProvider = async (providerId: string) => {
    try {
      await deleteProvider(providerId);
    } catch (error) {
      void messageApi.error(getErrorMessage(error));
    }
  };

  const handleFavoriteModelToggle = async (modelName: string) => {
    if (editingProvider === undefined) {
      setFavoriteModels((currentFavorites) => {
        const nextFavorites = new Set(currentFavorites);

        if (nextFavorites.has(modelName)) {
          nextFavorites.delete(modelName);
        } else {
          nextFavorites.add(modelName);
        }

        return nextFavorites;
      });
      return;
    }

    try {
      const provider = await toggleFavoriteModel(editingProvider.id, modelName);

      setEditingProvider(provider);
      setFavoriteModels(new Set(provider.favoriteModels));
    } catch (error) {
      void messageApi.error(getErrorMessage(error));
    }
  };

  const handleSaveProvider = async (input: ProviderInput) => {
    setIsSavingProvider(true);

    try {
      await (editingProvider === undefined
        ? createProvider(input)
        : updateProvider(editingProvider.id, input));

      setIsProviderModalOpen(false);
    } catch (error) {
      void messageApi.error(getErrorMessage(error));
    } finally {
      setIsSavingProvider(false);
    }
  };

  const handleValidateProvider = async (input: ProviderConnectionInput) => {
    setIsValidatingProvider(true);

    try {
      const result = await validateProviderConfig(input);

      if (result.ok) {
        void messageApi.success(result.message);
      } else {
        void messageApi.error(result.message);
      }
    } catch (error) {
      void messageApi.error(getErrorMessage(error));
    } finally {
      setIsValidatingProvider(false);
    }
  };

  const handleLoadModels = async (input: ProviderConnectionInput) => {
    setIsLoadingModels(true);

    try {
      const models = await listProviderModels(input);

      setModelRows(sortModelsByFavorite(models, favoriteModels));
      setIsModelListVisible(true);

      if (models.length === 0) {
        void messageApi.info('Провайдер не вернул доступные модели');
      }
    } catch (error) {
      void messageApi.error(getErrorMessage(error));
    } finally {
      setIsLoadingModels(false);
    }
  };

  const renderActiveSection = () => {
    switch (activeSection) {
      case 'general': {
        return (
          <GeneralSettingsTab
            areDictationSoundsEnabled={areDictationSoundsEnabled}
            uiLanguage={uiLanguage}
            onDictationSoundsEnabledChange={setAreDictationSoundsEnabled}
            onUiLanguageChange={setUiLanguage}
          />
        );
      }

      case 'hotkeys': {
        return (
          <HotkeysSettingsTab
            hotkey={hotkey}
            triggerMode={triggerMode}
            onHotkeyChange={setHotkey}
            onTriggerModeChange={setTriggerMode}
          />
        );
      }

      case 'providers': {
        return (
          <ProvidersSettingsTab
            providers={providers}
            onAddProvider={() => {
              handleOpenProviderModal();
            }}
            onDeleteProvider={(providerId) => {
              void handleDeleteProvider(providerId);
            }}
            onEditProvider={handleOpenProviderModal}
          />
        );
      }

      case 'speechToText': {
        return <SpeechToTextSettingsTab providers={providers} />;
      }

      case 'postProcessing': {
        return <PostProcessingSettingsTab providers={providers} />;
      }
    }
  };

  return (
    <>
      {messageContextHolder}
      <Modal footer={null} open={open} title="Настройки" width={920} onCancel={onClose}>
        <div className={styles.modalBody}>
          <ConfigProvider
            theme={{
              components: {
                Menu: {
                  itemMarginInline: 0,
                },
              },
            }}
          >
            <Menu
              className={styles.settingsMenu}
              items={settingsMenuItems}
              mode="inline"
              selectedKeys={[activeSection]}
              onClick={({ key }) => {
                setActiveSection(key as SettingsSectionKey);
              }}
            />
          </ConfigProvider>
          <div className={styles.panel}>{renderActiveSection()}</div>
        </div>
      </Modal>

      <ProviderSettingsModal
        editingProvider={editingProvider}
        favoriteModels={favoriteModels}
        isLoadingModels={isLoadingModels}
        isModelListVisible={isModelListVisible}
        isSaving={isSavingProvider}
        isValidating={isValidatingProvider}
        modelRows={modelRows}
        okText={isEditingProvider ? 'Сохранить' : 'Добавить'}
        open={isProviderModalOpen}
        title={isEditingProvider ? 'Редактировать провайдера' : 'Добавить провайдера'}
        onCancel={handleCloseProviderModal}
        onFavoriteModelToggle={(modelName) => {
          void handleFavoriteModelToggle(modelName);
        }}
        onLoadModels={handleLoadModels}
        onModelListHide={() => {
          setIsModelListVisible(false);
        }}
        onSubmit={handleSaveProvider}
        onValidate={handleValidateProvider}
      />
    </>
  );
};

export default AppSettingsModal;
