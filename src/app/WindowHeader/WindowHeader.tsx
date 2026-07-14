import { type FC, useEffect, useState } from 'react';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { Typography } from 'antd';
import clsx from 'clsx';
import { CopyIcon, MinusIcon, SquareIcon, XIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import styles from './WindowHeader.module.scss';

const appWindow = getCurrentWindow();

const closeWindow = async () => {
  await appWindow.close();
};

const minimizeWindow = async () => {
  await appWindow.minimize();
};

interface WindowHeaderProps {
  title: string;
}

interface WindowState {
  isClosable: boolean;
  isMaximizable: boolean;
  isMaximized: boolean;
  isMinimizable: boolean;
  isResizable: boolean;
}

const initialWindowState: WindowState = {
  isClosable: false,
  isMaximizable: false,
  isMaximized: false,
  isMinimizable: false,
  isResizable: false,
};

const WindowHeader: FC<WindowHeaderProps> = ({ title }) => {
  const { t } = useTranslation();
  const [windowState, setWindowState] = useState(initialWindowState);

  useEffect(() => {
    let isMounted = true;

    const refreshWindowState = async () => {
      const [isClosable, isMaximizable, isMaximized, isMinimizable, isResizable] =
        await Promise.all([
          appWindow.isClosable(),
          appWindow.isMaximizable(),
          appWindow.isMaximized(),
          appWindow.isMinimizable(),
          appWindow.isResizable(),
        ]);

      if (!isMounted) {
        return;
      }

      setWindowState({
        isClosable,
        isMaximizable,
        isMaximized,
        isMinimizable,
        isResizable,
      });
    };

    void refreshWindowState();

    let unlistenResize: (() => void) | undefined;

    void appWindow
      .onResized(() => {
        void refreshWindowState();
      })
      .then((fn) => {
        unlistenResize = fn;
        return null;
      });

    return () => {
      isMounted = false;
      unlistenResize?.();
    };
  }, []);

  const handleToggleMaximize = async () => {
    await appWindow.toggleMaximize();

    setWindowState((currentState) => ({
      ...currentState,
      isMaximized: !currentState.isMaximized,
    }));
  };

  const handleHeaderDoubleClick = async () => {
    if (!windowState.isResizable || !windowState.isMaximizable) {
      return;
    }

    await handleToggleMaximize();
  };

  return (
    <div className={styles.windowHeader}>
      <div
        className={styles.dragRegion}
        data-tauri-drag-region=""
        onDoubleClick={() => {
          void handleHeaderDoubleClick();
        }}
      >
        <Typography.Title className={styles.title} level={4}>
          {title}
        </Typography.Title>
      </div>

      <div className={styles.controls}>
        <button
          aria-label={t('common.windowControls.minimize')}
          className={styles.controlButton}
          disabled={!windowState.isMinimizable}
          tabIndex={-1}
          title={t('common.windowControls.minimize')}
          type="button"
          onClick={() => {
            void minimizeWindow();
          }}
        >
          <MinusIcon size={14} strokeWidth={2} />
        </button>
        <button
          aria-label={
            windowState.isMaximized
              ? t('common.windowControls.restore')
              : t('common.windowControls.maximize')
          }
          className={styles.controlButton}
          disabled={!windowState.isMaximizable || !windowState.isResizable}
          tabIndex={-1}
          title={
            windowState.isMaximized
              ? t('common.windowControls.restore')
              : t('common.windowControls.maximize')
          }
          type="button"
          onClick={() => {
            void handleToggleMaximize();
          }}
        >
          {windowState.isMaximized ? (
            <CopyIcon size={14} strokeWidth={2} />
          ) : (
            <SquareIcon size={14} strokeWidth={2} />
          )}
        </button>
        <button
          aria-label={t('common.windowControls.close')}
          className={clsx(styles.controlButton, styles.closeButton)}
          disabled={!windowState.isClosable}
          tabIndex={-1}
          title={t('common.windowControls.close')}
          type="button"
          onClick={() => {
            void closeWindow();
          }}
        >
          <XIcon size={14} strokeWidth={2} />
        </button>
      </div>
    </div>
  );
};

export default WindowHeader;
