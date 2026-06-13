import { type FC, useState } from 'react';
import { Menu, type MenuProps, Modal } from 'antd';
import {
  BookOpenIcon,
  KeyboardIcon,
  SettingsIcon,
  SlidersHorizontalIcon,
  WandSparklesIcon,
} from 'lucide-react';

import { defaultProviders, providerModels, providerOptions } from './constants';
import GeneralSettingsTab from './GeneralSettingsTab';
import HotkeysSettingsTab from './HotkeysSettingsTab';
import PostProcessingSettingsTab from './PostProcessingSettingsTab';
import ProviderSettingsModal from './ProviderSettingsModal';
import ProvidersSettingsTab from './ProvidersSettingsTab';
import SpeechToTextSettingsTab from './SpeechToTextSettingsTab';
import type {
  ProviderConfig,
  ProviderKind,
  SettingsSectionKey,
  TriggerMode,
  UiLanguage,
} from './types';

import styles from './AppSettingsModal.module.scss';

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

const AppSettingsModal: FC<AppSettingsModalProps> = ({ open, onClose }) => {
  const [activeSection, setActiveSection] = useState<SettingsSectionKey>('general');
  const [uiLanguage, setUiLanguage] = useState<UiLanguage>('ru');
  const [areDictationSoundsEnabled, setAreDictationSoundsEnabled] = useState(true);
  const [hotkey, setHotkey] = useState('Ctrl + Shift + Space');
  const [triggerMode, setTriggerMode] = useState<TriggerMode>('press');
  const [providers, setProviders] = useState<ProviderConfig[]>(defaultProviders);
  const [isProviderModalOpen, setIsProviderModalOpen] = useState(false);
  const [editingProviderId, setEditingProviderId] = useState<string>();
  const [selectedProvider, setSelectedProvider] = useState<ProviderKind>('openai');
  const [areAdvancedSettingsEnabled, setAreAdvancedSettingsEnabled] = useState(false);
  const [isModelListVisible, setIsModelListVisible] = useState(false);
  const [favoriteModels, setFavoriteModels] = useState<Set<string>>(() => new Set());

  const selectedProviderLabel = providerOptions.find(
    ({ value }) => value === selectedProvider,
  )?.label;
  const canUseAdvancedSettings = selectedProvider !== 'custom';
  const isEditingProvider = editingProviderId !== undefined;

  const handleOpenProviderModal = (provider?: ProviderConfig) => {
    setEditingProviderId(provider?.id);
    setSelectedProvider(provider?.provider ?? 'openai');
    setAreAdvancedSettingsEnabled(false);
    setIsModelListVisible(false);
    setIsProviderModalOpen(true);
  };

  const handleCloseProviderModal = () => {
    setIsProviderModalOpen(false);
  };

  const handleProviderChange = (provider: ProviderKind) => {
    setSelectedProvider(provider);
    setAreAdvancedSettingsEnabled(false);
    setIsModelListVisible(false);
  };

  const handleDeleteProvider = (providerId: string) => {
    setProviders((currentProviders) =>
      currentProviders.filter((provider) => provider.id !== providerId),
    );
  };

  const handleFavoriteModelToggle = (modelName: string) => {
    setFavoriteModels((currentFavorites) => {
      const nextFavorites = new Set(currentFavorites);

      if (nextFavorites.has(modelName)) {
        nextFavorites.delete(modelName);
        return nextFavorites;
      }

      nextFavorites.add(modelName);
      return nextFavorites;
    });
  };

  const handleSaveProvider = () => {
    const providerLabel = selectedProviderLabel ?? 'Custom';

    if (isEditingProvider) {
      setProviders((currentProviders) =>
        currentProviders.map((provider) =>
          provider.id === editingProviderId
            ? {
                ...provider,
                name: providerLabel,
                provider: selectedProvider,
              }
            : provider,
        ),
      );
      setIsProviderModalOpen(false);
      return;
    }

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
            onDeleteProvider={handleDeleteProvider}
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

      <ProviderSettingsModal
        areAdvancedSettingsEnabled={areAdvancedSettingsEnabled}
        canUseAdvancedSettings={canUseAdvancedSettings}
        favoriteModels={favoriteModels}
        isModelListVisible={isModelListVisible}
        modelRows={providerModels[selectedProvider]}
        okText={isEditingProvider ? 'Сохранить' : 'Добавить'}
        open={isProviderModalOpen}
        selectedProvider={selectedProvider}
        title={isEditingProvider ? 'Редактировать провайдера' : 'Добавить провайдера'}
        onAdvancedSettingsEnabledChange={setAreAdvancedSettingsEnabled}
        onCancel={handleCloseProviderModal}
        onFavoriteModelToggle={handleFavoriteModelToggle}
        onModelListVisibleToggle={() => {
          setIsModelListVisible((isVisible) => !isVisible);
        }}
        onProviderChange={handleProviderChange}
        onSubmit={handleSaveProvider}
      />
    </>
  );
};

export default AppSettingsModal;
