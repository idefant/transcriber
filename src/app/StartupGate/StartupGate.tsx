import { type FC, type ReactNode, useEffect, useState } from 'react';

import DataTooNewScreen from '#/app/DataTooNewScreen';
import { getStartupStatus } from '#/shared/maintenanceApi';

import { useSettingsStore } from '#/stores';

interface StartupGateProps {
  children: ReactNode;
}

type GateStatus = 'loading' | 'ready' | 'tooNew';

/**
 * Проверяет статус запуска до монтирования приложения. Если данные записаны
 * более новой версией приложения, показывает блокирующий экран вместо обычного
 * интерфейса, чтобы старый код не читал и не портил несовместимые данные.
 */
const StartupGate: FC<StartupGateProps> = ({ children }) => {
  const [status, setStatus] = useState<GateStatus>('loading');

  useEffect(() => {
    void getStartupStatus()
      .then((result) => {
        setStatus(result.dataTooNew ? 'tooNew' : 'ready');
        return null;
      })
      .catch(() => {
        // Если статус не удалось получить, не блокируем приложение.
        setStatus('ready');
      });
  }, []);

  // В режиме «данные новее кода» обычные сторы не монтируются, поэтому
  // подгружаем настройки отдельно: только ради языка и темы блокирующего
  // экрана. Чтение settings.json безопасно (только чтение, неизвестные поля
  // игнорируются) и не трогает потенциально несовместимые данные истории.
  useEffect(() => {
    if (status === 'tooNew') {
      void useSettingsStore.getState().load();
    }
  }, [status]);

  if (status === 'loading') {
    return null;
  }

  if (status === 'tooNew') {
    return <DataTooNewScreen />;
  }

  return <>{children}</>;
};

export default StartupGate;
