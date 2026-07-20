import * as Sentry from '@sentry/react';

const isTelemetryAvailable = !import.meta.env.DEV && Boolean(import.meta.env.VITE_SENTRY_DSN);
let isTelemetryInitialized = false;
const capturedErrors = new WeakSet<Error>();

const isValidDsn = (dsn: string): boolean => {
  try {
    const url = new URL(dsn);
    return (url.protocol === 'http:' || url.protocol === 'https:') && Boolean(url.hostname);
  } catch {
    return false;
  }
};

const sanitizeEvent = (event: Sentry.ErrorEvent): Sentry.ErrorEvent => {
  event.breadcrumbs = [];
  event.contexts = {};
  event.extra = {};
  event.fingerprint = undefined;
  event.message = undefined;
  event.request = undefined;
  event.tags = {};
  event.transaction = undefined;
  event.user = undefined;

  for (const exception of event.exception?.values ?? []) {
    exception.value = 'Unhandled application error';
  }

  return event;
};

/** Включает или выключает передачу строго обезличенных ошибок в Sentry. */
export const configureTelemetry = (isEnabled: boolean): void => {
  if (!isTelemetryAvailable || !isEnabled) {
    if (isTelemetryInitialized) {
      void Sentry.close();
      isTelemetryInitialized = false;
    }

    return;
  }

  if (isTelemetryInitialized) {
    return;
  }

  const dsn = import.meta.env.VITE_SENTRY_DSN;

  if (!dsn || !isValidDsn(dsn)) {
    return;
  }

  try {
    Sentry.init({
      beforeSend: sanitizeEvent,
      dsn,
      enableLogs: false,
      enableMetrics: false,
      environment: import.meta.env.VITE_APP_CHANNEL === 'canary' ? 'canary' : 'production',
      integrations: [Sentry.globalHandlersIntegration(), Sentry.dedupeIntegration()],
      release: `transcriber@${__APP_VERSION__}`,
      sendClientReports: false,
      sendDefaultPii: false,
      tracesSampleRate: 0,
    });
    isTelemetryInitialized = true;
  } catch {
    // Некорректный DSN не должен влиять на запуск приложения.
  }
};
/** Передаёт пойманную React Error Boundary ошибку в уже настроенный Sentry-клиент. */
export const captureTelemetryException = (error: Error): boolean => {
  if (!isTelemetryInitialized || capturedErrors.has(error)) {
    return false;
  }

  try {
    const eventId = Sentry.captureException(error);
    capturedErrors.add(error);
    return Boolean(eventId);
  } catch {
    return false;
  }
};
