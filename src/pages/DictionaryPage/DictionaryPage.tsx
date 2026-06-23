import {
  type FC,
  type KeyboardEvent,
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
} from 'react';
import { flushSync } from 'react-dom';
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

import * as dictionaryApi from '#/shared/dictionaryApi';

import styles from './DictionaryPage.module.scss';

const getErrorMessage = (error: unknown) =>
  error instanceof Error ? error.message : String(error);

const DictionaryPage: FC = () => {
  const { t } = useTranslation();
  const [messageApi, messageContextHolder] = message.useMessage();
  const [wordInput, setWordInput] = useState('');
  const [words, setWords] = useState<string[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const inputRef = useRef<InputRef>(null);

  const sortedWords = useMemo(
    () => words.toSorted((firstWord, secondWord) => firstWord.localeCompare(secondWord, 'ru')),
    [words],
  );

  const loadWords = useCallback(async () => {
    setIsLoading(true);

    try {
      const nextWords = await dictionaryApi.getDictionaryWords();

      setWords(nextWords);
    } catch (error) {
      void messageApi.error(getErrorMessage(error));
    } finally {
      setIsLoading(false);
    }
  }, [messageApi]);

  useEffect(() => {
    queueMicrotask(() => {
      void loadWords();
    });
  }, [loadWords]);

  const addWord = async () => {
    const normalizedWord = wordInput.trim();

    if (normalizedWord.length === 0) {
      return;
    }

    setIsSaving(true);

    try {
      const nextWords = await dictionaryApi.addDictionaryWord(normalizedWord);

      setWords(nextWords);
      setWordInput('');
    } catch (error) {
      void messageApi.error(getErrorMessage(error));
    } finally {
      flushSync(() => {
        setIsSaving(false);
      });
      inputRef.current?.focus();
    }
  };

  const handleInputKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'Enter') {
      void addWord();
    }
  };

  const removeWord = async (wordToRemove: string) => {
    setIsSaving(true);

    try {
      const nextWords = await dictionaryApi.deleteDictionaryWord(wordToRemove);

      setWords(nextWords);
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

            {sortedWords.length > 0 ? (
              <div className={styles.words}>
                {sortedWords.map((word) => (
                  <Tag
                    className={styles.word}
                    closable={!isSaving}
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
