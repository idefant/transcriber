import {
  defaultStackParser,
  type ErrorEvent,
  type EventHint,
  type StackFrame,
} from '@sentry/react';

const errorTypes = new Set([
  'Error',
  'TypeError',
  'RangeError',
  'ReferenceError',
  'SyntaxError',
  'URIError',
  'EvalError',
  'AggregateError',
  'UnhandledRejection',
]);
const mechanisms = new Set([
  'generic',
  'auto.browser.global_handlers.onerror',
  'auto.browser.global_handlers.onunhandledrejection',
]);

// Разрешены только имена JS-ресурсов сборки, без адреса WebView, query и fragment.
const sanitizeFilename = (filename: string | undefined): string | undefined => {
  const match = filename?.match(
    /^(?:(?:tauri:\/\/localhost|https?:\/\/tauri\.localhost|app:\/\/)?\/)?(assets\/[\w-]+\.js)(?:[?#].*)?$/,
  );
  return match ? `app:///${match[1]}` : undefined;
};

const sanitizeFrames = (frames: StackFrame[] | undefined): StackFrame[] =>
  (frames ?? []).flatMap((frame) => {
    const filename = sanitizeFilename(frame.filename);
    return filename ? [{ filename, lineno: frame.lineno, colno: frame.colno, in_app: true }] : [];
  });

const classifyError = (type: string, message: unknown = ''): string => {
  if (typeof message !== 'string') return 'unclassified';
  const reactCode = /^Minified React error #(\d{1,3});/.exec(message);
  if (reactCode) return `react_error_${reactCode[1]}`;
  if (type === 'TypeError' && /^(?:Cannot read|Cannot set) propert/.test(message))
    return 'invalid_property_access';
  if (type === 'TypeError' && message.endsWith(' is not a function')) return 'not_callable';
  if (type === 'ReferenceError' && message.endsWith(' is not defined'))
    return 'undefined_reference';
  if (type === 'RangeError' && message === 'Maximum call stack size exceeded')
    return 'stack_overflow';
  if (
    ['Failed to fetch', 'Load failed', 'NetworkError when attempting to fetch resource.'].includes(
      message,
    )
  )
    return 'network_failure';
  return 'unclassified';
};

const buildSanitizedTelemetryEvent = (event: ErrorEvent, hint: EventHint): ErrorEvent => {
  hint.attachments = [];
  const isRejection = event.exception?.values?.some(
    (exception) =>
      exception.mechanism?.type === 'auto.browser.global_handlers.onunhandledrejection',
  );
  const isNonErrorRejection = isRejection && !(hint.originalException instanceof Error);
  const values = (event.exception?.values ?? []).map((exception) => {
    const originalType = exception.type ?? 'Error';
    const type = errorTypes.has(originalType) ? originalType : 'Error';
    const category = classifyError(type, exception.value);
    const mechanism = exception.mechanism;
    const frames = isNonErrorRejection ? [] : sanitizeFrames(exception.stacktrace?.frames);
    return {
      type,
      value: category,
      ...(frames.length > 0 ? { stacktrace: { frames } } : {}),
      ...(mechanism && mechanisms.has(mechanism.type)
        ? { mechanism: { type: mechanism.type, handled: mechanism.handled } }
        : {}),
    };
  });
  const hasOriginalStack = values.some((exception) => exception.stacktrace);
  // Это стек обработчика, а не место создания отклонённого Promise.
  const observationFrames = hasOriginalStack
    ? []
    : sanitizeFrames(defaultStackParser(new Error('Telemetry observation').stack ?? ''));

  const images = event.debug_meta?.images?.flatMap((image) => {
    const codeFile = 'code_file' in image ? sanitizeFilename(image.code_file) : undefined;
    return image.type === 'sourcemap' && codeFile && /^[\da-f-]{36}$/i.test(image.debug_id)
      ? [{ type: 'sourcemap' as const, code_file: codeFile, debug_id: image.debug_id }]
      : [];
  });

  return {
    type: undefined,
    event_id: event.event_id,
    timestamp: event.timestamp,
    level: event.level,
    platform: 'javascript',
    release: `transcriber@${__APP_VERSION__}`,
    environment: import.meta.env.VITE_APP_CHANNEL === 'canary' ? 'canary' : 'production',
    // Нулевой адрес запрещает серверу выводить IP и географию из транспортного запроса.
    user: { ip_address: '0.0.0.0' },
    exception: { values },
    tags: {
      'diagnostic.schema': '2',
      'diagnostic.origin': isRejection ? 'unhandled_rejection' : 'exception',
      'diagnostic.reason_type': isRejection ? typeof hint.originalException : 'error',
      'diagnostic.stack': hasOriginalStack ? 'original' : 'unavailable',
    },
    ...(observationFrames.length > 0
      ? {
          contexts: {
            diagnostic_observation: { frames: observationFrames, origin: 'before_send' },
          },
        }
      : {}),
    ...(images?.length ? { debug_meta: { images } } : {}),
  };
};

/** Собирает отчёт по разрешённым полям; при сбое очистки отбрасывает событие без повторной отправки ошибки. */
export const sanitizeTelemetryEvent = (event: ErrorEvent, hint: EventHint): ErrorEvent | null => {
  try {
    return buildSanitizedTelemetryEvent(event, hint);
  } catch {
    // Ошибка beforeSend иначе может попасть в Sentry в обход очистки как внутренний сбой SDK.
    return null;
  }
};
