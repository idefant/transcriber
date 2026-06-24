import { type FC, useCallback, useEffect, useState } from 'react';
import { Button, Input, Space, Tooltip } from 'antd';
import { KeyboardIcon, LoaderIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import * as dictationApi from '#/shared/dictationApi';
import { formatHotkeyFromEvent, MODIFIER_CODES } from '#/shared/hotkey';
import { lockHotkeyCapture, unlockHotkeyCapture } from '#/shared/hotkeyCaptureLock';

import styles from './HotkeyInput.module.scss';

interface HotkeyInputProps {
  onChange: (value: string) => void;
  value: string;
}

const HotkeyInput: FC<HotkeyInputProps> = ({ onChange, value }) => {
  const { t } = useTranslation();
  const [isRecording, setIsRecording] = useState(false);

  const startRecording = useCallback(() => {
    lockHotkeyCapture();
    void dictationApi.setHotkeyCaptureActive(true);
    setIsRecording(true);
  }, []);

  const stopRecording = useCallback(() => {
    setIsRecording(false);
  }, []);

  const toggleRecording = useCallback(() => {
    if (isRecording) {
      stopRecording();
    } else {
      startRecording();
    }
  }, [isRecording, startRecording, stopRecording]);

  useEffect(() => {
    if (!isRecording) {
      return;
    }

    const pressedModifierCodes = new Set<string>();

    const handleKeyDown = (event: KeyboardEvent) => {
      event.preventDefault();
      event.stopPropagation();

      if (MODIFIER_CODES.has(event.code)) {
        pressedModifierCodes.add(event.code);
        return;
      }

      const hotkey = formatHotkeyFromEvent(event, pressedModifierCodes);

      if (hotkey === undefined) {
        return;
      }

      onChange(hotkey);
      stopRecording();
    };

    const handleKeyUp = (event: KeyboardEvent) => {
      if (MODIFIER_CODES.has(event.code)) {
        pressedModifierCodes.delete(event.code);
      }
    };

    const handleBlur = () => {
      pressedModifierCodes.clear();
      stopRecording();
    };

    globalThis.addEventListener('keydown', handleKeyDown, { capture: true });
    globalThis.addEventListener('keyup', handleKeyUp, { capture: true });
    globalThis.addEventListener('blur', handleBlur);

    return () => {
      globalThis.removeEventListener('keydown', handleKeyDown, { capture: true });
      globalThis.removeEventListener('keyup', handleKeyUp, { capture: true });
      globalThis.removeEventListener('blur', handleBlur);
      unlockHotkeyCapture();
      void dictationApi.setHotkeyCaptureActive(false);
      pressedModifierCodes.clear();
    };
  }, [isRecording, onChange, stopRecording]);

  return (
    <Space.Compact className={styles.root}>
      <Input readOnly value={value} />
      <Tooltip title={isRecording ? t('settings.hotkeys.recording') : t('settings.hotkeys.record')}>
        <Button
          aria-label={isRecording ? t('settings.hotkeys.recording') : t('settings.hotkeys.record')}
          icon={
            isRecording ? (
              <LoaderIcon size={16} strokeWidth={2} />
            ) : (
              <KeyboardIcon size={16} strokeWidth={2} />
            )
          }
          type={isRecording ? 'primary' : 'default'}
          onClick={toggleRecording}
        />
      </Tooltip>
    </Space.Compact>
  );
};

export default HotkeyInput;
