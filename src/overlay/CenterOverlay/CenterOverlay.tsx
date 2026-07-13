import { type FC, useMemo } from 'react';
import {
  CircleAlertIcon,
  LoaderCircleIcon,
  MicIcon,
  PauseIcon,
  SparklesIcon,
  SquareArrowOutUpRightIcon,
  TriangleAlertIcon,
  XIcon,
} from 'lucide-react';

import type { OverlayState } from '../types';
import { cancelLabel, closeLabel, isNoticeState, openRecordLabel, stateLabels } from '../types';

import styles from './CenterOverlay.module.scss';

const MAX_LEVEL_HEIGHT = 20;

interface CenterOverlayProps {
  isVisible: boolean;
  levels: number[];
  onCancel: () => void;
  onClose: () => void;
  onNoticeMouseLeave?: () => void;
  onNoticeMouseMove?: () => void;
  onOpenRecord: () => void;
  recordId?: string | null;
  state: OverlayState;
}

const CenterOverlay: FC<CenterOverlayProps> = ({
  isVisible,
  levels,
  onCancel,
  onClose,
  onNoticeMouseLeave,
  onNoticeMouseMove,
  onOpenRecord,
  recordId,
  state,
}) => {
  const isNotice = isNoticeState(state);
  const isPaused = state === 'paused';
  const showsMicLevels = state === 'recording';
  const statusIcon = useMemo(() => {
    if (state === 'recording') return <MicIcon aria-hidden size={22} strokeWidth={2} />;
    if (state === 'paused') return <PauseIcon aria-hidden size={22} strokeWidth={2} />;
    if (state === 'transcribing') {
      return <LoaderCircleIcon aria-hidden className={styles.spinIcon} size={22} strokeWidth={2} />;
    }
    if (state === 'error') return <CircleAlertIcon aria-hidden size={22} strokeWidth={2} />;
    if (state === 'warning') return <TriangleAlertIcon aria-hidden size={22} strokeWidth={2} />;

    return <SparklesIcon aria-hidden size={22} strokeWidth={2} />;
  }, [state]);

  const className = [
    isVisible ? styles.overlayVisible : styles.overlay,
    state === 'error' ? styles.error : undefined,
    isPaused ? styles.paused : undefined,
    state === 'warning' ? styles.warning : undefined,
  ]
    .filter(Boolean)
    .join(' ');

  return (
    <div
      className={className}
      onMouseLeave={isNotice ? onNoticeMouseLeave : undefined}
      onMouseMove={isNotice ? onNoticeMouseMove : undefined}
    >
      <div className={styles.statusIcon}>{statusIcon}</div>
      <span className={styles.statusText}>{stateLabels[state]}</span>

      <div aria-hidden className={styles.levels}>
        {showsMicLevels &&
          levels.map((level, index) => (
            <span
              className={styles.level}
              key={index}
              style={{ height: `${level * MAX_LEVEL_HEIGHT}px` }}
            />
          ))}
      </div>

      {isNotice && (
        <div className={styles.actions}>
          {recordId ? (
            <button
              aria-label={openRecordLabel}
              className={styles.actionButton}
              type="button"
              onClick={onOpenRecord}
            >
              <SquareArrowOutUpRightIcon aria-hidden size={14} strokeWidth={2.4} />
              <span aria-hidden>{openRecordLabel}</span>
            </button>
          ) : null}

          <button
            aria-label={closeLabel}
            className={styles.actionButton}
            type="button"
            onClick={onClose}
          >
            <XIcon aria-hidden size={14} strokeWidth={2.4} />
            <span aria-hidden>{closeLabel}</span>
          </button>
        </div>
      )}

      {!isNotice && (
        <button
          aria-label={cancelLabel}
          className={styles.actionButton}
          type="button"
          onClick={onCancel}
        >
          <XIcon aria-hidden size={14} strokeWidth={2.4} />
          <span aria-hidden>{cancelLabel}</span>
        </button>
      )}
    </div>
  );
};

export default CenterOverlay;
