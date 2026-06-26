import type { OverlayVariant } from '#/models/Settings';

export type { OverlayVariant } from '#/models/Settings';

export type OverlayState = 'error' | 'processing' | 'recording' | 'transcribing' | 'warning';

export interface OverlayShowPayload {
  state: OverlayState;
  variant: OverlayVariant;
  recordId?: string | null;
}

export const stateLabels: Record<OverlayState, string> = {
  error: 'Error',
  processing: 'Processing',
  recording: 'Recording',
  transcribing: 'Transcribing',
  warning: 'Warning',
};

export const cancelLabel = 'Cancel';
export const openRecordLabel = 'Details';
export const closeLabel = 'Close';

/** States that render the error/warning notification (colored card + actions). */
export const isNoticeState = (state: OverlayState): boolean =>
  state === 'error' || state === 'warning';
