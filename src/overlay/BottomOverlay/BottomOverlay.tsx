import { type FC, useMemo } from 'react';
import {
  CircleAlertIcon,
  LoaderCircleIcon,
  MicIcon,
  PauseIcon,
  SquareArrowOutUpRightIcon,
  TriangleAlertIcon,
  XIcon,
} from 'lucide-react';

import type { OverlayState } from '../types';
import { cancelLabel, closeLabel, isNoticeState, openRecordLabel, stateLabels } from '../types';

import styles from './BottomOverlay.module.scss';

const MAX_LEVEL_HEIGHT = 16;

interface BottomOverlayProps {
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

const BottomOverlay: FC<BottomOverlayProps> = ({
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
    if (state === 'recording') return <MicIcon aria-hidden size={15} strokeWidth={2.2} />;
    if (state === 'paused') return <PauseIcon aria-hidden size={15} strokeWidth={2.2} />;
    if (state === 'error') return <CircleAlertIcon aria-hidden size={15} strokeWidth={2.2} />;
    if (state === 'warning') return <TriangleAlertIcon aria-hidden size={15} strokeWidth={2.2} />;

    return <LoaderCircleIcon aria-hidden className={styles.spinIcon} size={15} strokeWidth={2.2} />;
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
      <div className={styles.status}>
        <span className={styles.statusIcon}>{statusIcon}</span>
        <span className={styles.statusText}>{stateLabels[state]}</span>
      </div>

      {isNotice && (
        <div className={styles.actions}>
          {recordId ? (
            <button
              aria-label={openRecordLabel}
              className={styles.actionButton}
              title={openRecordLabel}
              type="button"
              onClick={onOpenRecord}
            >
              <SquareArrowOutUpRightIcon aria-hidden size={14} strokeWidth={2.2} />
            </button>
          ) : null}
          <button
            aria-label={closeLabel}
            className={styles.actionButton}
            title={closeLabel}
            type="button"
            onClick={onClose}
          >
            <XIcon aria-hidden size={14} strokeWidth={2.4} />
          </button>
        </div>
      )}

      {!isNotice && (
        <>
          {showsMicLevels ? (
            <div aria-hidden className={styles.levels}>
              {levels.map((level, index) => (
                <span
                  className={styles.level}
                  key={index}
                  style={{ height: `${level * MAX_LEVEL_HEIGHT}px` }}
                />
              ))}
            </div>
          ) : null}

          <button
            aria-label={cancelLabel}
            className={styles.cancelButton}
            type="button"
            onClick={onCancel}
          >
            <XIcon aria-hidden size={14} strokeWidth={2.4} />
          </button>
        </>
      )}
    </div>
  );
};

export default BottomOverlay;
