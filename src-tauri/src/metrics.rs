//! Сбор потайминговой разбивки обработки: сколько заняли локальные этапы и
//! сетевые вызовы каждой диктовки.
//!
//! Клиентское время вызова не равно времени провайдера, а провайдеры сообщают
//! своё время по-разному и не всегда. Что именно и откуда берётся, разобрано в
//! `docs/development/processing-timings.md`.

use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use chrono::{Duration as ChronoDuration, SecondsFormat, Utc};
use uuid::Uuid;

use crate::{db, debug_log::ModelRunSource, error::AppResult};

/// Сколько хранятся метрики. Строка весит около сотни байт, поэтому предел
/// выбран по полезности данных, а не по месту: за квартал видны и сезонные
/// изменения качества связи, и деградация конкретного провайдера.
const RETENTION_DAYS: i64 = 90;

/// Локальный этап обработки между остановкой записи и вставкой текста.
#[derive(Clone, Copy)]
pub enum RunStage {
    /// Остановка потока захвата и забор накопленных сэмплов.
    RecordStop,
    /// Локальное определение речи и обрезка тишины.
    Vad,
    /// Нормализация громкости, ресемплинг и кодирование WAV.
    Encode,
    /// Чтение конфигурации, словаря и учётных данных провайдера с диска.
    Snapshot,
    /// Запись WAV на диск и запись истории в базу.
    HistorySave,
    /// Помещение текста в буфер обмена и синтетическая вставка.
    Paste,
}

/// Чем закончилась операция. Пишется в `processing_runs.outcome`.
#[derive(Clone, Copy)]
pub enum RunOutcome {
    Completed,
    SttError,
    PostProcessError,
    Cancelled,
    Failed,
}

impl RunOutcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::SttError => "sttError",
            Self::PostProcessError => "postProcessError",
            Self::Cancelled => "cancelled",
            Self::Failed => "failed",
        }
    }
}

/// Какой сетевой вызов описывает строка `provider_calls`.
#[derive(Clone, Copy)]
pub enum ProviderCallStage {
    /// Распознавание речи.
    Stt,
    /// Постобработка текста.
    PostProcess,
    /// Прогрев соединения перед распознаванием. Его `headers_ms` — это цена
    /// установки соединения плюс RTT до хоста, то есть базовая линия, которую
    /// нужно вычесть из вызова, чтобы получить оценку времени провайдера.
    Warmup,
    /// Догрузка стоимости генерации OpenRouter, выполняемая после вставки текста.
    OpenrouterGeneration,
}

impl ProviderCallStage {
    fn as_str(self) -> &'static str {
        match self {
            Self::Stt => "stt",
            Self::PostProcess => "postProcess",
            Self::Warmup => "warmup",
            Self::OpenrouterGeneration => "openrouterGeneration",
        }
    }
}

/// Тайминги, сообщённые самим провайдером. Все поля необязательные: набор
/// зависит от провайдера, а у части маршрутов таких данных нет вовсе.
#[derive(Default)]
pub struct ProviderTimings {
    /// Время обработки на стороне провайдера целиком.
    pub total_ms: Option<u64>,
    /// Ожидание в очереди до начала обработки (сообщает Groq).
    pub queue_ms: Option<u64>,
    /// Время до первого токена (сообщает OpenRouter).
    pub ttft_ms: Option<u64>,
    /// Идентификатор запроса на стороне провайдера, для обращений в поддержку.
    pub request_id: Option<String>,
    /// Апстрим, фактически обработавший запрос (сообщает OpenRouter).
    pub upstream_provider: Option<String>,
    pub retry_after_ms: Option<u64>,
    /// Всё остальное, что удалось получить, но что не имеет своей колонки.
    pub raw: Option<serde_json::Value>,
}

/// Метрики одного HTTP-вызова, собранные вызывающей стороной.
pub struct ProviderCall {
    pub id: String,
    pub stage: ProviderCallStage,
    pub provider_kind: String,
    pub provider_id: String,
    pub base_url: String,
    pub model: String,
    pub status: Option<u16>,
    pub error_kind: Option<String>,
    pub request_bytes: Option<u64>,
    pub headers_ms: u64,
    pub body_ms: u64,
    pub provider: ProviderTimings,
}

#[derive(Default)]
struct RunTimerState {
    history_record_id: Option<String>,
    audio_duration_ms: Option<u64>,
    audio_bytes: Option<u64>,
    record_stop_ms: Option<u64>,
    vad_ms: Option<u64>,
    encode_ms: Option<u64>,
    snapshot_ms: Option<u64>,
    history_save_ms: Option<u64>,
    paste_ms: Option<u64>,
    calls: Vec<db::ProviderCallRow>,
    is_finished: bool,
}

/// Накопитель метрик одной операции обработки.
///
/// Клонируется дёшево и разделяет состояние между клонами, поэтому его можно
/// передавать в фоновые задачи, которые досчитывают метрики уже после вставки
/// текста. Пока [`RunTimer::finish`] не вызван, ничего не записывается.
#[derive(Clone)]
pub struct RunTimer {
    id: String,
    source: &'static str,
    started_at: Instant,
    state: Arc<Mutex<RunTimerState>>,
}

impl RunTimer {
    pub fn new(source: &ModelRunSource) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            source: source_as_str(source),
            started_at: Instant::now(),
            state: Arc::new(Mutex::new(RunTimerState::default())),
        }
    }

    /// Идентификатор операции. Он же связывает [`ProcessingRunRow`](db::ProcessingRunRow)
    /// со всеми её строками `provider_calls`.
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn set_history_record_id(&self, history_record_id: &str) {
        self.with_state(|state| state.history_record_id = Some(history_record_id.to_string()));
    }

    pub fn set_audio(&self, duration_ms: u64, bytes: usize) {
        self.with_state(|state| {
            state.audio_duration_ms = Some(duration_ms);
            state.audio_bytes = Some(bytes as u64);
        });
    }

    /// Записывает длительность локального этапа. Повторный вызов для того же
    /// этапа суммируется: постобработка может дозапросить снапшот повторно, и
    /// интересна суммарная цена этапа, а не последняя из попыток.
    pub fn record_stage(&self, stage: RunStage, elapsed: Duration) {
        let elapsed_ms = duration_as_ms(elapsed);

        self.with_state(|state| {
            let slot = match stage {
                RunStage::RecordStop => &mut state.record_stop_ms,
                RunStage::Vad => &mut state.vad_ms,
                RunStage::Encode => &mut state.encode_ms,
                RunStage::Snapshot => &mut state.snapshot_ms,
                RunStage::HistorySave => &mut state.history_save_ms,
                RunStage::Paste => &mut state.paste_ms,
            };

            *slot = Some(slot.unwrap_or(0) + elapsed_ms);
        });
    }

    /// Выполняет синхронный этап, измеряя его длительность.
    pub fn measure<T>(&self, stage: RunStage, action: impl FnOnce() -> T) -> T {
        let started_at = Instant::now();
        let result = action();

        self.record_stage(stage, started_at.elapsed());

        result
    }

    pub fn record_call(&self, call: ProviderCall) {
        let row = provider_call_row(&self.id, call);

        self.with_state(|state| state.calls.push(row));
    }

    /// Записывает накопленные метрики в базу.
    ///
    /// Вызывать только после того, как пользователь получил свой текст:
    /// запись в базу не должна попадать во время, которое сама же измеряет.
    /// Повторные вызовы игнорируются — операция может завершаться по нескольким
    /// веткам, и каждая из них вправе закрыть замер.
    pub fn finish(&self, app: &tauri::AppHandle, outcome: RunOutcome) {
        let total_ms = duration_as_ms(self.started_at.elapsed());
        let payload = self.with_state(|state| {
            if state.is_finished {
                return None;
            }

            state.is_finished = true;

            let run = db::ProcessingRunRow {
                id: self.id.clone(),
                history_record_id: state.history_record_id.clone(),
                created_at: now_rfc3339(),
                source: self.source.to_string(),
                outcome: outcome.as_str().to_string(),
                audio_duration_ms: state.audio_duration_ms,
                audio_bytes: state.audio_bytes,
                record_stop_ms: state.record_stop_ms,
                vad_ms: state.vad_ms,
                encode_ms: state.encode_ms,
                snapshot_ms: state.snapshot_ms,
                history_save_ms: state.history_save_ms,
                paste_ms: state.paste_ms,
                total_ms: Some(total_ms),
            };

            Some((run, std::mem::take(&mut state.calls)))
        });

        let Some(Some((run, calls))) = payload else {
            return;
        };

        // Сбой записи метрик не должен влиять на диктовку: пользователь свой
        // текст уже получил, а диагностика — вспомогательные данные.
        let _ = db::insert_metrics(app, &run, &calls);
    }

    fn with_state<T>(&self, action: impl FnOnce(&mut RunTimerState) -> T) -> Option<T> {
        self.state.lock().ok().map(|mut state| action(&mut state))
    }
}

/// Записывает метрики вызова, завершившегося уже после того, как операция была
/// закрыта: например, догрузки стоимости у OpenRouter.
///
/// Сводка операции к этому моменту в базе уже есть, поэтому дописывается только
/// строка вызова. Ошибки проглатываются — это диагностические данные.
pub fn record_background_call(app: &tauri::AppHandle, run_id: &str, call: ProviderCall) {
    let row = provider_call_row(run_id, call);

    let _ = db::insert_provider_call(app, &row);
}

fn provider_call_row(run_id: &str, call: ProviderCall) -> db::ProviderCallRow {
    db::ProviderCallRow {
        id: call.id,
        run_id: run_id.to_string(),
        created_at: now_rfc3339(),
        stage: call.stage.as_str().to_string(),
        provider_kind: call.provider_kind,
        provider_id: call.provider_id,
        base_host: host_of(&call.base_url),
        model: call.model,
        status: call.status,
        error_kind: call.error_kind,
        request_bytes: call.request_bytes,
        headers_ms: call.headers_ms,
        body_ms: call.body_ms,
        total_ms: call.headers_ms + call.body_ms,
        provider_total_ms: call.provider.total_ms,
        provider_queue_ms: call.provider.queue_ms,
        provider_ttft_ms: call.provider.ttft_ms,
        provider_request_id: call.provider.request_id,
        upstream_provider: call.provider.upstream_provider,
        retry_after_ms: call.provider.retry_after_ms,
        raw_timings: call
            .provider
            .raw
            .as_ref()
            .and_then(|raw| serde_json::to_string(raw).ok()),
    }
}

/// Удаляет метрики старше [`RETENTION_DAYS`]. Запускается в фоне при старте:
/// на горячем пути диктовки этой работе делать нечего.
pub fn cleanup_in_background(app: &tauri::AppHandle) {
    let app = app.clone();

    std::thread::spawn(move || {
        if let Err(error) = cleanup(&app) {
            eprintln!("Metrics cleanup failed: {}", error.into_message());
        }
    });
}

fn cleanup(app: &tauri::AppHandle) -> AppResult<usize> {
    let threshold = Utc::now() - ChronoDuration::days(RETENTION_DAYS);

    db::delete_metrics_before(app, &threshold.to_rfc3339_opts(SecondsFormat::Millis, true))
}

fn source_as_str(source: &ModelRunSource) -> &'static str {
    match source {
        ModelRunSource::Dictation => "dictation",
        ModelRunSource::HistoryRepeat => "historyRepeat",
        ModelRunSource::SettingsTest => "settingsTest",
    }
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

fn duration_as_ms(duration: Duration) -> u64 {
    duration.as_millis().try_into().unwrap_or(u64::MAX)
}

/// Хост базового URL провайдера. Путь и параметры отбрасываются: в них может
/// оказаться лишнее, а для разбора метрик достаточно хоста.
fn host_of(base_url: &str) -> String {
    reqwest::Url::parse(base_url)
        .ok()
        .and_then(|url| url.host_str().map(str::to_string))
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_host_without_path() {
        assert_eq!(host_of("https://api.groq.com/openai/v1"), "api.groq.com");
        assert_eq!(host_of("https://openrouter.ai/api/v1"), "openrouter.ai");
    }

    #[test]
    fn falls_back_when_base_url_is_not_a_url() {
        assert_eq!(host_of("not a url"), "unknown");
    }

    #[test]
    fn sums_repeated_stage_measurements() {
        let timer = RunTimer::new(&ModelRunSource::Dictation);

        timer.record_stage(RunStage::Snapshot, Duration::from_millis(30));
        timer.record_stage(RunStage::Snapshot, Duration::from_millis(12));

        let snapshot_ms = timer
            .with_state(|state| state.snapshot_ms)
            .expect("state should be readable");

        assert_eq!(snapshot_ms, Some(42));
    }

    #[test]
    fn keeps_stages_independent() {
        let timer = RunTimer::new(&ModelRunSource::Dictation);

        timer.record_stage(RunStage::Vad, Duration::from_millis(367));

        let (vad_ms, encode_ms) = timer
            .with_state(|state| (state.vad_ms, state.encode_ms))
            .expect("state should be readable");

        assert_eq!(vad_ms, Some(367));
        assert_eq!(encode_ms, None);
    }
}
