# State Management Architecture

This document explains why the app uses Zustand stores instead of React Context, how stores are structured, and the constraints that must be preserved in future changes.

## Background

Prior to the Zustand migration, all shared state lived in React Context providers (`AppSettingsProvider`, `ProvidersProvider`, `ProcessingProvider`, `CatalogProvider`). History and Dictionary pages kept their data in local `useState` and fetched directly from Rust on every mount.

This caused several problems:

- Dictionary items were sorted on the Rust side (lowercase key) and again on the React side (`localeCompare('ru')`), producing inconsistent orderings and visible flickering when the list was replaced.
- Changing `closable` per-tag during save caused every tag to re-render twice.
- `flushSync` inside event handlers forced extra paints.
- The `history-updated` Tauri event carried no payload, so every update triggered a full cold-refetch of the entire month, creating a race with optimistic UI updates.
- The event subscription lived inside `HistoryPage`, so it was inactive when the page was not mounted.

## Store structure

All stores live in `src/stores/`. Each file owns one domain:

| File                 | Domain                                     |
| -------------------- | ------------------------------------------ |
| `settingsStore.ts`   | App settings (theme, language)             |
| `providersStore.ts`  | AI provider configs                        |
| `processingStore.ts` | STT / post-process config, default prompts |
| `catalogStore.ts`    | Model catalog                              |
| `dictionaryStore.ts` | Dictionary word list                       |
| `historyStore.ts`    | History groups, Tauri event subscription   |

`src/stores/index.ts` re-exports the raw stores and provides compatibility hooks (`useAppSettings`, `useProviders`, `useProcessing`, `useCatalog`) that wrap `useShallow` so consumers that destructure multiple fields do not re-render on unrelated changes.

## Canonical sort order belongs to Rust

**Do not sort data in the frontend.** The Rust side owns canonical ordering — dictionary words are sorted in `normalize_dictionary_words` (`src-tauri/src/dictionary.rs`), history groups in `sort_records` (`src-tauri/src/history.rs`). Stores receive the API response and write it to state as-is. Adding a client-side sort will desync ordering and cause flickering during the transition between the old and new list.

## Initial load

`StoreLoader` (a side-effect-only component rendered in `App.tsx`) calls every store's `load()` action once on mount via `queueMicrotask`. Stores set `isLoading: true` on entry and `false` in `finally`, so the `Spin` wrapper in pages reflects real loading state without a flash.

Pages do not call `load()` on their own mount — the data is already available (or loading) from `StoreLoader`. The exception is `HistoryPage`, which calls `historyStore.load()` explicitly because history is month-scoped and the current month changes during the session.

## History event subscription

`history-updated` is a Tauri event emitted from `src-tauri/src/history.rs` after every record mutation. The payload is `HistoryRecord | null`:

- `HistoryRecord` — a record was created or updated. `historyStore.mergeRecord()` finds the record by `id` in the current groups and replaces it in-place (one `set()` call, one re-render, no refetch).
- `null` — a record was deleted. The store does a silent reload of the current month (`load(month, { silent: true })`) so grouping is recalculated by Rust without showing the loading spinner.

The subscription is established in `HistorySubscription` (rendered in `App.tsx`), not in `HistoryPage`. This keeps the subscription alive regardless of which page is visible, so background dictations update the store even when the history page is not mounted.

**Do not move the subscription back into `HistoryPage`** — it would silently stop receiving updates while the user is on another page, and the next visit to history would show stale data until the month is reloaded.

## In-place merge vs. cold refetch

Before the migration, every `history-updated` event caused a full `get_history_groups` call. This created a race: the component applied an optimistic splice, then the refetch landed and replaced the list, causing a double-render and visible reordering.

The fix: `mergeRecord(record)` scans `groups` for a record with matching `id` and replaces it using `map()`. If the record is not found (new dictation from background), a single silent reload is triggered. Repeat handlers no longer apply any optimistic update — they fire the Rust command and wait for the event.

## Component-local vs. store state

Transient UI state stays local in components — do not promote it to a store:

- `isSaving` / `processingRecordId` — loading flags scoped to a single user action
- `activeDate` / `selectedRecord` in `HistoryPage` — derived from store groups via `useMemo`, not synced with a `useEffect`

When `HistoryPage` needs to reflect event-driven group changes in `activeDate` or `selectedRecord`, it computes them synchronously:

```ts
const activeDate = useMemo(
  () => (groups.some((g) => g.date === preferredDate) ? preferredDate : groups[0]?.date),
  [groups, preferredDate],
);
```

This avoids the `react-hooks/set-state-in-effect` lint error and eliminates the cascading render from a `useEffect` that calls `setState`.

## Default prompts

`processingStore` owns `defaultPrompts` and exposes `loadDefaultPrompts()`. The initial fetch happens in `StoreLoader`. `ProcessingSettingsForm` calls `loadDefaultPrompts()` inside a `useEffect` whenever `config.stt.language` or `settings.effectiveUiLanguage` changes, because Rust derives the default prompt text from those values. The component does not call `processingApi.getDefaultPrompts()` directly.
