import { type FC, useEffect, useState } from 'react';
import { ConfigProvider, Menu, type MenuProps, message, Modal } from 'antd';
import {
  BookOpenIcon,
  KeyboardIcon,
  SettingsIcon,
  SlidersHorizontalIcon,
  WandSparklesIcon,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';

import { useProviders } from '#/app/providersContext';
import { useAppSettings } from '#/app/settingsContext';
import * as catalogApi from '#/shared/catalogApi';
import * as settingsApi from '#/shared/settingsApi';

import GeneralSettingsTab from './GeneralSettingsTab';
import HotkeysSettingsTab from './HotkeysSettingsTab';
import PostProcessingSettingsTab from './PostProcessingSettingsTab';
import ProviderSettingsModal from './ProviderSettingsModal';
import ProvidersSettingsTab from './ProvidersSettingsTab';
import SpeechToTextSettingsTab from './SpeechToTextSettingsTab';

import styles from './AppSettingsModal.module.scss';

import type { CuratedModelInfo } from '#/models/Catalog';
import type {
  ModelInfo,
  ProviderConfig,
  ProviderConnectionInput,
  ProviderInput,
} from '#/models/Provider';
import type { AppSettingsInput, SettingsSectionKey } from '#/models/Settings';

interface AppSettingsModalProps {
  open: boolean;
  onClose: () => void;
}

const getErrorMessage = (error: unknown) =>
  error instanceof Error ? error.message : String(error);

const AppSettingsModal: FC<AppSettingsModalProps> = ({ open, onClose }) => {
  const { t } = useTranslation();
  const [messageApi, messageContextHolder] = message.useMessage();
  const {
    createProvider,
    deleteProvider,
    listProviderModels,
    providers,
    updateProvider,
    validateProviderConfig,
  } = useProviders();
  const { settings, updateSettings } = useAppSettings();
  const [activeSection, setActiveSection] = useState<SettingsSectionKey>('general');
  const [isProviderModalOpen, setIsProviderModalOpen] = useState(false);
  const [editingProvider, setEditingProvider] = useState<ProviderConfig>();
  const [isModelListVisible, setIsModelListVisible] = useState(false);
  const [modelRows, setModelRows] = useState<ModelInfo[]>([]);
  const [isSavingProvider, setIsSavingProvider] = useState(false);
  const [isValidatingProvider, setIsValidatingProvider] = useState(false);
  const [isLoadingModels, setIsLoadingModels] = useState(false);
  const [catalog, setCatalog] = useState<CuratedModelInfo[]>([]);
  const isEditingProvider = editingProvider !== undefined;
  const settingsMenuItems: MenuProps['items'] = [
    {
      icon: <SettingsIcon size={18} strokeWidth={2} />,
      key: 'general',
      label: t('settings.sections.general'),
    },
    {
      icon: <KeyboardIcon size={18} strokeWidth={2} />,
      key: 'hotkeys',
      label: t('settings.sections.hotkeys'),
    },
    {
      icon: <BookOpenIcon size={18} strokeWidth={2} />,
      key: 'providers',
      label: t('settings.sections.providers'),
    },
    {
      icon: <SlidersHorizontalIcon size={18} strokeWidth={2} />,
      key: 'speechToText',
      label: t('settings.sections.speechToText'),
    },
    {
      icon: <WandSparklesIcon size={18} strokeWidth={2} />,
      key: 'postProcessing',
      label: t('settings.sections.postProcessing'),
    },
  ];

  useEffect(() => {
    catalogApi
      .getModelCatalog()
      .then(setCatalog)
      .catch(() => {
        // Catalog is static; ignore errors.
      });
  }, []);

  const handleOpenProviderModal = (provider?: ProviderConfig) => {
    setEditingProvider(provider);
    setModelRows([]);
    setIsModelListVisible(false);
    setIsProviderModalOpen(true);
  };

  const handleCloseProviderModal = () => {
    setIsProviderModalOpen(false);
  };

  const handleSettingsChange = async (input: AppSettingsInput) => {
    try {
      await updateSettings(input);
    } catch (error) {
      void messageApi.error(getErrorMessage(error));
    }
  };

  const handleOpenDebugLogsFolder = async () => {
    try {
      await settingsApi.openDebugLogsFolder();
    } catch (error) {
      void messageApi.error(getErrorMessage(error));
    }
  };

  const handleDeleteProvider = async (providerId: string) => {
    try {
      await deleteProvider(providerId);
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

      setModelRows(models);
      setIsModelListVisible(true);

      if (models.length === 0) {
        void messageApi.info(t('settings.providers.emptyModels'));
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
            isDebugLoggingEnabled={settings.isDebugLoggingEnabled}
            isLaunchAtLoginEnabled={settings.isLaunchAtLoginEnabled}
            isMuteWhileRecordingEnabled={settings.isMuteWhileRecordingEnabled}
            themePreference={settings.themePreference}
            uiLanguage={settings.uiLanguage}
            onDebugLogsFolderOpen={() => {
              void handleOpenDebugLogsFolder();
            }}
            onDebugLoggingEnabledChange={(isDebugLoggingEnabled) => {
              void handleSettingsChange({ isDebugLoggingEnabled });
            }}
            onLaunchAtLoginEnabledChange={(isLaunchAtLoginEnabled) => {
              void handleSettingsChange({ isLaunchAtLoginEnabled });
            }}
            onMuteWhileRecordingEnabledChange={(isMuteWhileRecordingEnabled) => {
              void handleSettingsChange({ isMuteWhileRecordingEnabled });
            }}
            onThemePreferenceChange={(themePreference) => {
              void handleSettingsChange({ themePreference });
            }}
            onUiLanguageChange={(uiLanguage) => {
              void handleSettingsChange({ uiLanguage });
            }}
          />
        );
      }

      case 'hotkeys': {
        return (
          <HotkeysSettingsTab
            hotkey={settings.hotkey}
            triggerMode={settings.triggerMode}
            onHotkeyChange={(hotkey) => {
              void handleSettingsChange({ hotkey });
            }}
            onTriggerModeChange={(triggerMode) => {
              void handleSettingsChange({ triggerMode });
            }}
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
        return <SpeechToTextSettingsTab />;
      }

      case 'postProcessing': {
        return <PostProcessingSettingsTab />;
      }
    }
  };

  return (
    <>
      {messageContextHolder}
      <Modal footer={null} open={open} title={t('settings.title')} width={920} onCancel={onClose}>
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
        catalog={catalog}
        editingProvider={editingProvider}
        isLoadingModels={isLoadingModels}
        isModelListVisible={isModelListVisible}
        isSaving={isSavingProvider}
        isValidating={isValidatingProvider}
        modelRows={modelRows}
        okText={isEditingProvider ? t('common.save') : t('common.add')}
        open={isProviderModalOpen}
        title={
          isEditingProvider
            ? t('settings.providers.modal.editTitle')
            : t('settings.providers.modal.addTitle')
        }
        onCancel={handleCloseProviderModal}
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
