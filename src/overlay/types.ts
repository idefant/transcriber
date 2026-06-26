import type { OverlayVariant } from '#/models/Settings';

export type { OverlayVariant } from '#/models/Settings';

export type OverlayState = 'processing' | 'recording' | 'transcribing';

export interface OverlayShowPayload {
  state: OverlayState;
  variant: OverlayVariant;
}

export const stateLabels: Record<OverlayState, string> = {
  processing: 'Processing',
  recording: 'Recording',
  transcribing: 'Transcribing',
};

export const cancelLabel = 'Cancel';
