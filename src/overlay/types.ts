import type { OverlayVariant } from '#/models/Settings';

export type { OverlayVariant } from '#/models/Settings';

export type OverlayState =
  | 'error'
  | 'paused'
  | 'processing'
  | 'recording'
  | 'transcribing'
  | 'vad'
  | 'warning';

export interface OverlayShowPayload {
  state: OverlayState;
  variant: OverlayVariant;
  recordId?: string | null;
}

export const stateLabels: Record<OverlayState, string> = {
  error: 'Error',
  paused: 'Paused',
  processing: 'Processing',
  recording: 'Recording',
  transcribing: 'Transcribing',
  vad: 'Detecting speech',
  warning: 'Warning',
};

export const cancelLabel = 'Cancel';
export const openRecordLabel = 'Details';
export const closeLabel = 'Close';

/** Состояния, при которых отображается уведомление об ошибке/предупреждении (цветная карточка + действия). */
export const isNoticeState = (state: OverlayState): boolean =>
  state === 'error' || state === 'warning';
