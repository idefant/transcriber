# Component Structure

Use this structure for components in `pages`, `components`, `ui`, `app`, and similar feature folders:

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

- Always create the component folder, even when the component could fit in a single file.
- Do not create empty `helpers`, `hooks`, `types`, or `types.ts`; add them only when needed.
- `helpers/` and `hooks/` can use descriptive camelCase file names, for example `getFullname.ts` or `useTableColumns.ts`.
- `types/` files use PascalCase names.
- Use `types.ts` instead of `types/` when the component has only a small amount of simple types.
- Component props interface must be named `ComponentNameProps` and live before the component declaration in `ComponentName.tsx`.
- Prefer component declarations as constants:

```ts
const ComponentName: FC<ComponentNameProps> = () => {
  // ...
};

export default ComponentName;
```

- Use `FC` without a props generic when the component has no props. Polymorphic or generic components can use a typed `const` instead of `FC` when `FC` would weaken or break type inference.
- Components may use default exports. Prefer default exports for components unless a named export is genuinely needed.
- `index.ts` should import and export only the necessary component, variables, and types:

```ts
import ComponentName from './ComponentName';

export default ComponentName;
```

- Consumers must import from the module folder, not from concrete files:

```ts
import Button from '#/ui/Button';
```

- Do not import from `#/ui/Button/index.ts`, `#/ui/Button/Button.tsx`, or `#/ui/Button/types.ts`.
