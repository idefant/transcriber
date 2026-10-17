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

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn enable_launch_at_login() -> AppResult<()> {
    Ok(())
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn disable_launch_at_login() -> AppResult<()> {
    Ok(())
}

#[cfg(target_os = "linux")]
fn autostart_path() -> AppResult<std::path::PathBuf> {
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".config"))
        })
        .ok_or_else(|| AppError::from("Could not locate the user configuration directory"))?;
    let name = if cfg!(debug_assertions) {
        "transcriber-dev.desktop"
    } else {
        "transcriber.desktop"
    };
    Ok(config.join("autostart").join(name))
}

#[cfg(target_os = "linux")]
fn enable_launch_at_login() -> AppResult<()> {
    let path = autostart_path()?;
    let executable = std::env::var_os("APPIMAGE")
        .map(std::path::PathBuf::from)
        .unwrap_or(std::env::current_exe()?);
    let executable = executable.to_string_lossy();
    if executable.contains(['\n', '\r']) {
        return Err(AppError::from("Executable path contains a newline"));
    }
    // Exec сначала проходит разбор .desktop, затем разбор quoted arguments.
    let executable = executable
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('`', "\\`")
        .replace('$', "\\$")
        .replace('%', "%%")
        .replace('\\', "\\\\");
    let contents = format!("[Desktop Entry]\nType=Application\nName=Transcriber\nName[ru]=Transcriber\nGenericName=Voice dictation\nGenericName[ru]=Голосовая диктовка\nKeywords=Transcriber;Voice;Dictation;Transcription;\nKeywords[ru]=Transcriber;Voice dictation;Voice;Dictation;Transcription;голос;диктовка;расшифровка;\nExec=\"{executable}\" {HIDDEN_START_ARG}\nTerminal=false\nStartupNotify=false\n");
    if std::fs::read_to_string(&path).ok().as_deref() != Some(&contents) {
        std::fs::create_dir_all(
            path.parent()
                .ok_or_else(|| AppError::from("Invalid autostart path"))?,
        )?;
        std::fs::write(path, contents)?;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn disable_launch_at_login() -> AppResult<()> {
    match std::fs::remove_file(autostart_path()?) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}
