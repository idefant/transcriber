import { type FC } from 'react';

interface TelemetryReactCrashProps {
  isActive: boolean;
}

interface PendingImport {
  fileName: string;
}

interface ActiveImports {
  latestImport: PendingImport;
}

/** Временно воспроизводит ошибку рендера React для проверки error telemetry. */
const TelemetryReactCrash: FC<TelemetryReactCrashProps> = ({ isActive }) => {
  if (!isActive) {
    return null;
  }

  // Импорт уже завершился, но запись операции ещё остаётся обязательной для
  // отображения строки статуса. Такое предположение о жизненном цикле данных
  // приводит к типичной ошибке рендера после неудачного обновления состояния.
  const cachedImportState: unknown = JSON.parse('{}');
  const activeImports = cachedImportState as ActiveImports;
  const pendingImport = activeImports.latestImport;

  return <span>{pendingImport.fileName}</span>;
};

export default TelemetryReactCrash;
