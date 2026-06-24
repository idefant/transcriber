use crate::error::{AppError, AppResult};

pub const HIDDEN_START_ARG: &str = "--hidden";

#[cfg(target_os = "windows")]
const RUN_KEY_PATH: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
#[cfg(target_os = "windows")]
const RUN_VALUE_NAME: &str = if cfg!(debug_assertions) {
    "Transcriber DEV"
} else {
    "Transcriber"
};

pub fn sync_launch_at_login(is_enabled: bool) -> AppResult<()> {
    if is_enabled {
        enable_launch_at_login()
    } else {
        disable_launch_at_login()
    }
}

#[cfg(target_os = "windows")]
fn enable_launch_at_login() -> AppResult<()> {
    use std::{mem, ptr};

    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegSetValueExW, HKEY_CURRENT_USER, KEY_SET_VALUE,
        REG_OPTION_NON_VOLATILE, REG_SZ,
    };

    let exe_path = std::env::current_exe()?;
    let command = format!("\"{}\" {HIDDEN_START_ARG}", exe_path.to_string_lossy());
    let key_path = wide_null(RUN_KEY_PATH);
    let value_name = wide_null(RUN_VALUE_NAME);
    let value = wide_null(&command);
    let mut key = ptr::null_mut();

    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            key_path.as_ptr(),
            0,
            ptr::null_mut(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            ptr::null(),
            &mut key,
            ptr::null_mut(),
        )
    };

    if status != 0 {
        return Err(AppError::from(format!(
            "Could not open Windows startup registry key: {status}",
        )));
    }

    let status = unsafe {
        RegSetValueExW(
            key,
            value_name.as_ptr(),
            0,
            REG_SZ,
            value.as_ptr() as *const u8,
            (value.len() * mem::size_of::<u16>()) as u32,
        )
    };

    unsafe {
        RegCloseKey(key);
    }

    if status != 0 {
        return Err(AppError::from(format!(
            "Could not set Windows startup registry value: {status}",
        )));
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn disable_launch_at_login() -> AppResult<()> {
    use std::ptr;

    use windows_sys::Win32::System::Registry::{
        RegCloseKey, RegCreateKeyExW, RegDeleteValueW, HKEY_CURRENT_USER, KEY_SET_VALUE,
        REG_OPTION_NON_VOLATILE,
    };

    let key_path = wide_null(RUN_KEY_PATH);
    let value_name = wide_null(RUN_VALUE_NAME);
    let mut key = ptr::null_mut();

    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            key_path.as_ptr(),
            0,
            ptr::null_mut(),
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            ptr::null(),
            &mut key,
            ptr::null_mut(),
        )
    };

    if status != 0 {
        return Err(AppError::from(format!(
            "Could not open Windows startup registry key: {status}",
        )));
    }

    let status = unsafe { RegDeleteValueW(key, value_name.as_ptr()) };

    unsafe {
        RegCloseKey(key);
    }

    if status != 0 && status != 2 {
        return Err(AppError::from(format!(
            "Could not delete Windows startup registry value: {status}",
        )));
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(not(target_os = "windows"))]
fn enable_launch_at_login() -> AppResult<()> {
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn disable_launch_at_login() -> AppResult<()> {
    Ok(())
}
