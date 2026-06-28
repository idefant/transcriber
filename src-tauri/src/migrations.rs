use serde::{Deserialize, Serialize};
use tauri::Manager;

use crate::{error::AppResult, storage};

const META_FILE_NAME: &str = "_meta.json";

// Increment this constant when a breaking storage schema change is made,
// and add a corresponding arm to run_migration_step.
const CURRENT_SCHEMA_VERSION: u32 = 1;

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct MetaStore {
    #[serde(default)]
    schema_version: u32,
}

/// Must be called once during app setup, before any domain stores are read.
/// Runs any pending migrations and writes the current schema version to _meta.json.
pub fn run(app: &tauri::AppHandle) -> AppResult<()> {
    let meta: MetaStore = storage::load_json_or_default(app, META_FILE_NAME)?;

    let from_version = if meta.schema_version == 0 {
        // _meta.json did not exist (or was empty/corrupt).
        // Check whether any domain data files are already present.
        // If so, this is an existing installation that predates versioning — treat as v1.
        // If not, it's a fresh install — start at the current version directly.
        let app_data_dir = app.path().app_data_dir()?;
        let has_existing_data = [
            "settings.json",
            "providers.json",
            "processing.json",
            "history.json",
        ]
        .iter()
        .any(|name| app_data_dir.join(name).exists());

        if has_existing_data {
            1
        } else {
            CURRENT_SCHEMA_VERSION
        }
    } else {
        meta.schema_version
    };

    if from_version < CURRENT_SCHEMA_VERSION {
        for target_version in (from_version + 1)..=CURRENT_SCHEMA_VERSION {
            run_migration_step(app, target_version)?;
        }
    }

    storage::save_json(
        app,
        META_FILE_NAME,
        &MetaStore {
            schema_version: CURRENT_SCHEMA_VERSION,
        },
    )?;

    Ok(())
}

fn run_migration_step(app: &tauri::AppHandle, to_version: u32) -> AppResult<()> {
    // Add future migration steps here, e.g.:
    // match to_version {
    //     2 => migrate_to_v2(app),
    //     _ => Ok(()),
    // }
    let _ = (app, to_version);
    Ok(())
}
