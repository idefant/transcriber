import { type FC, useCallback, useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import BottomOverlay from '../BottomOverlay';
import CenterOverlay from '../CenterOverlay';
import {
  isNoticeState,
  type OverlayShowPayload,
  type OverlayState,
  type OverlayVariant,
} from '../types';

const NOTICE_AUTO_HIDE_MS = 5000;

const RecordingOverlay: FC = () => {
  const [isVisible, setIsVisible] = useState(false);
  const [state, setState] = useState<OverlayState>('recording');
  const [variant, setVariant] = useState<OverlayVariant>('center');
  const [levels, setLevels] = useState<number[]>([0, 0, 0]);
  const [recordId, setRecordId] = useState<string | null>(null);

  useEffect(() => {
    // A window created for a secondary monitor may mount after the `show-overlay`
    // event was broadcast, so it recovers the current state on mount.
    void invoke<OverlayShowPayload | null>('get_overlay_state').then((payload) => {
      if (payload) {
        setState(payload.state);
        setVariant(payload.variant);
        setRecordId(payload.recordId ?? null);
        setIsVisible(true);
      }

      return null;
    });

    const unlisteners = [
      listen<OverlayShowPayload>('show-overlay', (event) => {
        setState(event.payload.state);
        setVariant(event.payload.variant);
        setRecordId(event.payload.recordId ?? null);
        setIsVisible(true);
      }),
      listen('hide-overlay', () => {
        setIsVisible(false);
      }),
      listen<number[]>('mic-level', (event) => {
        setLevels(event.payload.length > 0 ? event.payload.slice(0, 3) : [0, 0, 0]);
      }),
    ];

    return () => {
      void Promise.all(unlisteners).then((resolvedUnlisteners) => {
        for (const unlisten of resolvedUnlisteners) {
          unlisten();
        }

        return null;
      });
    };
  }, []);

  // Auto-hide the error/warning notification after a few seconds. The timer is
  // cleared when the state changes (e.g. a new dictation reuses the overlay).
  useEffect(() => {
    if (!isVisible || !isNoticeState(state)) {
      return;
    }

    const timer = setTimeout(() => {
      void invoke('dismiss_overlay');
    }, NOTICE_AUTO_HIDE_MS);

    return () => {
      clearTimeout(timer);
    };
  }, [isVisible, state, recordId]);

  const handleCancel = useCallback(() => {
    void invoke('cancel_dictation');
  }, []);

  const handleClose = useCallback(() => {
    void invoke('dismiss_overlay');
  }, []);

  const handleOpenRecord = useCallback(() => {
    if (recordId) {
      void invoke('open_history_record', { recordId });
    }
  }, [recordId]);

  const OverlayComponent = variant === 'center' ? CenterOverlay : BottomOverlay;

  return (
    <OverlayComponent
      isVisible={isVisible}
      levels={levels}
      recordId={recordId}
      state={state}
      onCancel={handleCancel}
      onClose={handleClose}
      onOpenRecord={handleOpenRecord}
    />
  );
};

export default RecordingOverlay;
