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
    // Даём целевому приложению время обработать вставку, прежде чем буфер
    // обмена будет восстановлен. SendInput асинхронен с точки зрения
    // получателя, поэтому требуется небольшая пауза.
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

/// Форматы буфера обмена Windows, которые указывают монитору буфера обмена
/// (история Win+V, облачная синхронизация) игнорировать текущую запись.
#[cfg(target_os = "windows")]
const EXCLUDE_FROM_MONITOR_FORMAT: &str = "ExcludeClipboardContentFromMonitorProcessing";

#[cfg(target_os = "windows")]
const CAN_INCLUDE_IN_HISTORY_FORMAT: &str = "CanIncludeInClipboardHistory";

#[cfg(target_os = "windows")]
enum ClipboardHistoryMode {
    Hidden,
    Visible,
}

/// Один формат буфера обмена и его сырые байты. `data` равно `None`, когда
/// формат присутствовал с нулевым дескриптором — так формат-маркер несёт
/// смысл самим фактом своего присутствия.
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

/// Открывает буфер обмена, повторяя попытки, потому что другое приложение
/// может удерживать его открытым во время обработки вставки.
///
/// `Clipboard::new_attempts` лишь уступает квант планировщика между
/// попытками, а этого слишком мало, чтобы пережить занятую цель вставки,
/// поэтому ожидание реализовано здесь.
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

/// Копирует в память все восстановимые форматы буфера обмена, чтобы затем
/// вернуть буфер обмена ровно в исходное состояние. Возвращает `None`, если
/// буфер обмена вообще не удалось открыть; в этом случае вызывающий код
/// должен оставить буфер обмена нетронутым, а не уничтожать содержимое,
/// которое не удалось прочитать.
#[cfg(target_os = "windows")]
fn read_clipboard_snapshot() -> Option<Vec<ClipboardEntry>> {
    use clipboard_win::{formats, raw};

    let _clipboard = open_clipboard()?;

    // Сначала перечисляем все форматы, а данные читаем уже после. `GetClipboardData`
    // может вызвать отложенную отрисовку (delayed rendering) в приложении-владельце,
    // которое запишет данные в буфер обмена и тем самым сделает недействительным
    // перечисление, выполняемое в данный момент.
    let available: Vec<u32> = raw::EnumFormats::new().collect();

    // Маркеры заново проставляются при восстановлении, поэтому переносить их
    // через снимок означало бы лишь дважды установить одни и те же форматы.
    let markers = [
        registered_format(EXCLUDE_FROM_MONITOR_FORMAT),
        registered_format(CAN_INCLUDE_IN_HISTORY_FORMAT),
    ];

    // Изображение всегда перечисляется одновременно как CF_DIB и CF_DIBV5 —
    // какой бы из них ни поместил источник, — а чтение любого из них обходится
    // Windows в полноразмерное преобразование: около 50 мс на 4K-скриншот.
    // Оставляем CF_DIB и даём Windows синтезировать остальные форматы
    // изображения обратно из него.
    //
    // CF_DIB — безопасная половина этой пары. За его BITMAPINFOHEADER всегда
    // следуют три маски BI_BITFIELDS, поэтому смещение пикселей однозначно.
    // BITMAPV5HEADER уже несёт эти маски внутри заголовка, но буфер, который
    // синтезирует Windows, всё равно добавляет их же; если записать эти байты
    // обратно как нативный CF_DIBV5, читатели примут 12 байт масок за
    // пиксельные данные и сдвинут всё изображение на три пикселя вбок.
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

/// Восстанавливает снимок, сделанный [`read_clipboard_snapshot`]. Пустой снимок
/// восстанавливает пустой буфер обмена. Восстановленная запись помечается как
/// скрытая, чтобы не создавать дублирующую запись в истории Win+V рядом с той,
/// что уже была создана исходным копированием.
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

/// Читает один формат как сырой блок памяти. Возвращает `None` для форматов
/// с нулевым дескриптором и для дескрипторов, которые менеджер памяти не
/// распознаёт как `HGLOBAL`.
///
/// Буфер обмена должен быть открыт.
#[cfg(target_os = "windows")]
fn read_clipboard_format(format: u32) -> Option<Vec<u8>> {
    let mut data = Vec::new();

    match clipboard_win::raw::get_vec(format, &mut data) {
        Ok(_) if !data.is_empty() => Some(data),
        _ => None,
    }
}

/// Форматы, которые нельзя скопировать побайтово: дескрипторы GDI, метафайлы,
/// форматы отображения, отрисовываемые владельцем (owner-drawn), и приватные
/// диапазоны, память которых система не отслеживает.
///
/// Изображения при этом переживают снимок. Windows перечисляет `CF_DIB`/`CF_DIBV5`
/// вместе с `CF_BITMAP` и синтезирует дескрипторы bitmap и палитры обратно из
/// блока DIB, который восстанавливается здесь.
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

/// Исключает текущую запись буфера обмена из монитора буфера обмена Windows.
///
/// Буфер обмена должен быть открыт.
#[cfg(target_os = "windows")]
fn exclude_from_clipboard_history() {
    use clipboard_win::raw;

    // "ExcludeClipboardContentFromMonitorProcessing": достаточно самого
    // присутствия формата, полезная нагрузка с данными не требуется.
    let exclude_format = registered_format(EXCLUDE_FROM_MONITOR_FORMAT);

    if exclude_format != 0 {
        set_empty_format(exclude_format);
    }

    // "CanIncludeInClipboardHistory": установить DWORD 0, чтобы отключить.
    let no_history_format = registered_format(CAN_INCLUDE_IN_HISTORY_FORMAT);

    if no_history_format != 0 {
        let _ = raw::set_without_clear(no_history_format, &0u32.to_ne_bytes());
    }
}

/// Материализует CF_BITMAP из только что восстановленного CF_DIB.
///
/// Исходный буфер обмена обычно нёс настоящий `HBITMAP`, который нельзя
/// скопировать в снимок, поэтому CF_BITMAP теперь приходится синтезировать.
/// Windows выводит его из того формата DIB, что присутствует, а её путь через
/// CF_DIBV5 трактует три маски BI_BITFIELDS как пиксельные данные, что
/// сдвигает изображение на три пикселя вбок. Чтение CF_BITMAP здесь
/// принудительно получает и кэширует правильный дескриптор, выведенный из
/// CF_DIB, прежде чем цель вставки успеет материализовать CF_DIBV5 и испортить
/// преобразование.
#[cfg(target_os = "windows")]
fn force_bitmap_synthesis() {
    use clipboard_win::{formats, raw};

    let Some(_clipboard) = open_clipboard() else {
        return;
    };

    let _ = raw::get_clipboard_data(formats::CF_BITMAP);
}

/// Возвращает id зарегистрированного формата буфера обмена или `0`, если
/// регистрация не удалась.
#[cfg(target_os = "windows")]
fn registered_format(name: &str) -> u32 {
    clipboard_win::raw::register_format(name).map_or(0, |format| format.get())
}

/// Помещает в буфер обмена формат с нулевым дескриптором, отмечая его как
/// присутствующий без полезной нагрузки.
///
/// Здесь используется именно необработанный вызов Win32, потому что
/// `clipboard_win::raw::set_without_clear` при пустом срезе данных молча
/// завершается успешно, ничего не записав.
///
/// Буфер обмена должен быть открыт.
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
