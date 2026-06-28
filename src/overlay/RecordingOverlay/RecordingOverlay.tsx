import { type FC, useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

import BottomOverlay from '../BottomOverlay';
import CenterOverlay from '../CenterOverlay';
import type { OverlayShowPayload, OverlayState, OverlayVariant } from '../types';

type ActivityLevels = [number, number, number];

const MIN_ACTIVITY_LEVEL = 0.05;
const MAX_ACTIVITY_LEVEL = 1;
const ACTIVITY_LEVEL_RANGE = MAX_ACTIVITY_LEVEL - MIN_ACTIVITY_LEVEL;
const BASELINE_ACTIVITY_LEVELS: ActivityLevels = [
  MIN_ACTIVITY_LEVEL,
  MIN_ACTIVITY_LEVEL,
  MIN_ACTIVITY_LEVEL,
];
const SILENCE_GATE = 0.003;

const clamp01 = (value: number): number => Math.max(0, Math.min(1, value));

const blendActivityLevels = (
  [left, center, right]: ActivityLevels,
  [nextLeft, nextCenter, nextRight]: ActivityLevels,
): ActivityLevels => [
  left * 0.45 + nextLeft * 0.55,
  center * 0.45 + nextCenter * 0.55,
  right * 0.45 + nextRight * 0.55,
];

const createRecordingActivityLevels = (micLevel: number): ActivityLevels => {
  if (micLevel <= SILENCE_GATE) {
    return BASELINE_ACTIVITY_LEVELS;
  }

  const normalized = clamp01((micLevel - SILENCE_GATE) / (1 - SILENCE_GATE));
  const strength = clamp01(Math.pow(normalized, 0.35) * 1.4);

  return [
    clamp01(MIN_ACTIVITY_LEVEL + ACTIVITY_LEVEL_RANGE * strength * 0.72),
    clamp01(MIN_ACTIVITY_LEVEL + ACTIVITY_LEVEL_RANGE * strength),
    clamp01(MIN_ACTIVITY_LEVEL + ACTIVITY_LEVEL_RANGE * strength * 0.84),
  ];
};

const RecordingOverlay: FC = () => {
  const [isVisible, setIsVisible] = useState(false);
  const [state, setState] = useState<OverlayState>('recording');
  const [variant, setVariant] = useState<OverlayVariant>('center');
  const [activityLevels, setActivityLevels] = useState<ActivityLevels>(BASELINE_ACTIVITY_LEVELS);
  const [recordId, setRecordId] = useState<string | null>(null);
  const isNoticeHoverTrackedRef = useRef(false);

  const applyOverlayState = useCallback(
    (nextState: OverlayState, nextVariant: OverlayVariant, nextRecordId: string | null) => {
      setState(nextState);
      setVariant(nextVariant);
      setRecordId(nextRecordId);
      setActivityLevels(BASELINE_ACTIVITY_LEVELS);
      setIsVisible(true);
    },
    [],
  );

  useEffect(() => {
    // A window created for a secondary monitor may mount after the `show-overlay`
    // event was broadcast, so it recovers the current state on mount.
    void invoke<OverlayShowPayload | null>('get_overlay_state').then((payload) => {
      if (payload) {
        isNoticeHoverTrackedRef.current = false;
        applyOverlayState(payload.state, payload.variant, payload.recordId ?? null);
      }

      return null;
    });

    const unlisteners = [
      listen<OverlayShowPayload>('show-overlay', (event) => {
        isNoticeHoverTrackedRef.current = false;
        applyOverlayState(
          event.payload.state,
          event.payload.variant,
          event.payload.recordId ?? null,
        );
      }),
      listen('hide-overlay', () => {
        isNoticeHoverTrackedRef.current = false;
        setActivityLevels(BASELINE_ACTIVITY_LEVELS);
        setIsVisible(false);
      }),
      listen<number>('mic-level', (event) => {
        setActivityLevels((current) =>
          blendActivityLevels(current, createRecordingActivityLevels(event.payload)),
        );
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
  }, [applyOverlayState]);

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
      levels={activityLevels}
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
