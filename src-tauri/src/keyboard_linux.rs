//! Вставка Wayland с сохранением каждого предложенного MIME-формата.

use crate::error::{AppError, AppResult};
use std::{
    io::Read,
    os::fd::AsRawFd,
    process::Command,
    sync::Mutex,
    time::{Duration, Instant},
};
use wl_clipboard_rs::{
    copy::{self, MimeSource, MimeType, Options, Source},
    paste,
};

static CLIPBOARD_OPERATION: Mutex<()> = Mutex::new(());
const MAX_SNAPSHOT_BYTES: usize = 64 * 1024 * 1024;

/** Выполняет блокирующие Wayland-операции вне асинхронного executor. */
pub async fn paste_text(_app: &tauri::AppHandle, text: &str) -> AppResult<()> {
    let text = text.to_owned();
    tauri::async_runtime::spawn_blocking(move || paste_blocking(&text))
        .await
        .map_err(|error| AppError::from(error.to_string()))?
}

/** Явное копирование намеренно заменяет содержимое буфера и доступно менеджеру истории. */
pub fn copy_text(_app: &tauri::AppHandle, text: &str) -> AppResult<()> {
    let _lock = CLIPBOARD_OPERATION
        .lock()
        .map_err(|e| AppError::from(e.to_string()))?;
    Options::new()
        .copy(Source::Bytes(text.as_bytes().into()), MimeType::Text)
        .map_err(clipboard_error)
}

fn clipboard_error(error: impl std::fmt::Display) -> AppError {
    AppError::from(format!("Wayland clipboard: {error}"))
}

fn read_snapshot() -> AppResult<Vec<MimeSource>> {
    let types = match paste::get_mime_types(paste::ClipboardType::Regular, paste::Seat::Unspecified)
    {
        Ok(types) => types,
        Err(paste::Error::ClipboardEmpty) => return Ok(Vec::new()),
        Err(error) => return Err(clipboard_error(error)),
    };
    let mut snapshot = Vec::new();
    let mut total = 0;
    let deadline = Instant::now() + Duration::from_secs(3);
    let has_plain_text = types.iter().any(|mime| mime.starts_with("text/plain"));
    for mime in &types {
        // XWayland синтезирует legacy-алиасы из канонического text/plain.
        if has_plain_text
            && ["UTF8_STRING", "TEXT", "STRING", "COMPOUND_TEXT"].contains(&mime.as_str())
        {
            continue;
        }
        // Служебные X11 targets описывают обмен, но не несут восстанавливаемые данные.
        if [
            "TARGETS",
            "SAVE_TARGETS",
            "MULTIPLE",
            "TIMESTAMP",
            "DELETE",
            "INSERT_SELECTION",
            "INSERT_PROPERTY",
            "INCR",
        ]
        .contains(&mime.as_str())
        {
            continue;
        }
        let (mut reader, _) = paste::get_contents(
            paste::ClipboardType::Regular,
            paste::Seat::Unspecified,
            paste::MimeType::Specific(mime),
        )
        .map_err(clipboard_error)?;
        let mut bytes = Vec::new();
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(clipboard_error(
                    "snapshot timed out; existing clipboard was kept",
                ));
            }
            let mut fd = libc::pollfd {
                fd: reader.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            };
            // poll ограничивает ожидание владельца буфера, который может не ответить.
            let result = unsafe {
                libc::poll(
                    &mut fd,
                    1,
                    remaining.as_millis().min(i32::MAX as u128) as i32,
                )
            };
            if result <= 0 {
                return Err(clipboard_error(
                    "snapshot could not be read; existing clipboard was kept",
                ));
            }
            let mut buffer = [0u8; 8192];
            let count = reader.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            total += count;
            if total > MAX_SNAPSHOT_BYTES {
                return Err(clipboard_error(
                    "snapshot exceeds 64 MiB; existing clipboard was kept",
                ));
            }
            bytes.extend_from_slice(&buffer[..count]);
        }
        snapshot.push(MimeSource {
            source: Source::Bytes(bytes.into_boxed_slice()),
            mime_type: MimeType::Specific(mime.clone()),
        });
    }
    let current = paste::get_mime_types(paste::ClipboardType::Regular, paste::Seat::Unspecified)
        .map_err(clipboard_error)?;
    if current != types {
        return Err(clipboard_error(
            "clipboard changed during snapshot; insertion cancelled",
        ));
    }
    Ok(snapshot)
}

fn paste_blocking(text: &str) -> AppResult<()> {
    let _lock = CLIPBOARD_OPERATION
        .lock()
        .map_err(|e| AppError::from(e.to_string()))?;
    // Сначала проверяем фокус: если Hyprland недоступен, буфер не затрагиваем.
    let active = Command::new("hyprctl")
        .args(["activewindow", "-j"])
        .output()?;
    if !active.status.success() {
        return Err(AppError::from("Could not inspect active Hyprland window"));
    }
    let window: serde_json::Value = serde_json::from_slice(&active.stdout)?;
    let class = window["class"]
        .as_str()
        .unwrap_or_default()
        .to_ascii_lowercase();
    if class.is_empty() {
        return Err(AppError::from("No active window to paste into"));
    }
    let terminal = [
        "alacritty",
        "kitty",
        "foot",
        "ghostty",
        "wezterm",
        "org.wezfurlong.wezterm",
        "org.gnome.terminal",
        "konsole",
    ]
    .iter()
    .any(|name| class == *name || class.starts_with(&format!("{name}-")));
    let previous = read_snapshot()?;
    let focused = Command::new("hyprctl")
        .args(["activewindow", "-j"])
        .output()?;
    let focused: serde_json::Value = serde_json::from_slice(&focused.stdout)?;
    if focused["address"] != window["address"] {
        return Err(AppError::from(
            "Active window changed while reading the clipboard; insertion cancelled",
        ));
    }
    let marker = format!(
        "application/x-transcriber-insertion-{}",
        uuid::Uuid::new_v4()
    );
    let mut options = Options::new();
    options.sensitive(true);
    options
        .copy_multi(vec![
            MimeSource {
                source: Source::Bytes(text.as_bytes().into()),
                mime_type: MimeType::Text,
            },
            MimeSource {
                source: Source::Bytes(Box::new([])),
                mime_type: MimeType::Specific(marker.clone()),
            },
        ])
        .map_err(clipboard_error)?;
    let args: &[&str] = if terminal {
        &[
            "-M", "ctrl", "-M", "shift", "-k", "v", "-m", "shift", "-m", "ctrl",
        ]
    } else {
        &["-M", "ctrl", "-k", "v", "-m", "ctrl"]
    };
    let sent = Command::new("wtype").args(args).status();
    std::thread::sleep(Duration::from_millis(500));
    // Не затираем новое пользовательское копирование во время вставки.
    let still_ours =
        match paste::get_mime_types(paste::ClipboardType::Regular, paste::Seat::Unspecified) {
            Ok(types) => types.contains(&marker),
            Err(paste::Error::ClipboardEmpty) => false,
            Err(error) => {
                return Err(clipboard_error(format!(
                    "could not verify clipboard ownership before restore: {error}"
                )))
            }
        };
    if still_ours {
        if previous.is_empty() {
            copy::clear(copy::ClipboardType::Regular, copy::Seat::All).map_err(clipboard_error)?;
        } else {
            let mut options = Options::new();
            options
                .sensitive(true)
                .omit_additional_text_mime_types(true);
            options.copy_multi(previous).map_err(clipboard_error)?;
        }
    }
    match sent {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(AppError::from(format!(
            "wtype failed ({status}); transcription remains in history"
        ))),
        Err(error) => Err(error.into()),
    }
}
