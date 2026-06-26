import { type FC, useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import BottomOverlay from '../BottomOverlay';
import CenterOverlay from '../CenterOverlay';
import type { OverlayShowPayload, OverlayState, OverlayVariant } from '../types';

const RecordingOverlay: FC = () => {
  const [isVisible, setIsVisible] = useState(false);
  const [state, setState] = useState<OverlayState>('recording');
  const [variant, setVariant] = useState<OverlayVariant>('center');
  const [levels, setLevels] = useState<number[]>([0, 0, 0]);
  const [recordId, setRecordId] = useState<string | null>(null);
  const isNoticeHoverTrackedRef = useRef(false);

  useEffect(() => {
    // A window created for a secondary monitor may mount after the `show-overlay`
    // event was broadcast, so it recovers the current state on mount.
    void invoke<OverlayShowPayload | null>('get_overlay_state').then((payload) => {
      if (payload) {
        isNoticeHoverTrackedRef.current = false;
        setState(payload.state);
        setVariant(payload.variant);
        setRecordId(payload.recordId ?? null);
        setIsVisible(true);
      }

      return null;
    });

    const unlisteners = [
      listen<OverlayShowPayload>('show-overlay', (event) => {
        isNoticeHoverTrackedRef.current = false;
        setState(event.payload.state);
        setVariant(event.payload.variant);
        setRecordId(event.payload.recordId ?? null);
        setIsVisible(true);
      }),
      listen('hide-overlay', () => {
        isNoticeHoverTrackedRef.current = false;
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

  useEffect(() => {
    isNoticeHoverTrackedRef.current = false;
  }, [isVisible, recordId, state]);

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

  const handleNoticeMouseMove = useCallback(() => {
    if (isNoticeHoverTrackedRef.current) {
      return;
    }

    isNoticeHoverTrackedRef.current = true;
    void invoke('overlay_notice_mouse_move');
  }, []);

  const handleNoticeMouseLeave = useCallback(() => {
    if (!isNoticeHoverTrackedRef.current) {
      return;
    }

    isNoticeHoverTrackedRef.current = false;
    void invoke('overlay_notice_mouse_leave');
  }, []);

  const OverlayComponent = variant === 'center' ? CenterOverlay : BottomOverlay;

  return (
    <OverlayComponent
      isVisible={isVisible}
      levels={levels}
      recordId={recordId}
      state={state}
      onCancel={handleCancel}
      onClose={handleClose}
      onNoticeMouseLeave={handleNoticeMouseLeave}
      onNoticeMouseMove={handleNoticeMouseMove}
      onOpenRecord={handleOpenRecord}
    />
  );
};

export default RecordingOverlay;
