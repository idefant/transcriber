import { type FC, useState } from 'react';
import { Button, message, Modal } from 'antd';
import { Trash2Icon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import { resetAppData } from '#/shared/maintenanceApi';

const ResetAppDataButton: FC = () => {
  const { t } = useTranslation();
  const [messageApi, messageContextHolder] = message.useMessage();
  const [isConfirmOpen, setIsConfirmOpen] = useState(false);
  const [isResetting, setIsResetting] = useState(false);

  const handleReset = async () => {
    setIsResetting(true);

    try {
      await resetAppData();
      // Приложение перезапускается до разрешения промиса — сюда обычно не доходим.
    } catch (error) {
      setIsResetting(false);
      setIsConfirmOpen(false);
      void messageApi.error(error instanceof Error ? error.message : t('maintenance.reset.error'));
    }
  };

  return (
    <>
      {messageContextHolder}
      <Button
        danger
        icon={<Trash2Icon size={16} strokeWidth={2} />}
        onClick={() => {
          setIsConfirmOpen(true);
        }}
      >
        {t('maintenance.reset.button')}
      </Button>
      <Modal
        open={isConfirmOpen}
        title={t('maintenance.reset.confirmTitle')}
        okText={t('maintenance.reset.confirmOk')}
        cancelText={t('maintenance.reset.cancel')}
        okButtonProps={{ danger: true, loading: isResetting }}
        cancelButtonProps={{ disabled: isResetting }}
        closable={!isResetting}
        onOk={() => void handleReset()}
        onCancel={() => {
          setIsConfirmOpen(false);
        }}
      >
        {t('maintenance.reset.confirmContent')}
      </Modal>
    </>
  );
};

export default ResetAppDataButton;
