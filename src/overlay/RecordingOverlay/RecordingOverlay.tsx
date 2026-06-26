import { type FC, useCallback, useEffect, useState } from 'react';
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

  useEffect(() => {
    // A window created for a secondary monitor may mount after the `show-overlay`
    // event was broadcast, so it recovers the current state on mount.
    void invoke<OverlayShowPayload | null>('get_overlay_state').then((payload) => {
      if (payload) {
        setState(payload.state);
        setVariant(payload.variant);
        setIsVisible(true);
      }

      return null;
    });

    const unlisteners = [
      listen<OverlayShowPayload>('show-overlay', (event) => {
        setState(event.payload.state);
        setVariant(event.payload.variant);
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

  const handleCancel = useCallback(() => {
    void invoke('cancel_dictation');
  }, []);

  const OverlayComponent = variant === 'center' ? CenterOverlay : BottomOverlay;

  return (
    <OverlayComponent isVisible={isVisible} levels={levels} state={state} onCancel={handleCancel} />
  );
};

export default RecordingOverlay;
