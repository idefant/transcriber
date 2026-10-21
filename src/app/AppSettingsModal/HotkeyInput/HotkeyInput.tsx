import { type FC, useCallback, useEffect, useRef, useState } from 'react';
import { Button, Input, type InputRef, Space, Tooltip, Typography } from 'antd';
import { KeyboardIcon, LoaderIcon, XIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import * as dictationApi from '#/shared/dictationApi';
import { formatHotkeyFromEvent, MODIFIER_CODES } from '#/shared/hotkey';
import { lockHotkeyCapture, unlockHotkeyCapture } from '#/shared/hotkeyCaptureLock';

import styles from './HotkeyInput.module.scss';

interface HotkeyInputProps {
  allowEmpty: boolean;
  /** Разрешает ввод строки с сохранением по Enter или потере фокуса; Escape отменяет черновик. */
  allowManualInput?: boolean;
  defaultValue: string;
  emptyPlaceholder: string;
  /** Блокирует поле: комбинацию нельзя записать или сбросить, текущее значение остается видимым. */
  isDisabled?: boolean;
  onChange: (value: string) => void;
  resetLabel: string;
  value: string;
}

const HotkeyInput: FC<HotkeyInputProps> = ({
  allowEmpty,
  allowManualInput = true,
  defaultValue,
  emptyPlaceholder,
  isDisabled = false,
  onChange,
  resetLabel,
  value,
}) => {
  const { t } = useTranslation();
  const [isRecording, setIsRecording] = useState(false);
  const [isManualInputFocused, setIsManualInputFocused] = useState(false);
  const [draft, setDraft] = useState<string | null>(null);
  const [validationError, setValidationError] = useState<string>();
  const inputRef = useRef<InputRef>(null);
  const draftRef = useRef<string | null>(null);
  const editVersionRef = useRef(0);
  const pendingDraftRef = useRef<string | null>(null);

  const discardDraft = useCallback(() => {
    editVersionRef.current += 1;
    draftRef.current = null;
    pendingDraftRef.current = null;
    setDraft(null);
    setValidationError(undefined);
  }, []);

  const commitDraft = useCallback(async () => {
    const candidate = draftRef.current;
    if (candidate === null || pendingDraftRef.current === candidate) return;
    const version = ++editVersionRef.current;
    pendingDraftRef.current = candidate;
    try {
      const normalized =
        allowEmpty && candidate.trim() === '' ? '' : await dictationApi.normalizeHotkey(candidate);
      if (editVersionRef.current !== version) return;
      discardDraft();
      if (normalized !== value) onChange(normalized);
    } catch (error) {
      if (editVersionRef.current !== version) return;
      pendingDraftRef.current = null;
      setValidationError(String(error));
    }
  }, [allowEmpty, discardDraft, onChange, value]);

  useEffect(() => {
    if (!allowManualInput || isDisabled) return;
    // При уходе в другое приложение input может остаться activeElement без своего blur.
    const handleWindowBlur = () => {
      setIsManualInputFocused(false);
    };
    const handleWindowFocus = () => {
      setIsManualInputFocused(inputRef.current?.input === document.activeElement);
    };
    globalThis.addEventListener('blur', handleWindowBlur);
    globalThis.addEventListener('focus', handleWindowFocus);
    return () => {
      globalThis.removeEventListener('blur', handleWindowBlur);
      globalThis.removeEventListener('focus', handleWindowFocus);
    };
  }, [allowManualInput, isDisabled]);

  const isInteractionActive = !isDisabled && (isRecording || isManualInputFocused);
  useEffect(() => {
    if (!isInteractionActive) return;
    lockHotkeyCapture();
    return () => {
      unlockHotkeyCapture();
    };
  }, [isInteractionActive]);

  useEffect(
    () => () => {
      editVersionRef.current += 1;
    },
    [],
  );

  const startRecording = useCallback(() => {
    discardDraft();
    setIsRecording(true);
  }, [discardDraft]);

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

  const handleReset = useCallback(() => {
    discardDraft();
    onChange(defaultValue);
  }, [defaultValue, discardDraft, onChange]);

  const shouldShowReset = !isRecording && value !== defaultValue;

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
      pressedModifierCodes.clear();
    };
  }, [isRecording, onChange, stopRecording]);

  return (
    <div className={styles.root}>
      <Space.Compact block>
        <Input
          ref={inputRef}
          disabled={isDisabled}
          placeholder={allowEmpty ? emptyPlaceholder : undefined}
          readOnly={!allowManualInput || isRecording}
          title={allowManualInput && !isRecording ? t('settings.hotkeys.manualHint') : undefined}
          status={validationError === undefined ? undefined : 'error'}
          aria-invalid={validationError !== undefined}
          value={draft ?? value}
          onFocus={() => {
            if (allowManualInput) setIsManualInputFocused(true);
          }}
          onBlur={() => {
            setIsManualInputFocused(false);
            if (allowManualInput && !isRecording) void commitDraft();
          }}
          onChange={(event) => {
            editVersionRef.current += 1;
            pendingDraftRef.current = null;
            draftRef.current = event.target.value;
            setDraft(event.target.value);
            setValidationError(undefined);
          }}
          onKeyDown={(event) => {
            if (!allowManualInput || isRecording) return;
            if (event.key === 'Escape') {
              event.preventDefault();
              event.stopPropagation();
              discardDraft();
            } else if (event.key === 'Enter') {
              event.preventDefault();
              event.stopPropagation();
              void commitDraft();
            }
          }}
        />
        {shouldShowReset && (
          <Tooltip title={resetLabel}>
            <Button
              aria-label={resetLabel}
              disabled={isDisabled}
              icon={<XIcon size={16} strokeWidth={2} />}
              onMouseDown={(event) => {
                if (allowManualInput) {
                  event.preventDefault();
                  discardDraft();
                }
              }}
              onClick={handleReset}
            />
          </Tooltip>
        )}
        <Tooltip
          title={isRecording ? t('settings.hotkeys.recording') : t('settings.hotkeys.record')}
        >
          <Button
            aria-label={
              isRecording ? t('settings.hotkeys.recording') : t('settings.hotkeys.record')
            }
            disabled={isDisabled}
            icon={
              isRecording ? (
                <LoaderIcon size={16} strokeWidth={2} />
              ) : (
                <KeyboardIcon size={16} strokeWidth={2} />
              )
            }
            type={isRecording ? 'primary' : 'default'}
            onMouseDown={(event) => {
              if (allowManualInput) {
                event.preventDefault();
                discardDraft();
              }
            }}
            onClick={toggleRecording}
          />
        </Tooltip>
      </Space.Compact>
      {validationError !== undefined && (
        <Typography.Text type="danger" role="alert">
          {validationError}
        </Typography.Text>
      )}
    </div>
  );
};

export default HotkeyInput;
