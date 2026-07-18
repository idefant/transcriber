import { type FC, useState } from 'react';
import { Button, Form, Input, Typography } from 'antd';
import { RotateCcwIcon } from 'lucide-react';

import { useDebouncedCallback } from '#/shared/hooks';

import styles from './PromptField.module.scss';

interface PromptFieldProps {
  defaultValue: string;
  disabled: boolean;
  enabled: boolean;
  hint?: string;
  label: string;
  placeholder: string;
  resetLabel?: string;
  storedValue: string | null;
  onPersist: (value: string) => void;
  onReset?: () => void;
  onValueChange?: (value: string) => void;
}

const getSeedValue = ({
  defaultValue,
  enabled,
  storedValue,
}: Pick<PromptFieldProps, 'defaultValue' | 'enabled' | 'storedValue'>) => {
  if (!enabled) {
    return defaultValue;
  }

  return storedValue ?? defaultValue;
};

const PromptField: FC<PromptFieldProps> = ({
  defaultValue,
  disabled,
  enabled,
  hint,
  label,
  placeholder,
  resetLabel,
  storedValue,
  onPersist,
  onReset,
  onValueChange,
}) => {
  const [value, setValue] = useState(() => getSeedValue({ defaultValue, enabled, storedValue }));
  const persistPrompt = useDebouncedCallback(onPersist, 500);

  const handleChange = (next: string) => {
    setValue(next);
    onValueChange?.(next);
    persistPrompt.run(next);
  };

  // Уход с поля не должен ждать окончания задержки.
  const handleBlur = () => {
    persistPrompt.flush();
  };

  const handleReset = () => {
    persistPrompt.cancel();
    setValue(defaultValue);
    onValueChange?.(defaultValue);
    onReset?.();
  };

  return (
    <Form.Item className={styles.field}>
      <div className={styles.headerRow}>
        <Typography.Text>{label}</Typography.Text>
        {enabled && onReset && resetLabel && (
          <Button
            color="primary"
            disabled={disabled || storedValue === null}
            icon={<RotateCcwIcon size={16} strokeWidth={2} />}
            size="small"
            variant="text"
            onClick={handleReset}
          >
            {resetLabel}
          </Button>
        )}
      </div>

      <Input.TextArea
        autoSize={{ maxRows: 16, minRows: 3 }}
        className={styles.textArea}
        disabled={disabled || !enabled}
        placeholder={placeholder}
        value={value}
        onBlur={handleBlur}
        onChange={(event) => {
          handleChange(event.target.value);
        }}
      />
      {hint !== undefined && (
        <Typography.Text className={styles.hint} type="secondary">
          {hint}
        </Typography.Text>
      )}
    </Form.Item>
  );
};

export default PromptField;
