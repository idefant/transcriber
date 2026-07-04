use std::fs;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::Manager;

use crate::{error::AppResult, storage};

const META_FILE_NAME: &str = "_meta.json";
const PROCESSING_FILE_NAME: &str = "processing.json";

// Increment this constant when a breaking storage schema change is made,
// and add a corresponding arm to run_migration_step.
const CURRENT_SCHEMA_VERSION: u32 = 2;

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
    match to_version {
        2 => migrate_to_v2(app),
        _ => Ok(()),
    }
}

fn migrate_to_v2(app: &tauri::AppHandle) -> AppResult<()> {
    let path = app.path().app_data_dir()?.join(PROCESSING_FILE_NAME);

    if !path.exists() {
        return Ok(());
    }

    let content = fs::read_to_string(&path)?;

    if content.trim().is_empty() {
        return Ok(());
    }

    let mut root: Value = match serde_json::from_str(&content) {
        Ok(value) => value,
        Err(_) => return Ok(()),
    };

    let mut changed = false;

    changed |= ensure_object_key(&mut root, &["stt"], "systemPrompt");
    changed |= ensure_object_key(&mut root, &["postProcess"], "systemPrompt");
    changed |= ensure_object_key(&mut root, &["postProcess"], "userPromptTemplate");
    changed |= remove_object_key(&mut root, &["stt"], "systemPromptTouched");
    changed |= remove_object_key(&mut root, &["postProcess"], "systemPromptTouched");
    changed |= remove_object_key(&mut root, &["postProcess"], "userPromptTemplateTouched");

    if changed {
        storage::save_json(app, PROCESSING_FILE_NAME, &root)?;
    }

    Ok(())
}

fn remove_object_key(root: &mut Value, path: &[&str], key: &str) -> bool {
    let mut current = root;

    for segment in path {
        let Some(next) = current.get_mut(*segment) else {
            return false;
        };

        current = next;
    }

    let Some(object) = current.as_object_mut() else {
        return false;
    };

    object.remove(key).is_some()
}

fn ensure_object_key(root: &mut Value, path: &[&str], key: &str) -> bool {
    let mut current = root;

    for segment in path {
        let Some(next) = current.get_mut(*segment) else {
            return false;
        };

        current = next;
    }

    let Some(object) = current.as_object_mut() else {
        return false;
    };

    if object.contains_key(key) {
        return false;
    }

    object.insert(key.to_string(), Value::Null);

    true
}
