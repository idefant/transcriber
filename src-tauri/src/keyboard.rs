use crate::error::{AppError, AppResult};

#[cfg(target_os = "windows")]
pub async fn paste_text(text: &str) -> AppResult<()> {
    let previous = read_clipboard_text();
    copy_text(text)?;
    let send_result = send_ctrl_v();
    // Give the target application time to process the paste before the
    // clipboard is restored. SendInput is asynchronous from the recipient's
    // perspective, so a brief pause is required.
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    restore_clipboard(previous);
    send_result
}

#[cfg(not(target_os = "windows"))]
pub async fn paste_text(_text: &str) -> AppResult<()> {
    Err(AppError::from(
        "Dictation paste is only implemented on Windows in this version",
    ))
}

#[cfg(target_os = "windows")]
pub fn copy_text(text: &str) -> AppResult<()> {
    use std::ptr;

    use windows_sys::Win32::System::{
        DataExchange::{
            CloseClipboard, EmptyClipboard, OpenClipboard, RegisterClipboardFormatW,
            SetClipboardData,
        },
        Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE},
    };

    const CF_UNICODETEXT: u32 = 13;

    // Encode text as UTF-16 with null terminator.
    let mut utf16: Vec<u16> = text.encode_utf16().collect();
    utf16.push(0);
    let bytes_len = utf16.len() * size_of::<u16>();

    // Helper to encode a format name as null-terminated UTF-16.
    let encode_format_name = |name: &str| -> Vec<u16> {
        let mut v: Vec<u16> = name.encode_utf16().collect();
        v.push(0);
        v
    };

    // These Windows clipboard formats, when present, instruct the clipboard
    // monitor (Win+V history, cloud sync) to exclude this entry from tracking.
    let exclude_format_name = encode_format_name("ExcludeClipboardContentFromMonitorProcessing");
    let no_history_format_name = encode_format_name("CanIncludeInClipboardHistory");

    unsafe {
        if OpenClipboard(ptr::null_mut()) == 0 {
            return Err(AppError::from("Could not open Windows clipboard"));
        }

        let handle = GlobalAlloc(GMEM_MOVEABLE, bytes_len);

        if handle.is_null() {
            CloseClipboard();
            return Err(AppError::from(
                "Could not allocate Windows clipboard memory",
            ));
        }

        let target = GlobalLock(handle) as *mut u8;

        if target.is_null() {
            CloseClipboard();
            return Err(AppError::from("Could not lock Windows clipboard memory"));
        }

        ptr::copy_nonoverlapping(utf16.as_ptr() as *const u8, target, bytes_len);
        GlobalUnlock(handle);
        EmptyClipboard();

        if SetClipboardData(CF_UNICODETEXT, handle).is_null() {
            CloseClipboard();
            return Err(AppError::from("Could not set Windows clipboard data"));
        }

        // Exclude from clipboard history (Win+V) and cloud sync.
        // "ExcludeClipboardContentFromMonitorProcessing": presence of the format
        // is enough — no data payload needed.
        let exclude_fmt = RegisterClipboardFormatW(exclude_format_name.as_ptr());
        if exclude_fmt != 0 {
            SetClipboardData(exclude_fmt, ptr::null_mut());
        }

        // "CanIncludeInClipboardHistory": set DWORD 0 to opt out.
        let no_history_fmt = RegisterClipboardFormatW(no_history_format_name.as_ptr());
        if no_history_fmt != 0 {
            let dword_handle = GlobalAlloc(GMEM_MOVEABLE, size_of::<u32>());
            if !dword_handle.is_null() {
                let dword_ptr = GlobalLock(dword_handle) as *mut u32;
                if !dword_ptr.is_null() {
                    *dword_ptr = 0;
                    GlobalUnlock(dword_handle);
                    SetClipboardData(no_history_fmt, dword_handle);
                }
            }
        }

        CloseClipboard();
    }

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn copy_text(_text: &str) -> AppResult<()> {
    Err(AppError::from(
        "Clipboard copy is only implemented on Windows in this version",
    ))
}

/// Reads the current clipboard text (CF_UNICODETEXT). Returns `None` if the
/// clipboard is empty, has no text, or cannot be opened.
#[cfg(target_os = "windows")]
fn read_clipboard_text() -> Option<String> {
    use std::ptr;

    use windows_sys::Win32::System::{
        DataExchange::{CloseClipboard, GetClipboardData, OpenClipboard},
        Memory::{GlobalLock, GlobalUnlock},
    };

    const CF_UNICODETEXT: u32 = 13;

    unsafe {
        if OpenClipboard(ptr::null_mut()) == 0 {
            return None;
        }

        let handle = GetClipboardData(CF_UNICODETEXT);
        if handle.is_null() {
            CloseClipboard();
            return None;
        }

        let ptr = GlobalLock(handle) as *const u16;
        if ptr.is_null() {
            CloseClipboard();
            return None;
        }

        // Read null-terminated UTF-16 string.
        let mut len = 0;
        while *ptr.add(len) != 0 {
            len += 1;
        }

        let text = if len > 0 {
            let slice = std::slice::from_raw_parts(ptr, len);
            Some(String::from_utf16_lossy(slice).to_owned())
        } else {
            None
        };

        GlobalUnlock(handle);
        CloseClipboard();

        text
    }
}

/// Restores the clipboard to its pre-paste state. When the previous contents
/// were text, they are written back (also excluded from history to avoid a
/// duplicate entry). When the previous contents were non-text or the clipboard
/// was empty, the clipboard is cleared.
#[cfg(target_os = "windows")]
fn restore_clipboard(previous: Option<String>) {
    match previous {
        Some(text) => {
            let _ = copy_text(&text);
        }
        None => clear_clipboard(),
    }
}

#[cfg(target_os = "windows")]
fn clear_clipboard() {
    use std::ptr;

    use windows_sys::Win32::System::DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard};

    unsafe {
        if OpenClipboard(ptr::null_mut()) != 0 {
            EmptyClipboard();
            CloseClipboard();
        }
    }
}

#[cfg(target_os = "windows")]
fn send_ctrl_v() -> AppResult<()> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, KEYEVENTF_KEYUP, VK_CONTROL, VK_V,
    };

    let mut inputs = [
        keyboard_input(VK_CONTROL, 0),
        keyboard_input(VK_V, 0),
        keyboard_input(VK_V, KEYEVENTF_KEYUP),
        keyboard_input(VK_CONTROL, KEYEVENTF_KEYUP),
    ];

    let sent = unsafe {
        SendInput(
            inputs.len() as u32,
            inputs.as_mut_ptr(),
            size_of::<INPUT>() as i32,
        )
    };

    if sent != inputs.len() as u32 {
        return Err(AppError::from("Could not send Ctrl+V input"));
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn keyboard_input(key: u16, flags: u32) -> windows_sys::Win32::UI::Input::KeyboardAndMouse::INPUT {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT,
    };

    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: key,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}
