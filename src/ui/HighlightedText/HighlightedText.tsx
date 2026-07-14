import { type FC, Fragment } from 'react';
import { escapeRegExp } from 'lodash-es';

import styles from './HighlightedText.module.scss';

interface HighlightedTextProps {
  /** Искомая подстрока. Пустое значение выводит текст без подсветки. */
  query?: string;
  text: string;
}

/**
 * Выводит текст, подсвечивая в нём все вхождения `query` без учёта регистра.
 * Подсветка — только фон: цвет самого текста не меняется.
 */
const HighlightedText: FC<HighlightedTextProps> = ({ query, text }) => {
  if (query === undefined || query.length === 0) {
    return text;
  }

  // Скобочная группа заставляет split() оставить сами совпадения в массиве,
  // поэтому нечётные индексы — это найденные фрагменты.
  const parts = text.split(new RegExp(`(${escapeRegExp(query)})`, 'giu'));

  return parts.map((part, index) => {
    const key = `${index.toString()}:${part}`;

    return index % 2 === 0 ? (
      <Fragment key={key}>{part}</Fragment>
    ) : (
      <mark className={styles.mark} key={key}>
        {part}
      </mark>
    );
  });
};

export default HighlightedText;
