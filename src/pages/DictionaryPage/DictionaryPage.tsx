import { type FC, type KeyboardEvent, useEffect, useRef, useState } from 'react';
import {
  Button,
  Card,
  Empty,
  Input,
  type InputRef,
  message,
  Space,
  Spin,
  Tag,
  Tooltip,
} from 'antd';
import { PlusIcon } from 'lucide-react';
import { useTranslation } from 'react-i18next';

import styles from './DictionaryPage.module.scss';

import { useDictionaryStore } from '#/stores';

const getErrorMessage = (error: unknown) =>
  error instanceof Error ? error.message : String(error);

const DictionaryPage: FC = () => {
  const { t } = useTranslation();
  const [messageApi, messageContextHolder] = message.useMessage();
  const [wordInput, setWordInput] = useState('');
  const [isSaving, setIsSaving] = useState(false);
  const inputRef = useRef<InputRef>(null);

  const words = useDictionaryStore((s) => s.words);
  const isLoading = useDictionaryStore((s) => s.isLoading);
  const load = useDictionaryStore((s) => s.load);
  const storeAddWord = useDictionaryStore((s) => s.addWord);
  const storeRemoveWord = useDictionaryStore((s) => s.removeWord);

  useEffect(() => {
    queueMicrotask(() => {
      void load().catch((error: unknown) => {
        void messageApi.error(getErrorMessage(error));
      });
    });
  }, [load, messageApi]);

  // Refocus input after a save finishes so the user can type the next word immediately.
  const prevIsSavingRef = useRef(false);
  useEffect(() => {
    if (prevIsSavingRef.current && !isSaving) {
      inputRef.current?.focus();
    }
    prevIsSavingRef.current = isSaving;
  }, [isSaving]);

  const addWord = async () => {
    const normalizedWord = wordInput.trim();

    if (normalizedWord.length === 0) {
      return;
    }

    setIsSaving(true);

    try {
      await storeAddWord(normalizedWord);
      setWordInput('');
    } catch (error) {
      void messageApi.error(getErrorMessage(error));
    } finally {
      setIsSaving(false);
    }
  };

  const handleInputKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'Enter') {
      void addWord();
    }
  };

  const removeWord = async (wordToRemove: string) => {
    if (isSaving) return;
    setIsSaving(true);

    try {
      await storeRemoveWord(wordToRemove);
    } catch (error) {
      void messageApi.error(getErrorMessage(error));
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <>
      {messageContextHolder}
      <Card className={styles.dictionaryCard}>
        <Spin spinning={isLoading}>
          <div className={styles.dictionary}>
            <Space.Compact className={styles.addWord}>
              <Input
                ref={inputRef}
                aria-label={t('dictionary.newWord')}
                className={styles.wordInput}
                disabled={isLoading || isSaving}
                placeholder={t('dictionary.addWord')}
                value={wordInput}
                onChange={(event) => {
                  setWordInput(event.target.value);
                }}
                onKeyDown={handleInputKeyDown}
              />
              <Tooltip title={t('dictionary.addWord')}>
                <Button
                  aria-label={t('dictionary.addWord')}
                  className={styles.addButton}
                  disabled={isLoading || wordInput.trim().length === 0}
                  icon={<PlusIcon size={18} strokeWidth={2} />}
                  loading={isSaving}
                  onClick={() => {
                    void addWord();
                  }}
                />
              </Tooltip>
            </Space.Compact>

            {words.length > 0 ? (
              // pointer-events: none during save prevents close-button clicks without
              // toggling closable on every tag (which would cause a full list re-render).
              <div
                className={styles.words}
                style={{ pointerEvents: isSaving ? 'none' : undefined }}
              >
                {words.map((word) => (
                  <Tag
                    className={styles.word}
                    closable
                    color="blue"
                    variant="outlined"
                    key={word}
                    onClose={(event) => {
                      event.preventDefault();
                      void removeWord(word);
                    }}
                  >
                    {word}
                  </Tag>
                ))}
              </div>
            ) : isLoading ? null : (
              <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} />
            )}
          </div>
        </Spin>
      </Card>
    </>
  );
};

export default DictionaryPage;
