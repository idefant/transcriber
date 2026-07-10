use crate::{
    error::{AppError, AppResult},
    i18n,
};

#[cfg(target_os = "windows")]
pub async fn paste_text(app: &tauri::AppHandle, text: &str) -> AppResult<()> {
    let previous = read_clipboard_snapshot();
    write_clipboard(&[text_entry(text)], ClipboardHistoryMode::Hidden)
        .map_err(|error| error.into_app_error(app))?;
    let send_result = send_ctrl_v(app);
    // Give the target application time to process the paste before the
    // clipboard is restored. SendInput is asynchronous from the recipient's
    // perspective, so a brief pause is required.
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    restore_clipboard(previous);
    send_result
}

#[cfg(not(target_os = "windows"))]
pub async fn paste_text(app: &tauri::AppHandle, _text: &str) -> AppResult<()> {
    Err(AppError::from(i18n::text(
        app,
        "clipboard-paste-windows-only",
    )))
}

#[cfg(target_os = "windows")]
pub fn copy_text(app: &tauri::AppHandle, text: &str) -> AppResult<()> {
    write_clipboard(&[text_entry(text)], ClipboardHistoryMode::Visible)
        .map_err(|error| error.into_app_error(app))
}

#[cfg(not(target_os = "windows"))]
pub fn copy_text(app: &tauri::AppHandle, _text: &str) -> AppResult<()> {
    Err(AppError::from(i18n::text(
        app,
        "clipboard-copy-windows-only",
    )))
}

/// Windows clipboard formats that tell the clipboard monitor (Win+V history,
/// cloud sync) to ignore the current entry.
#[cfg(target_os = "windows")]
const EXCLUDE_FROM_MONITOR_FORMAT: &str = "ExcludeClipboardContentFromMonitorProcessing";

#[cfg(target_os = "windows")]
const CAN_INCLUDE_IN_HISTORY_FORMAT: &str = "CanIncludeInClipboardHistory";

#[cfg(target_os = "windows")]
enum ClipboardHistoryMode {
    Hidden,
    Visible,
}

/// One clipboard format and its raw bytes. `data` is `None` when the format was
/// present with a null handle, which is how marker formats carry meaning through
/// their presence alone.
#[cfg(target_os = "windows")]
struct ClipboardEntry {
    format: u32,
    data: Option<Vec<u8>>,
}

#[cfg(target_os = "windows")]
enum ClipboardError {
    Open,
    Write,
}

#[cfg(target_os = "windows")]
impl ClipboardError {
    fn into_app_error(self, app: &tauri::AppHandle) -> AppError {
        let key = match self {
            Self::Open => "clipboard-open-failed",
            Self::Write => "clipboard-set-data-failed",
        };

        AppError::from(i18n::text(app, key))
    }
}

#[cfg(target_os = "windows")]
fn text_entry(text: &str) -> ClipboardEntry {
    let mut data = Vec::with_capacity((text.len() + 1) * size_of::<u16>());

    for unit in text.encode_utf16().chain(std::iter::once(0)) {
        data.extend_from_slice(&unit.to_ne_bytes());
    }

    ClipboardEntry {
        format: clipboard_win::formats::CF_UNICODETEXT,
        data: Some(data),
    }
}

/// Opens the clipboard, retrying because another application can hold it open
/// while it processes a paste.
///
/// `Clipboard::new_attempts` only yields the scheduler slice between tries, which
/// is too short to outlast a busy paste target, so the waiting is done here.
#[cfg(target_os = "windows")]
fn open_clipboard() -> Option<clipboard_win::Clipboard> {
    for attempt in 0..5u32 {
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_millis(30));
        }

        if let Ok(clipboard) = clipboard_win::Clipboard::new() {
            return Some(clipboard);
        }
    }

    None
}

/// Copies every restorable clipboard format into memory so the clipboard can be
/// put back exactly as it was. Returns `None` when the clipboard could not be
/// opened at all; the caller must then leave the clipboard untouched rather than
/// destroy contents it failed to read.
#[cfg(target_os = "windows")]
fn read_clipboard_snapshot() -> Option<Vec<ClipboardEntry>> {
    use clipboard_win::{formats, raw};

    let _clipboard = open_clipboard()?;

    // Enumerate every format first and read the data afterwards. `GetClipboardData`
    // can trigger delayed rendering in the owning application, which writes to the
    // clipboard and would invalidate an in-flight enumeration.
    let available: Vec<u32> = raw::EnumFormats::new().collect();

    // The markers are re-applied on restore, so carrying them through the snapshot
    // would only set the same formats twice.
    let markers = [
        registered_format(EXCLUDE_FROM_MONITOR_FORMAT),
        registered_format(CAN_INCLUDE_IN_HISTORY_FORMAT),
    ];

    // An image is always enumerated as both CF_DIB and CF_DIBV5, whichever one the
    // source placed, and reading either costs a full-size conversion in Windows —
    // around 50 ms apiece for a 4K screenshot. Keep CF_DIB and let Windows
    // synthesize the rest of the image formats back from it.
    //
    // CF_DIB is the safe half of the pair. Its BITMAPINFOHEADER is always followed
    // by the three BI_BITFIELDS masks, so the pixel offset is unambiguous. A
    // BITMAPV5HEADER already carries those masks inside the header, yet the buffer
    // Windows synthesizes still appends them; writing those bytes back as a native
    // CF_DIBV5 makes readers treat the 12 mask bytes as pixel data and shifts the
    // whole image three pixels sideways.
    let has_dib = available.contains(&formats::CF_DIB);

    let entries = available
        .into_iter()
        .filter(|format| {
            is_restorable_format(*format)
                && !markers.contains(format)
                && !(has_dib && *format == formats::CF_DIBV5)
        })
        .map(|format| ClipboardEntry {
            format,
            data: read_clipboard_format(format),
        })
        .collect();

    Some(entries)
}

/// Restores a snapshot taken by [`read_clipboard_snapshot`]. An empty snapshot
/// restores an empty clipboard. The restored entry is marked as hidden so it does
/// not create a duplicate Win+V history entry next to the one the original copy
/// already produced.
#[cfg(target_os = "windows")]
fn restore_clipboard(previous: Option<Vec<ClipboardEntry>>) {
    let Some(entries) = previous else {
        return;
    };

    let has_image = entries
        .iter()
        .any(|entry| entry.format == clipboard_win::formats::CF_DIB);

    if write_clipboard(&entries, ClipboardHistoryMode::Hidden).is_err() {
        return;
    }

    if has_image {
        force_bitmap_synthesis();
    }
}

/// Reads one format as a raw memory block. Returns `None` for formats with a null
/// handle and for handles the memory manager does not recognise as an `HGLOBAL`.
///
/// The clipboard must be open.
#[cfg(target_os = "windows")]
fn read_clipboard_format(format: u32) -> Option<Vec<u8>> {
    let mut data = Vec::new();

    match clipboard_win::raw::get_vec(format, &mut data) {
        Ok(_) if !data.is_empty() => Some(data),
        _ => None,
    }
}

/// Formats that cannot be copied byte for byte: GDI handles, metafiles,
/// owner-drawn display formats, and the private ranges whose memory the system
/// does not manage.
///
/// Images still survive a snapshot. Windows enumerates `CF_DIB`/`CF_DIBV5`
/// alongside `CF_BITMAP` and synthesizes the bitmap and palette handles back from
/// the DIB block that is restored here.
#[cfg(target_os = "windows")]
fn is_restorable_format(format: u32) -> bool {
    use clipboard_win::formats::{
        CF_BITMAP, CF_DSPBITMAP, CF_DSPENHMETAFILE, CF_DSPMETAFILEPICT, CF_ENHMETAFILE,
        CF_GDIOBJFIRST, CF_GDIOBJLAST, CF_METAFILEPICT, CF_OWNERDISPLAY, CF_PALETTE,
        CF_PRIVATEFIRST, CF_PRIVATELAST,
    };

    let handle_based = matches!(
        format,
        CF_BITMAP
            | CF_METAFILEPICT
            | CF_PALETTE
            | CF_ENHMETAFILE
            | CF_OWNERDISPLAY
            | CF_DSPBITMAP
            | CF_DSPMETAFILEPICT
            | CF_DSPENHMETAFILE
    );

    !handle_based
        && !(CF_PRIVATEFIRST..=CF_PRIVATELAST).contains(&format)
        && !(CF_GDIOBJFIRST..=CF_GDIOBJLAST).contains(&format)
}

#[cfg(target_os = "windows")]
fn write_clipboard(
    entries: &[ClipboardEntry],
    history_mode: ClipboardHistoryMode,
) -> Result<(), ClipboardError> {
    use clipboard_win::raw;

    let _clipboard = open_clipboard().ok_or(ClipboardError::Open)?;

    raw::empty().map_err(|_| ClipboardError::Write)?;

    for entry in entries {
        match &entry.data {
            Some(data) => {
                raw::set_without_clear(entry.format, data).map_err(|_| ClipboardError::Write)?
            }
            None => set_empty_format(entry.format),
        }
    }

    if matches!(history_mode, ClipboardHistoryMode::Hidden) && !entries.is_empty() {
        exclude_from_clipboard_history();
    }

    Ok(())
}

/// Opts the current clipboard entry out of the Windows clipboard monitor.
///
/// The clipboard must be open.
#[cfg(target_os = "windows")]
fn exclude_from_clipboard_history() {
    use clipboard_win::raw;

    // "ExcludeClipboardContentFromMonitorProcessing": presence of the format is
    // enough, no data payload is required.
    let exclude_format = registered_format(EXCLUDE_FROM_MONITOR_FORMAT);

    if exclude_format != 0 {
        set_empty_format(exclude_format);
    }

    // "CanIncludeInClipboardHistory": set DWORD 0 to opt out.
    let no_history_format = registered_format(CAN_INCLUDE_IN_HISTORY_FORMAT);

    if no_history_format != 0 {
        let _ = raw::set_without_clear(no_history_format, &0u32.to_ne_bytes());
    }
}

/// Materializes CF_BITMAP from the CF_DIB that was just restored.
///
/// The original clipboard usually carried a real `HBITMAP`, which cannot be copied
/// into a snapshot, so CF_BITMAP now has to be synthesized. Windows derives it from
/// whichever DIB format is present, and its CF_DIBV5 path treats the three
/// BI_BITFIELDS masks as pixel data, which shifts the image three pixels sideways.
/// Reading CF_BITMAP here forces the correct CF_DIB-derived handle and caches it,
/// before a paste target can materialize CF_DIBV5 and poison the conversion.
#[cfg(target_os = "windows")]
fn force_bitmap_synthesis() {
    use clipboard_win::{formats, raw};

    let Some(_clipboard) = open_clipboard() else {
        return;
    };

    let _ = raw::get_clipboard_data(formats::CF_BITMAP);
}

/// Returns the id of a registered clipboard format, or `0` when registration fails.
#[cfg(target_os = "windows")]
fn registered_format(name: &str) -> u32 {
    clipboard_win::raw::register_format(name).map_or(0, |format| format.get())
}

/// Places a format on the clipboard with a null handle, marking it as present with
/// no payload.
///
/// This stays on the raw Win32 call because `clipboard_win::raw::set_without_clear`
/// silently succeeds without writing anything when the data slice is empty.
///
/// The clipboard must be open.
#[cfg(target_os = "windows")]
fn set_empty_format(format: u32) {
    use windows_sys::Win32::System::DataExchange::SetClipboardData;

    unsafe {
        SetClipboardData(format, std::ptr::null_mut());
    }
}

#[cfg(target_os = "windows")]
fn send_ctrl_v(app: &tauri::AppHandle) -> AppResult<()> {
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
        return Err(AppError::from(i18n::text(
            app,
            "clipboard-send-ctrl-v-failed",
        )));
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
