# Структура компонента

Используй эту структуру для компонентов в `pages`, `components`, `ui`, `app` и подобных фиче-папках:

```text
ComponentName/
  ComponentName.tsx
  index.ts
  ComponentName.module.scss
  helpers/
  hooks/
  types/
  types.ts
```

- Всегда создавай папку компонента, даже если компонент мог бы поместиться в одном файле.
- Не создавай пустые `helpers`, `hooks`, `types` или `types.ts`; добавляй их только по необходимости.
- `helpers/` и `hooks/` могут использовать описательные имена файлов в camelCase, например `getFullname.ts` или `useTableColumns.ts`.
- Файлы в `types/` используют имена в PascalCase.
- Используй `types.ts` вместо `types/`, когда у компонента есть лишь небольшое количество простых типов.
- Интерфейс пропсов компонента должен называться `ComponentNameProps` и располагаться перед объявлением компонента в `ComponentName.tsx`.
- Предпочитай объявление компонента через константу:

```ts
const ComponentName: FC<ComponentNameProps> = () => {
  // ...
};

export default ComponentName;
```

- Используй `FC` без generic-параметра пропсов, когда у компонента нет пропсов. Полиморфные или generic-компоненты могут использовать типизированную `const` вместо `FC`, если `FC` ослабляет или ломает вывод типов.
- Компоненты могут использовать экспорт по умолчанию. Предпочитай экспорт по умолчанию для компонентов, если только именованный экспорт не нужен на самом деле.
- `index.ts` должен импортировать и экспортировать только необходимый компонент, переменные и типы:

```ts
import ComponentName from './ComponentName';

export default ComponentName;
```

- Потребители должны импортировать из папки модуля, а не из конкретных файлов:

```ts
import Button from '#/ui/Button';
```

- Не импортируй из `#/ui/Button/index.ts`, `#/ui/Button/Button.tsx` или `#/ui/Button/types.ts`.
