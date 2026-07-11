import { type FC, useCallback, useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { clamp } from 'lodash-es';

import { rotate } from '#/shared/utils';

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
const ACTIVITY_COEFFICIENTS: ActivityLevels = [0.52, 1, 0.74];
const ROTATION_STEP_MS = 400;
const MIC_LEVEL_SMOOTHING = 0.55;

const lerp = (from: number, to: number, ratio: number): number => from + (to - from) * ratio;

// Ease in and out so a coefficient settles into its new slot instead of sliding at a constant speed.
const smoothstep = (ratio: number): number => ratio * ratio * (3 - 2 * ratio);

// The coefficients travel across the levels: 1,2,3 -> 3,1,2 -> 2,3,1.
const rotateCoefficients = (step: number): ActivityLevels =>
  rotate(ACTIVITY_COEFFICIENTS, step) as ActivityLevels;

const createRotatedCoefficients = (elapsedMs: number): ActivityLevels => {
  const progress = elapsedMs / ROTATION_STEP_MS;
  const step = Math.floor(progress);
  const blend = smoothstep(progress - step);
  const [fromLeft, fromCenter, fromRight] = rotateCoefficients(step);
  const [toLeft, toCenter, toRight] = rotateCoefficients(step + 1);

  return [
    lerp(fromLeft, toLeft, blend),
    lerp(fromCenter, toCenter, blend),
    lerp(fromRight, toRight, blend),
  ];
};

const calculateMicStrength = (micLevel: number): number => {
  if (micLevel <= SILENCE_GATE) {
    return 0;
  }

  const normalized = clamp((micLevel - SILENCE_GATE) / (1 - SILENCE_GATE), 0, 1);

  return clamp(Math.pow(normalized, 0.35) * 1.4, 0, 1);
};

const createRecordingActivityLevels = (strength: number, elapsedMs: number): ActivityLevels => {
  const [left, center, right] = createRotatedCoefficients(elapsedMs);
  const toLevel = (coefficient: number): number =>
    clamp(MIN_ACTIVITY_LEVEL + ACTIVITY_LEVEL_RANGE * strength * coefficient, 0, 1);

  return [toLevel(left), toLevel(center), toLevel(right)];
};

const RecordingOverlay: FC = () => {
  const [isVisible, setIsVisible] = useState(false);
  const [state, setState] = useState<OverlayState>('recording');
  const [variant, setVariant] = useState<OverlayVariant>('center');
  const [activityLevels, setActivityLevels] = useState<ActivityLevels>(BASELINE_ACTIVITY_LEVELS);
  const [recordId, setRecordId] = useState<string | null>(null);
  const isNoticeHoverTrackedRef = useRef(false);
  const micStrengthRef = useRef(0);
  const rotationStartedAtRef = useRef(0);

  const resetActivityLevels = useCallback(() => {
    micStrengthRef.current = 0;
    rotationStartedAtRef.current = performance.now();
    setActivityLevels(BASELINE_ACTIVITY_LEVELS);
  }, []);

  const applyOverlayState = useCallback(
    (nextState: OverlayState, nextVariant: OverlayVariant, nextRecordId: string | null) => {
      setState(nextState);
      setVariant(nextVariant);
      setRecordId(nextRecordId);
      resetActivityLevels();
      setIsVisible(true);
    },
    [resetActivityLevels],
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
        resetActivityLevels();
        setIsVisible(false);
      }),
      listen<number>('mic-level', (event) => {
        micStrengthRef.current = lerp(
          micStrengthRef.current,
          calculateMicStrength(event.payload),
          MIC_LEVEL_SMOOTHING,
        );
        setActivityLevels(
          createRecordingActivityLevels(
            micStrengthRef.current,
            performance.now() - rotationStartedAtRef.current,
          ),
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
  }, [applyOverlayState, resetActivityLevels]);

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
