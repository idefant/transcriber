import { type FC, useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { LoaderCircleIcon, MicIcon, SparklesIcon, XIcon } from 'lucide-react';

import styles from './RecordingOverlay.module.scss';

type OverlayState = 'processing' | 'recording' | 'transcribing';

const stateLabels: Record<OverlayState, string> = {
  processing: 'Processing',
  recording: 'Recording',
  transcribing: 'Transcribing',
};

const cancelLabel = 'Cancel dictation';

const RecordingOverlay: FC = () => {
  const [isVisible, setIsVisible] = useState(false);
  const [state, setState] = useState<OverlayState>('recording');
  const [levels, setLevels] = useState<number[]>([0, 0, 0]);

  useEffect(() => {
    const unlisteners = [
      listen<OverlayState>('show-overlay', (event) => {
        setState(event.payload);
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

  const statusIcon = useMemo(() => {
    if (state === 'recording') return <MicIcon aria-hidden size={15} strokeWidth={2.2} />;
    if (state === 'transcribing') {
      return (
        <LoaderCircleIcon aria-hidden className={styles.spinIcon} size={15} strokeWidth={2.2} />
      );
    }

    return <SparklesIcon aria-hidden size={15} strokeWidth={2.2} />;
  }, [state]);

  return (
    <div className={isVisible ? styles.overlayVisible : styles.overlay}>
      <div className={styles.status}>
        <span className={styles.statusIcon}>{statusIcon}</span>
        <span className={styles.statusText}>{stateLabels[state]}</span>
      </div>

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
        onClick={() => {
          void invoke('cancel_dictation');
        }}
      >
        <XIcon aria-hidden size={14} strokeWidth={2.4} />
      </button>
    </div>
  );
};

export default RecordingOverlay;
