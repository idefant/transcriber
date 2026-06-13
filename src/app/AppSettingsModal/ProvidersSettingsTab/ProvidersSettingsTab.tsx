import { type FC } from 'react';
import { Button, Card, Tooltip } from 'antd';
import { PencilIcon, PlusIcon, Trash2Icon } from 'lucide-react';

import type { ProviderConfig } from '../types';

import styles from './ProvidersSettingsTab.module.scss';

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
}) => (
  <div className={styles.providersTab}>
    <div className={styles.providerToolbar}>
      <Button icon={<PlusIcon size={18} strokeWidth={2} />} type="primary" onClick={onAddProvider}>
        Добавить провайдера
      </Button>
    </div>

    <div className={styles.providerList}>
      {providers.map((provider) => (
        <Card className={styles.providerItemCard} key={provider.id} size="small">
          <div className={styles.providerItem}>
            <div className={styles.providerItemInfo}>
              <h3 className={styles.providerItemTitle}>{provider.name}</h3>
              <p className={styles.providerItemDescription}>
                Провайдер: {provider.provider}; ключ: {provider.keyPreview}
              </p>
            </div>
            <div className={styles.providerItemActions}>
              <Tooltip title="Редактировать провайдера">
                <Button
                  aria-label="Редактировать провайдера"
                  icon={<PencilIcon size={18} strokeWidth={2} />}
                  type="text"
                  onClick={() => {
                    onEditProvider(provider);
                  }}
                />
              </Tooltip>
              <Tooltip title="Удалить провайдера">
                <Button
                  aria-label="Удалить провайдера"
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

export default ProvidersSettingsTab;
