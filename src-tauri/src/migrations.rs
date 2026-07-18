use std::fs;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::Manager;

use crate::{error::AppResult, history, storage};

const META_FILE_NAME: &str = "_meta.json";
const PROCESSING_FILE_NAME: &str = "processing.json";
const SETTINGS_FILE_NAME: &str = "settings.json";

// Увеличивай эту константу при обратно несовместимом изменении схемы
// хранилища и добавляй соответствующую ветку в run_migration_step.
const CURRENT_SCHEMA_VERSION: u32 = 4;

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct MetaStore {
    #[serde(default)]
    schema_version: u32,
}

/// Результат проверки версии хранилища при старте.
pub enum StartupData {
    /// Схема на текущей версии (миграции выполнены при необходимости).
    Ready,
    /// Данные записаны более новой версией приложения, чем эта. Миграции не
    /// выполнялись, `_meta.json` НЕ понижался. Приложение должно предложить
    /// пользователю обновиться или сбросить данные, а не читать их как свои.
    TooNew,
}

/// Должна вызываться один раз при настройке приложения, после инициализации
/// базы истории ([`crate::db::init`]) и до чтения любых доменных хранилищ.
/// Выполняет все ожидающие миграции и записывает текущую версию схемы.
///
/// Если версия данных выше известной коду, возвращает [`StartupData::TooNew`]
/// и НЕ трогает `_meta.json`: понижать версию нельзя, иначе старый код начнёт
/// читать данные более новой схемы как свои и повредит их.
pub fn run(app: &tauri::AppHandle) -> AppResult<StartupData> {
    let meta: MetaStore = storage::load_json_or_default(app, META_FILE_NAME)?;

    let from_version = if meta.schema_version == 0 {
        // _meta.json не существовал (или был пустым/повреждённым).
        // Проверяем, присутствуют ли уже какие-либо файлы доменных данных.
        // Если да, это существующая установка, появившаяся до введения версионирования, — считаем её v1.
        // Если нет, это новая установка — сразу начинаем с текущей версии.
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

    if from_version > CURRENT_SCHEMA_VERSION {
        return Ok(StartupData::TooNew);
    }

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

    Ok(StartupData::Ready)
}

fn run_migration_step(app: &tauri::AppHandle, to_version: u32) -> AppResult<()> {
    match to_version {
        2 => migrate_to_v2(app),
        3 => migrate_to_v3(app),
        4 => migrate_to_v4(app),
        _ => Ok(()),
    }
}

/// Заменяет булеву настройку `isMuteWhileRecordingEnabled` режимом
/// `recordingAudioMode`: включённое заглушение становится `mute`, выключенное —
/// `off` (звук системы не трогаем).
///
/// Шаг обязателен: `AppSettings` игнорирует незнакомые ключи, поэтому без него
/// старый ключ молча отбросился бы, а новое поле взяло бы дефолт `mute` — то есть
/// у выключивших заглушение оно бы само включилось.
fn migrate_to_v4(app: &tauri::AppHandle) -> AppResult<()> {
    let path = app.path().app_data_dir()?.join(SETTINGS_FILE_NAME);

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

    // Старого ключа нет (или он не булев) — настройки уже в новом формате.
    let Some(was_mute_enabled) = root
        .get("isMuteWhileRecordingEnabled")
        .and_then(Value::as_bool)
    else {
        return Ok(());
    };

    let Some(object) = root.as_object_mut() else {
        return Ok(());
    };

    // or_insert_with, а не insert: уже выставленный новый режим важнее старого флага.
    object
        .entry("recordingAudioMode")
        .or_insert_with(|| Value::from(if was_mute_enabled { "mute" } else { "off" }));
    object.remove("isMuteWhileRecordingEnabled");

    storage::save_json(app, SETTINGS_FILE_NAME, &root)?;

    Ok(())
}

/// Переносит историю из `history.json` в базу SQLite. Реализация — в
/// `history`, где живёт тип записи; исходный JSON сохраняется как резервная
/// копия, а не удаляется.
fn migrate_to_v3(app: &tauri::AppHandle) -> AppResult<()> {
    history::migrate_history_json_to_db(app)
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
