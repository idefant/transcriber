import { type FC, useEffect, useRef, useState } from 'react';
import { Button, Form, Input, Typography } from 'antd';
import { RotateCcwIcon } from 'lucide-react';

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
}

const PERSIST_DELAY_MS = 500;

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
}) => {
  const [value, setValue] = useState(() => getSeedValue({ defaultValue, enabled, storedValue }));
  const storedRef = useRef(storedValue);
  const timeoutRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  useEffect(() => {
    storedRef.current = storedValue;
  }, [storedValue]);

  useEffect(
    () => () => {
      if (timeoutRef.current) clearTimeout(timeoutRef.current);
    },
    [],
  );

  const handleChange = (next: string) => {
    setValue(next);

    if (timeoutRef.current) clearTimeout(timeoutRef.current);
    timeoutRef.current = setTimeout(() => {
      onPersist(next);
    }, PERSIST_DELAY_MS);
  };

  const handleBlur = () => {
    if (timeoutRef.current) clearTimeout(timeoutRef.current);

    const persistedValue = storedRef.current ?? defaultValue;

    if (value !== persistedValue) {
      onPersist(value);
    }
  };

  const handleReset = () => {
    if (timeoutRef.current) clearTimeout(timeoutRef.current);
    setValue(defaultValue);
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
