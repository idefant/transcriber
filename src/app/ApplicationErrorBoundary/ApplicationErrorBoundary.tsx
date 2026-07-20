import { Component, type FC, type ReactNode } from 'react';
import { useRouteError } from 'react-router';
import { Button } from 'antd';
import { useTranslation } from 'react-i18next';

import { captureTelemetryException } from '#/shared/telemetry';

import styles from './ApplicationErrorBoundary.module.scss';

import { useSettingsStore } from '#/stores';

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

interface ErrorRecoveryScreenProps {
  isErrorReportSent: boolean;
}

interface RouteErrorBoundaryContentProps {
  isTelemetryEnabled: boolean;
  routeError: unknown;
}

interface RouteErrorBoundaryContentState {
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

const ErrorRecoveryScreen: FC<ErrorRecoveryScreenProps> = ({ isErrorReportSent }) => {
  const { t } = useTranslation();

  return (
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
  );
};

class RouteErrorBoundaryContent extends Component<
  RouteErrorBoundaryContentProps,
  RouteErrorBoundaryContentState
> {
  public override state: RouteErrorBoundaryContentState = {
    isErrorReportSent: false,
  };

  public override componentDidMount(): void {
    const { isTelemetryEnabled, routeError } = this.props;

    if (isTelemetryEnabled && routeError instanceof Error) {
      this.setState({ isErrorReportSent: captureTelemetryException(routeError) });
    }
  }

  public override render(): ReactNode {
    return <ErrorRecoveryScreen isErrorReportSent={this.state.isErrorReportSent} />;
  }
}

/** Показывает экран восстановления для ошибок, перехваченных React Router. */
export const RouteErrorBoundary: FC = () => {
  const routeError = useRouteError();
  const isTelemetryEnabled = useSettingsStore((s) => s.settings.isTelemetryEnabled);

  return (
    <RouteErrorBoundaryContent isTelemetryEnabled={isTelemetryEnabled} routeError={routeError} />
  );
};

/** Показывает экран восстановления после неперехваченной ошибки основного React-интерфейса. */
const ApplicationErrorBoundary: FC<ApplicationErrorBoundaryProps> = ({
  children,
  isTelemetryEnabled,
}) => {
  return (
    <ErrorBoundaryContent
      fallback={(isErrorReportSent) => (
        <ErrorRecoveryScreen isErrorReportSent={isErrorReportSent} />
      )}
      isTelemetryEnabled={isTelemetryEnabled}
    >
      {children}
    </ErrorBoundaryContent>
  );
};

export default ApplicationErrorBoundary;
