# File Structure

These rules describe project-level folders. Component folders must also follow
[component-structure.md](component-structure.md).

## Models

Long-lived application data types live in `src/models`.

Use PascalCase file names for model files:

```text
src/models/
  History.ts
  Provider.ts
  Settings.ts
```

Model files describe domain data that can be stored, loaded, or passed across
features for a long time. Do not put component props in `src/models`; keep props
near their component.

## Mocks

Temporary mock data lives in `src/mocks`.

Use descriptive camelCase file names:

```text
src/mocks/
  history.ts
  providers.ts
```

Mocks may import model types from `src/models`, but components should not own
large mock datasets inline.

## Pages

Pages own route-level state and layout composition. Extract child components
only for meaningful UI areas or complex logic, for example a records list or a
details panel.

Avoid extracting tiny fragments such as two buttons, a single table, or a few
form fields unless they become reused or carry their own business logic.
