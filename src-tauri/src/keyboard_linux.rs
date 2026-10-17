//! Вставка Wayland с сохранением каждого предложенного MIME-формата.

use crate::error::{AppError, AppResult};
use std::{
    io::{Read, Write},
    os::{
        fd::AsRawFd,
        unix::{fs::OpenOptionsExt, process::CommandExt},
    },
    process::{Command, Stdio},
    sync::Mutex,
    time::{Duration, Instant},
};
use wl_clipboard_rs::{
    copy::{self, MimeSource, MimeType, Options, PreparedCopy, Source},
    paste,
};

static CLIPBOARD_OPERATION: Mutex<()> = Mutex::new(());
const MAX_SNAPSHOT_BYTES: usize = 64 * 1024 * 1024;

/** Выполняет блокирующие Wayland-операции вне асинхронного executor. */
pub async fn paste_text(_app: &tauri::AppHandle, text: &str) -> AppResult<()> {
    let text = text.to_owned();
    tauri::async_runtime::spawn_blocking(move || {
        let _lock = CLIPBOARD_OPERATION
            .lock()
            .map_err(|e| AppError::from(e.to_string()))?;
        start_owner("paste", &text)
    })
    .await
    .map_err(|error| AppError::from(error.to_string()))?
}

/** Явное копирование намеренно заменяет содержимое буфера и доступно менеджеру истории. */
pub fn copy_text(_app: &tauri::AppHandle, text: &str) -> AppResult<()> {
    let _lock = CLIPBOARD_OPERATION
        .lock()
        .map_err(|e| AppError::from(e.to_string()))?;
    start_owner("copy", text)
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
        if mime.starts_with("application/x-transcriber-owner-") {
            continue;
        }
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

fn paste_blocking(text: &str) -> AppResult<ClipboardOwner> {
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
    let mut previous = read_snapshot()?;
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
    let mut restored = None;
    let owner_marker = owner_marker();
    if still_ours {
        if previous.is_empty() {
            copy::clear(copy::ClipboardType::Regular, copy::Seat::All).map_err(clipboard_error)?;
        } else {
            let mut options = Options::new();
            options
                .sensitive(true)
                .omit_additional_text_mime_types(true);
            options.foreground(true);
            previous.push(marker_source(&owner_marker));
            restored = Some(
                options
                    .prepare_copy_multi(previous)
                    .map_err(clipboard_error)?,
            );
        }
    }
    let result = match sent {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => Err(AppError::from(format!(
            "wtype failed ({status}); transcription remains in history"
        ))),
        Err(error) => Err(error.into()),
    };
    // Даже отказ синтетического ввода не должен лишить владельца восстановленных данных.
    Ok(ClipboardOwner {
        prepared: restored,
        marker: owner_marker,
        result,
    })
}

fn acknowledge(result: &AppResult<()>) {
    let response = match result {
        Ok(()) => "ok\n".to_owned(),
        Err(error) => format!("error {}\n", error.to_string().replace(['\n', '\r'], " ")),
    };
    let _ = std::io::stdout().write_all(response.as_bytes());
    let _ = std::io::stdout().flush();
}

/** Обслуживает буфер независимо от GUI и завершает работу после смены владельца. */
pub fn run_clipboard_owner() -> bool {
    let mode = std::env::args().nth(1);
    if mode.as_deref() != Some("--clipboard-owner") {
        return false;
    }
    let result = (|| -> AppResult<(ClipboardOwner, std::fs::File)> {
        let mut input = std::io::stdin().lock();
        let mut header = [0u8; 8];
        input.read_exact(&mut header)?;
        let length = u64::from_le_bytes(header);
        if length > MAX_SNAPSHOT_BYTES as u64 {
            return Err(clipboard_error("payload exceeds 64 MiB"));
        }
        let mut text = Vec::new();
        input.take(length).read_to_end(&mut text)?;
        if text.len() as u64 != length {
            return Err(clipboard_error("incomplete helper payload"));
        }
        let text = String::from_utf8(text).map_err(clipboard_error)?;
        let lock = transaction_lock()?;
        let owner = if std::env::args().nth(2).as_deref() == Some("paste") {
            paste_blocking(&text)?
        } else {
            let marker = owner_marker();
            let mut options = Options::new();
            options.foreground(true);
            let prepared = options
                .prepare_copy_multi(vec![
                    MimeSource {
                        source: Source::Bytes(text.into_bytes().into_boxed_slice()),
                        mime_type: MimeType::Text,
                    },
                    marker_source(&marker),
                ])
                .map_err(clipboard_error)?;
            ClipboardOwner {
                prepared: Some(prepared),
                marker,
                result: Ok(()),
            }
        };
        Ok((owner, lock))
    })();
    match result {
        Ok((owner, lock)) => serve_owner(owner, lock),
        Err(error) => acknowledge(&Err(error)),
    }
    true
}

struct ClipboardOwner {
    prepared: Option<PreparedCopy>,
    marker: String,
    result: AppResult<()>,
}

fn owner_marker() -> String {
    format!("application/x-transcriber-owner-{}", uuid::Uuid::new_v4())
}

fn marker_source(marker: &str) -> MimeSource {
    MimeSource {
        source: Source::Bytes(Box::new([])),
        mime_type: MimeType::Specific(marker.to_owned()),
    }
}

fn transaction_lock() -> AppResult<std::fs::File> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .ok_or_else(|| clipboard_error("XDG_RUNTIME_DIR is required"))?;
    let lock = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(std::path::PathBuf::from(runtime).join("transcriber-clipboard.lock"))?;
    let deadline = Instant::now() + Duration::from_secs(8);
    loop {
        if unsafe { libc::flock(lock.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
            return Ok(lock);
        }
        let error = std::io::Error::last_os_error();
        if error.kind() != std::io::ErrorKind::WouldBlock {
            return Err(error.into());
        }
        if Instant::now() >= deadline {
            return Err(clipboard_error("clipboard transaction lock timed out"));
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn serve_owner(owner: ClipboardOwner, lock: std::fs::File) {
    let Some(prepared) = owner.prepared else {
        acknowledge(&owner.result);
        return;
    };
    let (finished, receiver) = std::sync::mpsc::channel::<AppResult<()>>();
    // PreparedCopy не Send. Главный поток обслуживает Wayland; отдельный подтверждает
    // публикацию, прежде чем разрешить следующую транзакцию другого GUI/helper.
    let acknowledgement = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(8);
        loop {
            if paste::get_mime_types(paste::ClipboardType::Regular, paste::Seat::Unspecified)
                .is_ok_and(|types| types.contains(&owner.marker))
            {
                acknowledge(&owner.result);
                break;
            }
            if let Ok(result) = receiver.try_recv() {
                acknowledge(&result.and(owner.result));
                break;
            }
            if Instant::now() >= deadline {
                acknowledge(&Err(clipboard_error("clipboard publication timed out")));
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        drop(lock);
    });
    let result = prepared.serve().map_err(clipboard_error);
    let _ = finished.send(result);
    let _ = acknowledgement.join();
}

fn start_owner(mode: &str, text: &str) -> AppResult<()> {
    if text.len() > MAX_SNAPSHOT_BYTES {
        return Err(clipboard_error("payload exceeds 64 MiB"));
    }
    let mut child = Command::new(std::env::current_exe()?)
        .args(["--clipboard-owner", mode])
        .process_group(0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    let result = (|| -> AppResult<()> {
        let deadline = Instant::now() + Duration::from_secs(8);
        let mut input = child
            .stdin
            .take()
            .ok_or_else(|| clipboard_error("missing helper stdin"))?;
        let mut output = child
            .stdout
            .take()
            .ok_or_else(|| clipboard_error("missing helper stdout"))?;
        for fd in [input.as_raw_fd(), output.as_raw_fd()] {
            let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
            if flags < 0 || unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) } < 0
            {
                return Err(std::io::Error::last_os_error().into());
            }
        }
        let mut payload = Vec::with_capacity(8 + text.len());
        payload.extend_from_slice(&(text.len() as u64).to_le_bytes());
        payload.extend_from_slice(text.as_bytes());
        let mut bytes = payload.as_slice();
        while !bytes.is_empty() {
            match input.write(bytes) {
                Ok(0) => return Err(clipboard_error("helper input closed")),
                Ok(count) => bytes = &bytes[count..],
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(error) => return Err(error.into()),
            }
            if Instant::now() >= deadline {
                return Err(clipboard_error("helper input timed out"));
            }
            if !bytes.is_empty() {
                std::thread::sleep(Duration::from_millis(5));
            }
        }
        drop(input);
        let mut response = Vec::new();
        loop {
            let mut buffer = [0u8; 1024];
            match output.read(&mut buffer) {
                Ok(0) => return Err(clipboard_error("helper exited before acknowledgement")),
                Ok(count) => response.extend_from_slice(&buffer[..count]),
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(error) => return Err(error.into()),
            }
            if response.contains(&b'\n') {
                return if response.starts_with(b"ok\n") {
                    Ok(())
                } else {
                    Err(clipboard_error(String::from_utf8_lossy(&response)))
                };
            }
            if response.len() > 4096 || Instant::now() >= deadline {
                return Err(clipboard_error("helper acknowledgement timed out"));
            }
            std::thread::sleep(Duration::from_millis(5));
        }
    })();
    // GUI может исчезнуть во время вставки: helper всё равно завершает восстановление.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    result
}
