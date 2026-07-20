import { Component, type FC, type ReactNode } from 'react';
import { Button } from 'antd';
import { useTranslation } from 'react-i18next';

import { captureTelemetryException } from '#/shared/telemetry';

import styles from './ApplicationErrorBoundary.module.scss';

interface ApplicationErrorBoundaryProps {
  children: ReactNode;
  isTelemetryEnabled: boolean;
}

interface ErrorBoundaryContentProps extends ApplicationErrorBoundaryProps {
  fallback: (isErrorReportSent: boolean) => ReactNode;
}

interface ErrorBoundaryContentState {
  hasError: boolean;
  isErrorReportSent: boolean;
}

const reloadApplication = () => {
  globalThis.location.reload();
};

class ErrorBoundaryContent extends Component<ErrorBoundaryContentProps, ErrorBoundaryContentState> {
  public override state: ErrorBoundaryContentState = {
    hasError: false,
    isErrorReportSent: false,
  };

  public static getDerivedStateFromError(): ErrorBoundaryContentState {
    return { hasError: true, isErrorReportSent: false };
  }

  public override componentDidCatch(error: Error): void {
    const isErrorReportSent = this.props.isTelemetryEnabled && captureTelemetryException(error);

    if (isErrorReportSent) {
      this.setState({ isErrorReportSent: true });
    }
  }

  public override render(): ReactNode {
    if (this.state.hasError) {
      return this.props.fallback(this.state.isErrorReportSent);
    }

    return this.props.children;
  }
}

/** Показывает экран восстановления после неперехваченной ошибки основного React-интерфейса. */
const ApplicationErrorBoundary: FC<ApplicationErrorBoundaryProps> = ({
  children,
  isTelemetryEnabled,
}) => {
  const { t } = useTranslation();

  return (
    <ErrorBoundaryContent
      fallback={(isErrorReportSent) => (
        <main className={styles.screen}>
          <div className={styles.content}>
            <h1 className={styles.title}>{t('errorBoundary.title')}</h1>
            <p className={styles.description}>{t('errorBoundary.description')}</p>
            {isErrorReportSent && (
              <p className={styles.telemetry}>{t('errorBoundary.telemetrySent')}</p>
            )}
            <Button type="primary" onClick={reloadApplication}>
              {t('errorBoundary.reload')}
            </Button>
          </div>
        </main>
      )}
      isTelemetryEnabled={isTelemetryEnabled}
    >
      {children}
    </ErrorBoundaryContent>
  );
};

export default ApplicationErrorBoundary;
