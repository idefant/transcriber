use std::str::FromStr;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, OnceLock,
};

use sentry::{
    integrations::panic::PanicIntegration,
    protocol::{DebugImage, Event, Exception, Frame, IpAddress, Mechanism, Stacktrace, User},
    ClientOptions,
};

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
            default_integrations: false,
            environment: Some(environment.into()),
            release: Some(release.into()),
            send_default_pii: false,
            ..Default::default()
        }
        .add_integration(sentry::integrations::debug_images::DebugImagesIntegration::default())
        .add_integration(sentry::integrations::backtrace::ProcessStacktraceIntegration)
        .add_integration(PanicIntegration::new().add_extractor(|info| {
            // Штатный extractor сохраняет стек в момент паники; location не зависит от символизации.
            let mut event = PanicIntegration::new().event_from_panic_info(info);
            if let Some(location) = info.location() {
                if let Some(filename) = sanitize_source_path(location.file()) {
                    event.tags.insert("panic.file".into(), filename);
                    event
                        .tags
                        .insert("panic.line".into(), location.line().to_string());
                    event
                        .tags
                        .insert("panic.column".into(), location.column().to_string());
                }
            }
            Some(event)
        })),
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

fn sanitize_source_path(path: &str) -> Option<String> {
    let path = path.replace('\\', "/");
    // Оставляем только часть пути внутри исходников, без домашней папки и пути registry.
    let source = path
        .strip_prefix("src/")
        .or_else(|| path.rsplit_once("/src/").map(|(_, source)| source))?;
    if !source.ends_with(".rs")
        || source
            .split('/')
            .any(|part| part.is_empty() || part == "..")
        || !source
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '/' | '.'))
    {
        return None;
    }
    Some(format!("src/{source}"))
}

fn application_binary(path: &str) -> Option<&'static str> {
    match path.rsplit(['/', '\\']).next()? {
        "transcriber" => Some("transcriber"),
        "transcriber.exe" => Some("transcriber.exe"),
        "transcriber_lib.dll" => Some("transcriber_lib.dll"),
        "transcriber_lib.pdb" => Some("transcriber_lib.pdb"),
        "transcriber.pdb" => Some("transcriber.pdb"),
        "libtranscriber_lib.so" => Some("libtranscriber_lib.so"),
        "libtranscriber_lib.dylib" => Some("libtranscriber_lib.dylib"),
        _ => None,
    }
}

fn sanitize_stacktrace(stacktrace: Stacktrace) -> Stacktrace {
    Stacktrace {
        frames: stacktrace
            .frames
            .into_iter()
            .map(|frame| Frame {
                // Символы берутся из бинарника, но имена модулей и пути к библиотекам не передаются.
                function: frame.function.filter(|name| {
                    name.len() <= 512
                        && name.chars().all(|ch| {
                            ch.is_ascii_alphanumeric()
                                || matches!(
                                    ch,
                                    '_' | ':'
                                        | '<'
                                        | '>'
                                        | ','
                                        | ' '
                                        | '&'
                                        | '['
                                        | ']'
                                        | '('
                                        | ')'
                                        | '{'
                                        | '}'
                                        | '#'
                                        | '$'
                                        | '.'
                                )
                        })
                }),
                filename: frame
                    .filename
                    .as_deref()
                    .or(frame.abs_path.as_deref())
                    .and_then(sanitize_source_path),
                lineno: frame.lineno,
                colno: frame.colno,
                in_app: frame.in_app,
                instruction_addr: frame.instruction_addr,
                image_addr: frame.image_addr,
                symbol_addr: frame.symbol_addr,
                ..Default::default()
            })
            .collect(),
        ..Default::default()
    }
}

fn classify_panic(message: &str) -> &'static str {
    if message.starts_with("called `Result::unwrap()` on an `Err` value:") {
        "result_unwrap_failed"
    } else if message == "called `Option::unwrap()` on a `None` value" {
        "option_unwrap_failed"
    } else if message.starts_with("index out of bounds:") {
        "index_out_of_bounds"
    } else if message.starts_with("assertion `left == right` failed")
        || message.starts_with("assertion `left != right` failed")
        || message.starts_with("assertion failed:")
    {
        "assertion_failed"
    } else if message == "already borrowed: BorrowMutError"
        || message == "already mutably borrowed: BorrowError"
    {
        "borrow_conflict"
    } else if message == "attempt to divide by zero" {
        "division_by_zero"
    } else if message.starts_with("Failed to setup app:") {
        "tauri_setup_failed"
    } else {
        "unclassified"
    }
}

fn sanitize_event(event: Event<'static>, is_enabled: &AtomicBool) -> Option<Event<'static>> {
    if !is_enabled.load(Ordering::Relaxed) {
        return None;
    }

    let mut sanitized = Event {
        event_id: event.event_id,
        timestamp: event.timestamp,
        level: event.level,
        platform: "native".into(),
        // Не позволяем серверу дополнять отчёт IP и географией транспортного запроса.
        user: Some(User {
            ip_address: Some(IpAddress::from(std::net::IpAddr::V4(
                std::net::Ipv4Addr::UNSPECIFIED,
            ))),
            ..Default::default()
        }),
        release: Some(format!("transcriber@{}", env!("CARGO_PKG_VERSION")).into()),
        environment: Some(
            if option_env!("VITE_APP_CHANNEL") == Some("canary") {
                "canary"
            } else {
                "production"
            }
            .into(),
        ),
        ..Default::default()
    };
    sanitized
        .tags
        .insert("diagnostic.schema".into(), "2".into());
    if let Some(filename) = event
        .tags
        .get("panic.file")
        .and_then(|s| sanitize_source_path(s))
    {
        sanitized.tags.insert("panic.file".into(), filename);
        for key in ["panic.line", "panic.column"] {
            if let Some(number) = event.tags.get(key).and_then(|s| s.parse::<u32>().ok()) {
                sanitized.tags.insert(key.into(), number.to_string());
            }
        }
    }
    sanitized.exception = event
        .exception
        .values
        .into_iter()
        .map(|exception| {
            let is_panic = exception.ty == "panic";
            Exception {
                ty: if is_panic { "panic" } else { "Error" }.into(),
                value: Some(
                    if is_panic {
                        classify_panic(exception.value.as_deref().unwrap_or_default())
                    } else {
                        "unclassified"
                    }
                    .into(),
                ),
                stacktrace: exception.stacktrace.map(sanitize_stacktrace),
                mechanism: is_panic.then(|| Mechanism {
                    ty: "panic".into(),
                    handled: Some(false),
                    ..Default::default()
                }),
                ..Default::default()
            }
        })
        .collect::<Vec<_>>()
        .into();
    // Для загруженных символов нужны адреса и debug ID, но не локальные пути образов.
    sanitized.debug_meta.to_mut().images = event
        .debug_meta
        .images
        .iter()
        .filter_map(|image| match image {
            DebugImage::Symbolic(image) => {
                let name = application_binary(&image.name)?;
                let mut image = image.clone();
                image.name = name.into();
                image.debug_file = image
                    .debug_file
                    .as_deref()
                    .and_then(application_binary)
                    .map(Into::into);
                Some(DebugImage::Symbolic(image))
            }
            _ => None,
        })
        .collect();
    Some(sanitized)
}
