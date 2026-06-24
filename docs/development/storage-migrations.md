# Storage Versioning and Migrations

## Overview

The app stores domain data as separate JSON files in the Tauri `app_data_dir`
(`com.transcriber.desktop`). The storage layer lives in
`src-tauri/src/storage.rs`; the migration runner in
`src-tauri/src/migrations.rs`.

## Schema versioning

A lightweight metadata file `_meta.json` tracks the current schema version:

```json
{ "schemaVersion": 1 }
```

`migrations::run` is called once in `lib.rs::setup` before any domain stores
are read. It checks `_meta.json` and runs any pending migration steps, then
rewrites `_meta.json` with `CURRENT_SCHEMA_VERSION`.

### First-run detection

When `_meta.json` is absent:

- **Domain files exist** → existing installation that predates versioning.
  Treat as schema v1 (no migration needed, just stamp the meta file).
- **No domain files** → fresh install. Stamp `CURRENT_SCHEMA_VERSION`
  directly, skip all migration steps.

## Resilient loading

Two loading functions exist; which one to use depends on how critical silent
fallback is for the domain:

### `storage::load_json_or_default` — silent fallback

1. File missing or empty → return `T::default()`.
2. JSON parses successfully → return the value.
3. JSON fails to parse → rename the file to
   `<name>.corrupt-<YYYYMMDD-HHMMSS>` as a backup, return `T::default()`.

Use for: `settings.json`, `processing.json`, `dictionary.json` — domains
where the default is a sensible starting point and there are no cross-domain
ID references that would break silently.

### `storage::load_json_strict` — backup + error on corrupt

1. File missing or empty → return `T::default()` (e.g. fresh install).
2. JSON parses successfully → return the value.
3. JSON fails to parse → rename the file as a backup, return `Err` with a
   descriptive message.

Use for: `providers.json`, `history.json` — domains where silent fallback
to an empty default would cause cascading failures (dangling provider IDs
in processing config) or significant data loss (transcription history).

When `load_json_strict` errors, the error propagates through the Tauri
command as a rejected Promise, and the frontend store shows an error state
in the relevant UI section. The corrupt file is preserved in the backup so
the user can recover their data.

> ⚠️ Always write a migration step rather than relying on the fallback
> when a domain's schema changes in a breaking way.

## Atomic writes

`storage::save_json` writes to `<name>.tmp` first, then `fs::rename` to the
target. `rename` on the same filesystem volume is atomic — a crash mid-write
cannot leave a partial file.

## Adding a migration step

1. Increment `CURRENT_SCHEMA_VERSION` in `src-tauri/src/migrations.rs`.
2. Add a new arm to `run_migration_step`:

```rust
2 => migrate_to_v2(app),
```

3. Implement `migrate_to_v2`:
   - Read the old JSON with `fs::read_to_string` / `serde_json::from_str`.
   - Transform the data.
   - Write back with `storage::save_json`.
   - If reading fails, either return an error (aborts startup) or back up and
     reset, depending on acceptable data loss for that domain.

Migration steps run in order from `from_version + 1` to
`CURRENT_SCHEMA_VERSION`. Each step must be idempotent if possible.
