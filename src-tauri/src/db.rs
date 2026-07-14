use std::{fs, path::PathBuf, sync::Mutex};

use rusqlite::{params, Connection, OptionalExtension};
use tauri::Manager;

use crate::error::{AppError, AppResult};

const DB_FILE_NAME: &str = "app.sqlite3";

/// Соединение с базой истории, хранимое в managed state.
///
/// Обёрнуто в `Option`, чтобы [`close`] мог забрать и уронить соединение,
/// сняв блокировку файла на Windows перед сбросом данных.
pub struct HistoryDb(pub Mutex<Option<Connection>>);

/// Одна строка таблицы истории. `data` — это весь `HistoryRecord`,
/// сериализованный в JSON; тексты вынесены в колонки для полнотекстового
/// поиска, а время создания — для сортировки и фильтрации по периоду.
pub struct RecordRow {
    pub id: String,
    /// UTC RFC3339, лексикографически сортируется как хронология.
    pub created_at: String,
    pub transcription_text: String,
    pub postprocessing_text: String,
    /// Полный `HistoryRecord` в JSON.
    pub data: String,
}

fn to_app_error(error: rusqlite::Error) -> AppError {
    AppError::from(format!("history database error: {error}"))
}

fn db_path(app: &tauri::AppHandle) -> AppResult<PathBuf> {
    let dir = app.path().app_data_dir()?;
    fs::create_dir_all(&dir)?;
    Ok(dir.join(DB_FILE_NAME))
}

/// Открывает базу истории, включает WAL и создаёт схему. Должна вызываться
/// один раз при старте до чтения истории и до миграций, которым нужна база.
pub fn init(app: &tauri::AppHandle) -> AppResult<()> {
    let connection = Connection::open(db_path(app)?).map_err(to_app_error)?;

    // WAL: запись не блокирует чтение и наоборот, а вставка новой записи не
    // переписывает весь файл — ровно то, чего не хватало JSON-хранилищу.
    connection
        .pragma_update(None, "journal_mode", "WAL")
        .map_err(to_app_error)?;

    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS history_records (
                id TEXT PRIMARY KEY,
                created_at TEXT NOT NULL,
                transcription_text TEXT NOT NULL DEFAULT '',
                postprocessing_text TEXT NOT NULL DEFAULT '',
                data TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_history_created_at ON history_records(created_at DESC);
            CREATE VIRTUAL TABLE IF NOT EXISTS history_records_fts USING fts5(
                transcription_text,
                postprocessing_text,
                content = 'history_records',
                content_rowid = 'rowid',
                tokenize = 'trigram'
            );
            CREATE TRIGGER IF NOT EXISTS history_records_fts_after_insert
            AFTER INSERT ON history_records BEGIN
                INSERT INTO history_records_fts(rowid, transcription_text, postprocessing_text)
                VALUES (new.rowid, new.transcription_text, new.postprocessing_text);
            END;
            CREATE TRIGGER IF NOT EXISTS history_records_fts_after_update
            AFTER UPDATE OF transcription_text, postprocessing_text ON history_records BEGIN
                INSERT INTO history_records_fts(history_records_fts, rowid, transcription_text, postprocessing_text)
                VALUES ('delete', old.rowid, old.transcription_text, old.postprocessing_text);
                INSERT INTO history_records_fts(rowid, transcription_text, postprocessing_text)
                VALUES (new.rowid, new.transcription_text, new.postprocessing_text);
            END;
            CREATE TRIGGER IF NOT EXISTS history_records_fts_after_delete
            AFTER DELETE ON history_records BEGIN
                INSERT INTO history_records_fts(history_records_fts, rowid, transcription_text, postprocessing_text)
                VALUES ('delete', old.rowid, old.transcription_text, old.postprocessing_text);
            END;",
        )
        .map_err(to_app_error)?;

    app.manage(HistoryDb(Mutex::new(Some(connection))));

    Ok(())
}

/// Закрывает соединение (роняет его), чтобы снять блокировку файла базы.
/// Используется перед переносом каталога данных при сбросе.
pub fn close(app: &tauri::AppHandle) {
    if let Some(state) = app.try_state::<HistoryDb>() {
        if let Ok(mut guard) = state.0.lock() {
            let _ = guard.take();
        }
    }
}

/// Выполняет замыкание с открытым соединением. Возвращает ошибку, если база
/// не инициализирована (например, в состоянии «данные новее кода», когда
/// приложение намеренно не работает с историей).
fn with_connection<T>(
    app: &tauri::AppHandle,
    action: impl FnOnce(&Connection) -> AppResult<T>,
) -> AppResult<T> {
    let state = app.state::<HistoryDb>();
    let guard = state
        .0
        .lock()
        .map_err(|_| AppError::from("history database lock is poisoned"))?;
    let connection = guard
        .as_ref()
        .ok_or_else(|| AppError::from("history database is not available"))?;

    action(connection)
}

/// Вставляет запись или обновляет её по `id`.
pub fn upsert(app: &tauri::AppHandle, row: &RecordRow) -> AppResult<()> {
    with_connection(app, |connection| {
        connection
            .execute(
                "INSERT INTO history_records
                    (id, created_at, transcription_text, postprocessing_text, data)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(id) DO UPDATE SET
                    created_at = excluded.created_at,
                    transcription_text = excluded.transcription_text,
                    postprocessing_text = excluded.postprocessing_text,
                    data = excluded.data",
                params![
                    row.id,
                    row.created_at,
                    row.transcription_text,
                    row.postprocessing_text,
                    row.data
                ],
            )
            .map_err(to_app_error)?;

        Ok(())
    })
}

/// Возвращает JSON записи по `id` или `None`, если записи нет.
pub fn get_data(app: &tauri::AppHandle, id: &str) -> AppResult<Option<String>> {
    with_connection(app, |connection| {
        connection
            .query_row(
                "SELECT data FROM history_records WHERE id = ?1",
                params![id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(to_app_error)
    })
}

/// Удаляет запись по `id`. Возвращает `true`, если строка была удалена.
pub fn delete(app: &tauri::AppHandle, id: &str) -> AppResult<bool> {
    with_connection(app, |connection| {
        let affected = connection
            .execute("DELETE FROM history_records WHERE id = ?1", params![id])
            .map_err(to_app_error)?;

        Ok(affected > 0)
    })
}

/// JSON самой свежей записи (по `created_at`) или `None`, если история пуста.
pub fn latest_data(app: &tauri::AppHandle) -> AppResult<Option<String>> {
    with_connection(app, |connection| {
        connection
            .query_row(
                "SELECT data FROM history_records ORDER BY created_at DESC LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(to_app_error)
    })
}

/// JSON всех записей за полуоткрытый UTC-интервал в порядке убывания
/// `created_at`. При `None` возвращает всю историю в том же порядке.
pub fn list_data(
    app: &tauri::AppHandle,
    created_at_range: Option<(&str, &str)>,
) -> AppResult<Vec<String>> {
    with_connection(app, |connection| {
        let mut records = Vec::new();

        match created_at_range {
            Some((from, to)) => {
                let mut statement = connection
                    .prepare(
                        "SELECT data FROM history_records
                         WHERE created_at >= ?1 AND created_at < ?2
                         ORDER BY created_at DESC",
                    )
                    .map_err(to_app_error)?;
                let rows = statement
                    .query_map(params![from, to], |row| row.get::<_, String>(0))
                    .map_err(to_app_error)?;

                for row in rows {
                    records.push(row.map_err(to_app_error)?);
                }
            }
            None => {
                let mut statement = connection
                    .prepare("SELECT data FROM history_records ORDER BY created_at DESC")
                    .map_err(to_app_error)?;
                let rows = statement
                    .query_map([], |row| row.get::<_, String>(0))
                    .map_err(to_app_error)?;

                for row in rows {
                    records.push(row.map_err(to_app_error)?);
                }
            }
        }

        Ok(records)
    })
}

/// JSON записей, чьи тексты содержат искомую фразу, от новых к старым, вместе с
/// общим числом совпадений во всей истории (нужно для пагинации).
///
/// `match_query` — уже готовое выражение FTS5, а не сырой пользовательский ввод:
/// экранированием занимается вызывающая сторона. Порядок задаётся временем
/// создания, а не релевантностью, поэтому используется индекс `created_at`.
pub fn search_data(
    app: &tauri::AppHandle,
    match_query: &str,
    limit: u32,
    offset: u32,
) -> AppResult<(Vec<String>, u32)> {
    with_connection(app, |connection| {
        let total = connection
            .query_row(
                "SELECT COUNT(*) FROM history_records_fts WHERE history_records_fts MATCH ?1",
                params![match_query],
                |row| row.get::<_, u32>(0),
            )
            .map_err(to_app_error)?;

        let mut statement = connection
            .prepare(
                "SELECT record.data FROM history_records AS record
                 JOIN history_records_fts AS fts ON fts.rowid = record.rowid
                 WHERE history_records_fts MATCH ?1
                 ORDER BY record.created_at DESC
                 LIMIT ?2 OFFSET ?3",
            )
            .map_err(to_app_error)?;
        let rows = statement
            .query_map(params![match_query, limit, offset], |row| {
                row.get::<_, String>(0)
            })
            .map_err(to_app_error)?;

        let mut records = Vec::new();

        for row in rows {
            records.push(row.map_err(to_app_error)?);
        }

        Ok((records, total))
    })
}

/// Вставляет пачку записей одной транзакцией. Используется миграцией
/// импорта `history.json`.
pub fn import(app: &tauri::AppHandle, rows: &[RecordRow]) -> AppResult<usize> {
    with_connection(app, |connection| {
        let transaction = connection.unchecked_transaction().map_err(to_app_error)?;

        {
            let mut statement = transaction
                .prepare(
                    "INSERT INTO history_records
                        (id, created_at, transcription_text, postprocessing_text, data)
                     VALUES (?1, ?2, ?3, ?4, ?5)
                     ON CONFLICT(id) DO UPDATE SET
                        created_at = excluded.created_at,
                        transcription_text = excluded.transcription_text,
                        postprocessing_text = excluded.postprocessing_text,
                        data = excluded.data",
                )
                .map_err(to_app_error)?;

            for row in rows {
                statement
                    .execute(params![
                        row.id,
                        row.created_at,
                        row.transcription_text,
                        row.postprocessing_text,
                        row.data
                    ])
                    .map_err(to_app_error)?;
            }
        }

        transaction.commit().map_err(to_app_error)?;

        Ok(rows.len())
    })
}
