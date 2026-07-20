use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, OnceLock,
};
use std::{fs, str::FromStr};

use sentry::{protocol::Event, ClientOptions};
use tauri::Manager;

use crate::{error::AppResult, settings};

static IS_TELEMETRY_ENABLED: OnceLock<Arc<AtomicBool>> = OnceLock::new();

/// Инициализирует Sentry для release-сборок, только когда задан DSN и пользователь не отказался от телеметрии.
pub fn initialize(app: &tauri::AppHandle) -> AppResult<()> {
    if cfg!(debug_assertions) {
        return Ok(());
    }

    let Some(dsn) = option_env!("SENTRY_DSN").filter(|value| !value.trim().is_empty()) else {
        return Ok(());
    };
    let Ok(dsn) = sentry::types::Dsn::from_str(dsn) else {
        return Ok(());
    };
    let settings = settings::load_app_settings(app)?;
    let is_enabled = Arc::new(AtomicBool::new(settings.is_telemetry_enabled()));
    let _ = IS_TELEMETRY_ENABLED.set(is_enabled.clone());
    let release = format!("transcriber@{}", env!("CARGO_PKG_VERSION"));
    let environment = if option_env!("VITE_APP_CHANNEL") == Some("canary") {
        "canary"
    } else {
        "production"
    };

    // Процесс живёт до завершения приложения, поэтому guard намеренно сохраняется до конца процесса.
    let guard = sentry::init((
        dsn,
        ClientOptions {
            before_send: Some(Arc::new(move |event| sanitize_event(event, &is_enabled))),
            environment: Some(environment.into()),
            release: Some(release.into()),
            send_default_pii: false,
            ..Default::default()
        },
    ));
    Box::leak(Box::new(guard));

    Ok(())
}

/// Немедленно прекращает отправку событий после изменения пользовательской настройки.
pub fn set_enabled(is_enabled: bool) {
    if let Some(enabled) = IS_TELEMETRY_ENABLED.get() {
        enabled.store(is_enabled, Ordering::Relaxed);
    }
}

/// Временно воспроизводит panic при загрузке обязательного файла состояния для проверки Rust telemetry.
#[tauri::command]
pub fn trigger_rust_telemetry_failure(app: tauri::AppHandle) {
    let state_path = app
        .path()
        .app_data_dir()
        .expect("application data directory must be available")
        .join("active-import-state.json");

    // Состояние активного импорта должно существовать, пока код пытается
    // восстановить его после перезапуска. Здесь файл намеренно отсутствует,
    // чтобы проверить захват настоящей ошибки обработки обязательного ресурса.
    fs::read_to_string(&state_path).expect("active import state must be readable");
}

fn sanitize_event(mut event: Event<'static>, is_enabled: &AtomicBool) -> Option<Event<'static>> {
    if !is_enabled.load(Ordering::Relaxed) {
        return None;
    }

    event.breadcrumbs.values.clear();
    event.contexts.clear();
    event.extra.clear();
    event.fingerprint = Default::default();
    event.message = None;
    event.request = None;
    event.tags.clear();
    event.transaction = None;
    event.user = None;

    for exception in &mut event.exception.values {
        exception.value = Some("Unhandled application error".to_owned());
    }

    Some(event)
}
