# Правила className

Для объединения CSS-классов используй `clsx`, а не шаблонные строки или тернарники с конкатенацией. Импортируй дефолтный экспорт:

```ts
import clsx from 'clsx';
```

## Тернарник «базовый класс против базового с модификатором»

Когда одна ветка тернарника — это базовый класс, а другая — базовый класс плюс модификатор, замени тернарник на `clsx` с опциональным классом.

Не пиши так:

```tsx
className={isDetailsOpen ? `${styles.page} ${styles.withDetails}` : styles.page}
```

Пиши так:

```tsx
className={clsx(styles.page, isDetailsOpen && styles.withDetails)}
```

## Конкатенация всегда присутствующих классов

Когда несколько классов присутствуют всегда, замени шаблонную строку на `clsx`.

Не пиши так:

```tsx
className={`${styles.controlButton} ${styles.closeButton}`}
```

Пиши так:

```tsx
className={clsx(styles.controlButton, styles.closeButton)}
```

## Правила

- Предпочитай форму `isDetailsOpen && styles.withDetails` вместо объектной формы `{ [styles.withDetails]: isDetailsOpen }`.
- Сначала перечисляй стабильные (всегда присутствующие) классы, затем опциональные.
- Не оборачивай в `clsx` одиночный класс: `className={styles.page}` оставляй как есть.
- Не трогай тернарник, который выбирает один из двух взаимоисключающих классов, например `isOpen ? styles.chevronOpen : styles.chevron`. Он уже читается однозначно, а `clsx` тут ничего не улучшает и потребовал бы переделки SCSS (выделения общей базы в отдельный класс).
