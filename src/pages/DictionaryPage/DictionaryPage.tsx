import { type FC, type KeyboardEvent, useMemo, useState } from 'react';
import { Button, Card, Input, Space, Tag, Tooltip } from 'antd';
import { PlusIcon } from 'lucide-react';

import styles from './DictionaryPage.module.scss';

const initialWords = ['аудио', 'диктовка', 'запись', 'транскрипция'];

const DictionaryPage: FC = () => {
  const [wordInput, setWordInput] = useState('');
  const [words, setWords] = useState<string[]>(initialWords);

  const sortedWords = useMemo(
    () => words.toSorted((firstWord, secondWord) => firstWord.localeCompare(secondWord, 'ru')),
    [words],
  );

  const addWord = () => {
    const normalizedWord = wordInput.trim();

    if (normalizedWord.length === 0) {
      return;
    }

    setWords((currentWords) => {
      const hasWord = currentWords.some(
        (word) => word.localeCompare(normalizedWord, 'ru', { sensitivity: 'base' }) === 0,
      );

      if (hasWord) {
        return currentWords;
      }

      return [...currentWords, normalizedWord];
    });
    setWordInput('');
  };

  const handleInputKeyDown = (event: KeyboardEvent<HTMLInputElement>) => {
    if (event.key === 'Enter') {
      addWord();
    }
  };

  const removeWord = (wordToRemove: string) => {
    setWords((currentWords) => currentWords.filter((word) => word !== wordToRemove));
  };

  return (
    <Card>
      <div className={styles.dictionary}>
        <Space.Compact className={styles.addWord}>
          <Input
            aria-label="Новое слово"
            className={styles.wordInput}
            placeholder="Добавить слово"
            value={wordInput}
            onChange={(event) => {
              setWordInput(event.target.value);
            }}
            onKeyDown={handleInputKeyDown}
          />
          <Tooltip title="Добавить слово">
            <Button
              aria-label="Добавить слово"
              className={styles.addButton}
              disabled={wordInput.trim().length === 0}
              icon={<PlusIcon size={18} strokeWidth={2} />}
              onClick={addWord}
            />
          </Tooltip>
        </Space.Compact>

        <div className={styles.words}>
          {sortedWords.map((word) => (
            <Tag
              className={styles.word}
              closable
              color="blue"
              variant="outlined"
              key={word}
              onClose={() => {
                removeWord(word);
              }}
            >
              {word}
            </Tag>
          ))}
        </div>
      </div>
    </Card>
  );
};

export default DictionaryPage;
