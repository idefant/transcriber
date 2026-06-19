import { type FC, type KeyboardEvent, useCallback, useEffect, useMemo, useState } from 'react';
import { Button, Card, Empty, Input, message, Space, Spin, Tag, Tooltip } from 'antd';
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
      setIsSaving(false);
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

            {sortedWords.length === 0 ? (
              <Empty image={Empty.PRESENTED_IMAGE_SIMPLE} />
            ) : (
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
            )}
          </div>
        </Spin>
      </Card>
    </>
  );
};

export default DictionaryPage;
