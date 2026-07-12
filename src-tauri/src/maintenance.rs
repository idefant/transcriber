use std::{fs, io, path::Path};

use chrono::Local;
use serde::Serialize;
use tauri::Manager;

use crate::{
    db,
    error::{AppError, AppResult},
};

const RESET_BACKUP_PREFIX: &str = "reset-backup-";

/// Состояние запуска приложения, доступное фронтенду.
pub struct StartupState {
    /// Данные в каталоге записаны более новой версией приложения, чем эта.
    /// В этом состоянии приложение не работает с историей и предлагает
    /// пользователю обновиться или сбросить данные.
    pub data_too_new: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupStatus {
    data_too_new: bool,
}

#[tauri::command]
pub fn get_startup_status(app: tauri::AppHandle) -> StartupStatus {
    let data_too_new = app
        .try_state::<StartupState>()
        .map(|state| state.data_too_new)
        .unwrap_or(false);

    StartupStatus { data_too_new }
}

/// Сбрасывает данные приложения: переносит всё содержимое каталога данных в
/// резервную подпапку и перезапускает приложение с чистого листа.
///
/// Нужна как аварийный выход из состояния «данные новее кода»: откатить версию
/// приложения нельзя, поэтому пользователь может начать заново, не теряя
/// прежние данные безвозвратно — они остаются в резервной подпапке.
#[tauri::command]
pub fn reset_app_data(app: tauri::AppHandle) -> Result<(), String> {
    reset_app_data_inner(&app).map_err(AppError::into_message)
}

fn reset_app_data_inner(app: &tauri::AppHandle) -> AppResult<()> {
    // Закрываем соединение с БД: на Windows открытый файл нельзя переименовать.
    db::close(app);

    let dir = app.path().app_data_dir()?;
    let timestamp = Local::now().format("%Y%m%d-%H%M%S");
    let backup_dir = dir.join(format!("{RESET_BACKUP_PREFIX}{timestamp}"));
    fs::create_dir_all(&backup_dir)?;

    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();

            // Не переносим сам бэкап и ранее созданные бэкапы сбросов.
            if path == backup_dir || name.to_string_lossy().starts_with(RESET_BACKUP_PREFIX) {
                continue;
            }

            let _ = move_into_backup(&path, &backup_dir.join(&name));
        }
    }

    // Перезапускаем на чистом каталоге: миграции увидят свежую установку.
    // `restart` не возвращается.
    app.restart()
}

/// Переносит файл или папку в бэкап, повторяя попытки: сразу после закрытия
/// БД операционная система может ещё несколько мгновений держать файл занятым.
fn move_into_backup(source: &Path, destination: &Path) -> io::Result<()> {
    let mut last_error = None;

    for attempt in 0..5 {
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }

        match fs::rename(source, destination) {
            Ok(()) => return Ok(()),
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error.unwrap_or_else(|| io::Error::other("move failed")))
}
