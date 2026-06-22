import { type FC, useEffect, useRef, useState } from 'react';
import { Form, Input, Typography } from 'antd';

import styles from './PromptField.module.scss';

interface PromptFieldProps {
  defaultValue: string;
  disabled: boolean;
  enabled: boolean;
  hint?: string;
  label: string;
  storedValue: string;
  onPersist: (value: string) => void;
}

const PERSIST_DELAY_MS = 500;

const PromptField: FC<PromptFieldProps> = ({
  defaultValue,
  disabled,
  enabled,
  hint,
  label,
  storedValue,
  onPersist,
}) => {
  const [value, setValue] = useState('');
  const storedRef = useRef(storedValue);
  const timeoutRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  useEffect(() => {
    storedRef.current = storedValue;
  }, [storedValue]);

  // Seed the editable value when the switch flips or the default loads. The stored
  // value is read through a ref (not a dependency) so our own debounced saves never
  // reset the caret while the user is typing.
  useEffect(() => {
    setValue(enabled ? storedRef.current : defaultValue);
  }, [enabled, defaultValue]);

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
    onPersist(value);
  };

  return (
    <Form.Item label={label}>
      <Input.TextArea
        autoSize={{ maxRows: 16, minRows: 3 }}
        className={styles.textArea}
        disabled={disabled || !enabled}
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
