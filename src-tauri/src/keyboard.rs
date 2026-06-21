use crate::error::{AppError, AppResult};

#[cfg(target_os = "windows")]
pub fn paste_text(text: &str) -> AppResult<()> {
    set_clipboard_text(text)?;
    send_ctrl_v()?;

    Ok(())
}

#[cfg(not(target_os = "windows"))]
pub fn paste_text(_text: &str) -> AppResult<()> {
    Err(AppError::from(
        "Dictation paste is only implemented on Windows in this version",
    ))
}

#[cfg(target_os = "windows")]
fn set_clipboard_text(text: &str) -> AppResult<()> {
    use std::ptr;

    use windows_sys::Win32::System::{
        DataExchange::{CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData},
        Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE},
    };

    const CF_UNICODETEXT: u32 = 13;

    let mut utf16: Vec<u16> = text.encode_utf16().collect();
    utf16.push(0);
    let bytes_len = utf16.len() * size_of::<u16>();

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

        CloseClipboard();
    }

    Ok(())
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
