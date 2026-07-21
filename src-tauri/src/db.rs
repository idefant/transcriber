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

/// Сводные метрики одной операции обработки: сколько заняли локальные этапы
/// между остановкой записи и вставкой текста.
///
/// Сетевые вызовы этой же операции лежат в [`ProviderCallRow`] и связаны с ней
/// по `run_id`. Строка пишется один раз, после вставки текста, чтобы запись в
/// базу не попадала в измеряемый путь.
pub struct ProcessingRunRow {
    /// Идентификатор операции, общий для всех её сетевых вызовов.
    pub id: String,
    /// `None` для тестов из настроек и для сбоев до создания записи истории.
    pub history_record_id: Option<String>,
    /// UTC RFC3339, лексикографически сортируется как хронология.
    pub created_at: String,
    pub source: String,
    pub outcome: String,
    pub audio_duration_ms: Option<u64>,
    pub audio_bytes: Option<u64>,
    pub record_stop_ms: Option<u64>,
    pub vad_ms: Option<u64>,
    pub encode_ms: Option<u64>,
    pub snapshot_ms: Option<u64>,
    pub history_save_ms: Option<u64>,
    pub paste_ms: Option<u64>,
    /// От остановки записи до вставки текста.
    pub total_ms: Option<u64>,
}

/// Метрики одного HTTP-вызова провайдера.
///
/// Клиентские (`headers_ms`, `body_ms`, `total_ms`) заполняются всегда,
/// провайдерские (`provider_*`) — только если провайдер их сообщает; чем
/// какой провайдер делится, описано в `docs/development/processing-timings.md`.
pub struct ProviderCallRow {
    /// `operation_id` вызова, он же корреляционный идентификатор в debug-логе.
    pub id: String,
    /// `ProcessingRunRow::id` операции, которой принадлежит вызов.
    pub run_id: String,
    pub created_at: String,
    /// `stt` | `postProcess` | `warmup` | `openrouterGeneration`.
    pub stage: String,
    pub provider_kind: String,
    pub provider_id: String,
    /// Только хост базового URL: путь и параметры могут содержать лишнее.
    pub base_host: String,
    pub model: String,
    /// `None`, если ответ не получен вовсе (сетевая ошибка, таймаут).
    pub status: Option<u16>,
    pub error_kind: Option<String>,
    pub request_bytes: Option<u64>,
    /// До получения заголовков ответа: отправка тела плюс работа провайдера.
    pub headers_ms: u64,
    /// Дочитывание тела ответа после заголовков.
    pub body_ms: u64,
    pub total_ms: u64,
    pub provider_total_ms: Option<u64>,
    pub provider_queue_ms: Option<u64>,
    pub provider_ttft_ms: Option<u64>,
    pub provider_request_id: Option<String>,
    /// Апстрим, фактически обработавший запрос (сообщает только OpenRouter).
    pub upstream_provider: Option<String>,
    pub retry_after_ms: Option<u64>,
    /// JSON с тем, что не легло в отдельные колонки.
    pub raw_timings: Option<String>,
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

    // Таблицы метрик добавляются здесь, а не шагом миграции: изменение чисто
    // аддитивное, поэтому версия схемы не растёт и более старая версия
    // приложения просто не увидит эти таблицы вместо отказа читать данные.
    //
    // Метрики намеренно не связаны внешним ключом с history_records и не
    // удаляются вместе с записью: причина медленной обработки остаётся
    // интересной и после того, как саму запись стёрли.
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS processing_runs (
                id TEXT PRIMARY KEY,
                history_record_id TEXT,
                created_at TEXT NOT NULL,
                source TEXT NOT NULL,
                outcome TEXT NOT NULL,
                audio_duration_ms INTEGER,
                audio_bytes INTEGER,
                record_stop_ms INTEGER,
                vad_ms INTEGER,
                encode_ms INTEGER,
                snapshot_ms INTEGER,
                history_save_ms INTEGER,
                paste_ms INTEGER,
                total_ms INTEGER
            );
            CREATE INDEX IF NOT EXISTS idx_processing_runs_created_at
                ON processing_runs(created_at DESC);
            CREATE TABLE IF NOT EXISTS provider_calls (
                id TEXT PRIMARY KEY,
                run_id TEXT NOT NULL,
                created_at TEXT NOT NULL,
                stage TEXT NOT NULL,
                provider_kind TEXT NOT NULL,
                provider_id TEXT NOT NULL,
                base_host TEXT NOT NULL,
                model TEXT NOT NULL,
                status INTEGER,
                error_kind TEXT,
                request_bytes INTEGER,
                headers_ms INTEGER NOT NULL,
                body_ms INTEGER NOT NULL,
                total_ms INTEGER NOT NULL,
                provider_total_ms INTEGER,
                provider_queue_ms INTEGER,
                provider_ttft_ms INTEGER,
                provider_request_id TEXT,
                upstream_provider TEXT,
                retry_after_ms INTEGER,
                raw_timings TEXT
            );
            CREATE INDEX IF NOT EXISTS idx_provider_calls_run ON provider_calls(run_id);
            CREATE INDEX IF NOT EXISTS idx_provider_calls_created_at
                ON provider_calls(created_at DESC);",
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

/// `created_at` самой старой записи или `None`, если история пуста.
/// Возвращается только эта колонка: разбирать `data` ради нижней границы
/// выбора месяца не нужно.
pub fn oldest_created_at(app: &tauri::AppHandle) -> AppResult<Option<String>> {
    with_connection(app, |connection| {
        connection
            .query_row(
                "SELECT created_at FROM history_records ORDER BY created_at ASC LIMIT 1",
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

/// Записывает сводку операции и метрики всех её сетевых вызовов одной
/// транзакцией.
///
/// Вызывается уже после вставки текста пользователю: запись в базу не должна
/// попадать в то время, которое сама же измеряет.
pub fn insert_metrics(
    app: &tauri::AppHandle,
    run: &ProcessingRunRow,
    calls: &[ProviderCallRow],
) -> AppResult<()> {
    with_connection(app, |connection| {
        let transaction = connection.unchecked_transaction().map_err(to_app_error)?;

        transaction
            .execute(
                "INSERT INTO processing_runs
                    (id, history_record_id, created_at, source, outcome,
                     audio_duration_ms, audio_bytes, record_stop_ms, vad_ms, encode_ms,
                     snapshot_ms, history_save_ms, paste_ms, total_ms)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
                 ON CONFLICT(id) DO NOTHING",
                params![
                    run.id,
                    run.history_record_id,
                    run.created_at,
                    run.source,
                    run.outcome,
                    run.audio_duration_ms,
                    run.audio_bytes,
                    run.record_stop_ms,
                    run.vad_ms,
                    run.encode_ms,
                    run.snapshot_ms,
                    run.history_save_ms,
                    run.paste_ms,
                    run.total_ms,
                ],
            )
            .map_err(to_app_error)?;

        {
            let mut statement = transaction
                .prepare(
                    "INSERT INTO provider_calls
                        (id, run_id, created_at, stage, provider_kind, provider_id, base_host,
                         model, status, error_kind, request_bytes, headers_ms, body_ms, total_ms,
                         provider_total_ms, provider_queue_ms, provider_ttft_ms,
                         provider_request_id, upstream_provider, retry_after_ms, raw_timings)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                             ?15, ?16, ?17, ?18, ?19, ?20, ?21)
                     ON CONFLICT(id) DO NOTHING",
                )
                .map_err(to_app_error)?;

            for call in calls {
                statement
                    .execute(params![
                        call.id,
                        call.run_id,
                        call.created_at,
                        call.stage,
                        call.provider_kind,
                        call.provider_id,
                        call.base_host,
                        call.model,
                        call.status,
                        call.error_kind,
                        call.request_bytes,
                        call.headers_ms,
                        call.body_ms,
                        call.total_ms,
                        call.provider_total_ms,
                        call.provider_queue_ms,
                        call.provider_ttft_ms,
                        call.provider_request_id,
                        call.upstream_provider,
                        call.retry_after_ms,
                        call.raw_timings,
                    ])
                    .map_err(to_app_error)?;
            }
        }

        transaction.commit().map_err(to_app_error)?;

        Ok(())
    })
}

/// Записывает метрики одного вызова к уже сохранённой операции.
/// Используется для работы, которая завершается после закрытия операции.
pub fn insert_provider_call(app: &tauri::AppHandle, call: &ProviderCallRow) -> AppResult<()> {
    with_connection(app, |connection| {
        connection
            .execute(
                "INSERT INTO provider_calls
                    (id, run_id, created_at, stage, provider_kind, provider_id, base_host,
                     model, status, error_kind, request_bytes, headers_ms, body_ms, total_ms,
                     provider_total_ms, provider_queue_ms, provider_ttft_ms,
                     provider_request_id, upstream_provider, retry_after_ms, raw_timings)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
                         ?15, ?16, ?17, ?18, ?19, ?20, ?21)
                 ON CONFLICT(id) DO NOTHING",
                params![
                    call.id,
                    call.run_id,
                    call.created_at,
                    call.stage,
                    call.provider_kind,
                    call.provider_id,
                    call.base_host,
                    call.model,
                    call.status,
                    call.error_kind,
                    call.request_bytes,
                    call.headers_ms,
                    call.body_ms,
                    call.total_ms,
                    call.provider_total_ms,
                    call.provider_queue_ms,
                    call.provider_ttft_ms,
                    call.provider_request_id,
                    call.upstream_provider,
                    call.retry_after_ms,
                    call.raw_timings,
                ],
            )
            .map_err(to_app_error)?;

        Ok(())
    })
}

/// Удаляет метрики, созданные раньше `created_at_before` (UTC RFC3339).
/// Возвращает число удалённых строк обеих таблиц.
pub fn delete_metrics_before(app: &tauri::AppHandle, created_at_before: &str) -> AppResult<usize> {
    with_connection(app, |connection| {
        let calls = connection
            .execute(
                "DELETE FROM provider_calls WHERE created_at < ?1",
                params![created_at_before],
            )
            .map_err(to_app_error)?;
        let runs = connection
            .execute(
                "DELETE FROM processing_runs WHERE created_at < ?1",
                params![created_at_before],
            )
            .map_err(to_app_error)?;

        Ok(calls + runs)
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
