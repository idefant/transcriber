//! Общие HTTP-клиенты приложения и прогрев соединения перед распознаванием.
//!
//! Раньше клиент создавался заново на каждый запрос, а вместе с ним — новый пул
//! соединений, поэтому DNS, TCP и TLS оплачивались повторно при каждой
//! диктовке. При RTT около 250 мс до Groq и OpenRouter это добавляло примерно
//! по 0,4 с на запрос. Здесь клиенты живут всё время работы приложения, а
//! соединение прогревается в момент старта записи — пока пользователь говорит.

use std::{
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

use reqwest::Client;
use uuid::Uuid;

use crate::{
    error::AppResult,
    metrics::{ProviderCall, ProviderCallStage, ProviderTimings},
};

/// Таймаут запросов обработки: распознавание длинной записи легально идёт долго.
const PROCESSING_TIMEOUT: Duration = Duration::from_secs(60);

/// Таймаут служебных запросов из настроек: список моделей и проверка ключа
/// должны отвечать быстро, иначе интерфейс подвисает на ожидании.
const INTERACTIVE_TIMEOUT: Duration = Duration::from_secs(20);

/// Сколько держать неиспользуемое соединение в пуле. Должно с запасом
/// перекрывать типичную паузу между стартом записи и отправкой аудио,
/// иначе прогрев не доживёт до самого запроса.
const POOL_IDLE_TIMEOUT: Duration = Duration::from_secs(90);

/// Насколько свежим должен быть прогрев, чтобы его можно было считать базовой
/// линией для вызовов этой диктовки. Более старый замер описывает уже другое
/// состояние сети.
const WARMUP_MAX_AGE: Duration = Duration::from_secs(300);

static PROCESSING_CLIENT: OnceLock<Client> = OnceLock::new();
static INTERACTIVE_CLIENT: OnceLock<Client> = OnceLock::new();
static LAST_WARMUP: OnceLock<Mutex<Option<WarmupSample>>> = OnceLock::new();

struct WarmupSample {
    call: ProviderCall,
    measured_at: Instant,
}

/// Клиент для распознавания и постобработки.
pub fn processing_client() -> AppResult<Client> {
    client(&PROCESSING_CLIENT, PROCESSING_TIMEOUT)
}

/// Клиент для служебных запросов из настроек: списка моделей и проверки ключа.
pub fn interactive_client() -> AppResult<Client> {
    client(&INTERACTIVE_CLIENT, INTERACTIVE_TIMEOUT)
}

fn client(cell: &'static OnceLock<Client>, timeout: Duration) -> AppResult<Client> {
    if let Some(client) = cell.get() {
        return Ok(client.clone());
    }

    let client = Client::builder()
        .timeout(timeout)
        .pool_idle_timeout(POOL_IDLE_TIMEOUT)
        .pool_max_idle_per_host(4)
        .tcp_keepalive(Duration::from_secs(60))
        .build()?;

    // Гонку за инициализацию мог выиграть другой поток; его клиент не хуже.
    Ok(cell.get_or_init(|| client).clone())
}

/// Открывает соединение к провайдерам ещё до того, как появится что отправлять.
///
/// Запускается при старте записи: пока пользователь говорит, DNS, TCP и TLS
/// успевают отработать, и запрос распознавания уходит по готовому соединению.
/// Ошибки намеренно проглатываются — прогрев лишь оптимизация, и его неудача
/// не должна мешать записи.
pub fn warm_up_connections(app: &tauri::AppHandle) {
    let app = app.clone();

    tauri::async_runtime::spawn(async move {
        let Ok(config) = crate::processing::load_processing_config(&app) else {
            return;
        };

        // Прогревается только маршрут распознавания: он идёт сразу после записи
        // и первым упирается в холодное соединение. К моменту постобработки
        // соединение к её хосту, если он тот же, уже горячее, а если другой —
        // её задержку прогрев всё равно не успел бы окупить.
        let Some(provider_id) = config.stt.provider_id.as_deref() else {
            return;
        };
        let Ok(credentials) = crate::providers::resolve_provider_credentials(&app, provider_id)
        else {
            return;
        };

        if let Some(sample) = probe(&credentials).await {
            store_warmup(sample);
        }
    });
}

/// Дёргает лёгкий эндпоинт провайдера, чтобы поднять соединение и заодно
/// измерить его цену. Ключ намеренно не отправляется: ответ 401 годится не
/// хуже 200 — соединение к этому моменту уже установлено, а лишний раз
/// пересылать ключ незачем.
async fn probe(credentials: &crate::providers::ProviderCredentials) -> Option<WarmupSample> {
    let client = processing_client().ok()?;
    let url = format!("{}/models", credentials.base_url.trim_end_matches('/'));
    let started_at = Instant::now();
    let response = client.get(url).send().await;
    let headers_ms = elapsed_ms(started_at);

    let (status, error_kind) = match response {
        Ok(response) => (Some(response.status().as_u16()), None),
        Err(error) => (None, Some(classify_error(&error))),
    };

    Some(WarmupSample {
        call: ProviderCall {
            id: Uuid::new_v4().to_string(),
            stage: ProviderCallStage::Warmup,
            provider_kind: credentials.kind.as_str().to_string(),
            provider_id: credentials.id.clone(),
            base_url: credentials.base_url.clone(),
            model: String::new(),
            status,
            error_kind,
            request_bytes: None,
            headers_ms,
            body_ms: 0,
            provider: ProviderTimings::default(),
        },
        measured_at: Instant::now(),
    })
}

fn store_warmup(sample: WarmupSample) {
    if let Ok(mut last) = warmup_cell().lock() {
        *last = Some(sample);
    }
}

/// Забирает последний прогрев, если он достаточно свежий.
///
/// Замер отдаётся ровно один раз: он описывает состояние соединения перед
/// конкретной диктовкой, и приписывать его следующей было бы неверно.
pub fn take_last_warmup() -> Option<ProviderCall> {
    let mut last = warmup_cell().lock().ok()?;
    let sample = last.take()?;

    if sample.measured_at.elapsed() > WARMUP_MAX_AGE {
        return None;
    }

    Some(sample.call)
}

fn warmup_cell() -> &'static Mutex<Option<WarmupSample>> {
    LAST_WARMUP.get_or_init(|| Mutex::new(None))
}

/// Грубая причина неудачи запроса, пригодная для группировки в метриках.
pub fn classify_error(error: &reqwest::Error) -> String {
    if error.is_timeout() {
        "timeout"
    } else if error.is_connect() {
        "connect"
    } else if error.is_decode() {
        "decode"
    } else if error.is_body() {
        "body"
    } else if error.is_request() {
        "request"
    } else {
        "other"
    }
    .to_string()
}

pub fn elapsed_ms(started_at: Instant) -> u64 {
    started_at
        .elapsed()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}
