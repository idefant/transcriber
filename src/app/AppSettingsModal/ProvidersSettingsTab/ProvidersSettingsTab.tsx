import { type FC } from 'react';
import { Button, Card, Tooltip } from 'antd';
import { PencilIcon, PlusIcon, Trash2Icon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import styles from './ProvidersSettingsTab.module.scss';

import type { ProviderConfig } from '#/models/Provider';

interface ProvidersSettingsTabProps {
  onAddProvider: () => void;
  onDeleteProvider: (providerId: string) => void;
  onEditProvider: (provider: ProviderConfig) => void;
  providers: ProviderConfig[];
}

const ProvidersSettingsTab: FC<ProvidersSettingsTabProps> = ({
  onAddProvider,
  onDeleteProvider,
  onEditProvider,
  providers,
}) => {
  const { t } = useTranslation();

  return (
    <div className={styles.providersTab}>
      <div className={styles.providerToolbar}>
        <Button
          icon={<PlusIcon size={18} strokeWidth={2} />}
          type="primary"
          onClick={onAddProvider}
        >
          {t('settings.providers.add')}
        </Button>
      </div>

      <div className={styles.providerList}>
        {providers.map((provider) => (
          <Card className={styles.providerItemCard} key={provider.id} size="small">
            <div className={styles.providerItem}>
              <div className={styles.providerItemInfo}>
                <h3 className={styles.providerItemTitle}>{provider.name}</h3>
                <p className={styles.providerItemDescription}>
                  {t('settings.providers.providerWithKey', {
                    key: provider.keyPreview,
                    provider: provider.provider,
                  })}
                </p>
              </div>
              <div className={styles.providerItemActions}>
                <Tooltip title={t('settings.providers.edit')}>
                  <Button
                    aria-label={t('settings.providers.edit')}
                    icon={<PencilIcon size={18} strokeWidth={2} />}
                    type="text"
                    onClick={() => {
                      onEditProvider(provider);
                    }}
                  />
                </Tooltip>
                <Tooltip title={t('settings.providers.delete')}>
                  <Button
                    aria-label={t('settings.providers.delete')}
                    danger
                    icon={<Trash2Icon size={18} strokeWidth={2} />}
                    type="text"
                    onClick={() => {
                      onDeleteProvider(provider.id);
                    }}
                  />
                </Tooltip>
              </div>
            </div>
          </Card>
        ))}
      </div>
    </div>
  );
};

export default ProvidersSettingsTab;
