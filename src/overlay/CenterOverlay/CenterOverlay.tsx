import { type FC, useMemo } from 'react';
import { LoaderCircleIcon, MicIcon, SparklesIcon, XIcon } from 'lucide-react';

import type { OverlayState } from '../types';
import { cancelLabel, stateLabels } from '../types';

import styles from './CenterOverlay.module.scss';

interface CenterOverlayProps {
  isVisible: boolean;
  levels: number[];
  onCancel: () => void;
  state: OverlayState;
}

const CenterOverlay: FC<CenterOverlayProps> = ({ isVisible, levels, onCancel, state }) => {
  const statusIcon = useMemo(() => {
    if (state === 'recording') return <MicIcon aria-hidden size={22} strokeWidth={2} />;
    if (state === 'transcribing') {
      return <LoaderCircleIcon aria-hidden className={styles.spinIcon} size={22} strokeWidth={2} />;
    }

    return <SparklesIcon aria-hidden size={22} strokeWidth={2} />;
  }, [state]);

  return (
    <div className={isVisible ? styles.overlayVisible : styles.overlay}>
      <div className={styles.statusIcon}>{statusIcon}</div>
      <span className={styles.statusText}>{stateLabels[state]}</span>

      <div aria-hidden className={styles.levels}>
        {levels.map((level, index) => (
          <span
            className={styles.level}
            key={index}
            style={{ transform: `scaleY(${Math.max(0.18, Math.min(1, level * 3.2))})` }}
          />
        ))}
      </div>

      <button
        aria-label={cancelLabel}
        className={styles.cancelButton}
        type="button"
        onClick={onCancel}
      >
        <XIcon aria-hidden size={14} strokeWidth={2.4} />
        <span aria-hidden>{cancelLabel}</span>
      </button>
    </div>
  );
};

export default CenterOverlay;
