use std::sync::atomic::{AtomicBool, Ordering};

use crate::error::{AppError, AppResult};

static IS_HOTKEY_CAPTURE_ACTIVE: AtomicBool = AtomicBool::new(false);

#[tauri::command]
pub fn set_hotkey_capture_active(active: bool) {
    IS_HOTKEY_CAPTURE_ACTIVE.store(active, Ordering::Relaxed);
}

#[derive(Clone, Copy)]
pub enum ShortcutState {
    Pressed,
    Released,
}

pub fn normalize_hotkey(value: &str) -> AppResult<String> {
    Hotkey::parse(value).map(|hotkey| hotkey.to_normalized_string())
}

#[cfg(target_os = "windows")]
pub fn install_dictation_shortcut(app: tauri::AppHandle, hotkey: &str) -> AppResult<()> {
    windows_hook::install(app, hotkey)
}

#[cfg(not(target_os = "windows"))]
pub fn install_dictation_shortcut(_app: tauri::AppHandle, _hotkey: &str) -> AppResult<()> {
    Err(AppError::from(
        "Suppressing dictation shortcuts is only implemented on Windows in this version",
    ))
}

#[cfg(target_os = "windows")]
pub fn set_dictation_hotkey(hotkey: &str) -> AppResult<()> {
    windows_hook::set_hotkey(hotkey)
}

#[cfg(not(target_os = "windows"))]
pub fn set_dictation_hotkey(_hotkey: &str) -> AppResult<()> {
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn set_copy_latest_hotkey(hotkey: &str) -> AppResult<()> {
    windows_hook::set_copy_latest_hotkey(hotkey)
}

#[cfg(not(target_os = "windows"))]
pub fn set_copy_latest_hotkey(_hotkey: &str) -> AppResult<()> {
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn set_paste_latest_hotkey(hotkey: &str) -> AppResult<()> {
    windows_hook::set_paste_latest_hotkey(hotkey)
}

#[cfg(not(target_os = "windows"))]
pub fn set_paste_latest_hotkey(_hotkey: &str) -> AppResult<()> {
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn set_repeat_latest_hotkey(hotkey: &str) -> AppResult<()> {
    windows_hook::set_repeat_latest_hotkey(hotkey)
}

#[cfg(not(target_os = "windows"))]
pub fn set_repeat_latest_hotkey(_hotkey: &str) -> AppResult<()> {
    Ok(())
}

#[cfg(target_os = "windows")]
pub async fn wait_for_hotkey_release(hotkey: &str) -> AppResult<()> {
    windows_hook::wait_for_hotkey_release(hotkey).await
}

#[cfg(not(target_os = "windows"))]
pub async fn wait_for_hotkey_release(_hotkey: &str) -> AppResult<()> {
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn arm_cancel_hotkey(hotkey: &str) -> AppResult<()> {
    windows_hook::arm_cancel_hotkey(hotkey)
}

#[cfg(not(target_os = "windows"))]
pub fn arm_cancel_hotkey(_hotkey: &str) -> AppResult<()> {
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn disarm_cancel_hotkey() {
    windows_hook::disarm_cancel_hotkey();
}

#[cfg(not(target_os = "windows"))]
pub fn disarm_cancel_hotkey() {}

#[derive(Clone, Copy)]
enum ModifierSide {
    None,
    Either,
    Left,
    Right,
}

#[derive(Clone, Copy)]
struct Hotkey {
    ctrl: ModifierSide,
    alt: ModifierSide,
    shift: ModifierSide,
    win: ModifierSide,
    main_key: MainKey,
}

#[derive(Clone, Copy)]
struct MainKey {
    name: &'static str,
    vk: u32,
}

impl Hotkey {
    fn parse(value: &str) -> AppResult<Self> {
        let mut ctrl = ModifierSide::None;
        let mut alt = ModifierSide::None;
        let mut shift = ModifierSide::None;
        let mut win = ModifierSide::None;
        let mut main_key = None;

        for part in value
            .split('+')
            .map(str::trim)
            .filter(|part| !part.is_empty())
        {
            match part.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => ctrl = ModifierSide::Either,
                "lctrl" | "lcontrol" => ctrl = ModifierSide::Left,
                "rctrl" | "rcontrol" => ctrl = ModifierSide::Right,
                "alt" | "option" => alt = ModifierSide::Either,
                "lalt" | "loption" => alt = ModifierSide::Left,
                "ralt" | "roption" => alt = ModifierSide::Right,
                "shift" => shift = ModifierSide::Either,
                "lshift" => shift = ModifierSide::Left,
                "rshift" => shift = ModifierSide::Right,
                "win" | "windows" | "meta" | "super" | "cmd" | "command" => {
                    win = ModifierSide::Either
                }
                "lwin" => win = ModifierSide::Left,
                "rwin" => win = ModifierSide::Right,
                _ => {
                    if main_key.is_some() {
                        return Err(AppError::from("Shortcut can contain only one main key"));
                    }

                    main_key = Some(parse_main_key(part)?);
                }
            }
        }

        let Some(main_key) = main_key else {
            return Err(AppError::from("Shortcut must contain a non-modifier key"));
        };

        Ok(Self {
            ctrl,
            alt,
            shift,
            win,
            main_key,
        })
    }

    fn to_normalized_string(self) -> String {
        let mut parts = Vec::new();

        match self.ctrl {
            ModifierSide::Either => parts.push("Ctrl"),
            ModifierSide::Left => parts.push("LCtrl"),
            ModifierSide::Right => parts.push("RCtrl"),
            ModifierSide::None => {}
        }

        match self.alt {
            ModifierSide::Either => parts.push("Alt"),
            ModifierSide::Left => parts.push("LAlt"),
            ModifierSide::Right => parts.push("RAlt"),
            ModifierSide::None => {}
        }

        match self.shift {
            ModifierSide::Either => parts.push("Shift"),
            ModifierSide::Left => parts.push("LShift"),
            ModifierSide::Right => parts.push("RShift"),
            ModifierSide::None => {}
        }

        match self.win {
            ModifierSide::Either => parts.push("Win"),
            ModifierSide::Left => parts.push("LWin"),
            ModifierSide::Right => parts.push("RWin"),
            ModifierSide::None => {}
        }

        parts.push(self.main_key.name);
        parts.join("+")
    }
}

fn parse_main_key(value: &str) -> AppResult<MainKey> {
    let upper = value.trim().to_ascii_uppercase();

    if let Some(number) = upper.strip_prefix('F') {
        let number = number
            .parse::<u32>()
            .map_err(|_| AppError::from(format!("Unsupported shortcut key: {value}")))?;

        if (1..=24).contains(&number) {
            return Ok(MainKey {
                name: function_key_name(number),
                vk: 0x70 + number - 1,
            });
        }
    }

    if upper.len() == 1 {
        let byte = upper.as_bytes()[0];

        if byte.is_ascii_alphabetic() || byte.is_ascii_digit() {
            return Ok(MainKey {
                name: alpha_digit_key_name(byte),
                vk: byte.into(),
            });
        }
    }

    match upper.as_str() {
        "SPACE" => Ok(MainKey {
            name: "Space",
            vk: 0x20,
        }),
        "ENTER" | "RETURN" => Ok(MainKey {
            name: "Enter",
            vk: 0x0D,
        }),
        "ESC" | "ESCAPE" => Ok(MainKey {
            name: "Escape",
            vk: 0x1B,
        }),
        "TAB" => Ok(MainKey {
            name: "Tab",
            vk: 0x09,
        }),
        "BACKSPACE" => Ok(MainKey {
            name: "Backspace",
            vk: 0x08,
        }),
        "DELETE" | "DEL" => Ok(MainKey {
            name: "Delete",
            vk: 0x2E,
        }),
        "INSERT" | "INS" => Ok(MainKey {
            name: "Insert",
            vk: 0x2D,
        }),
        "HOME" => Ok(MainKey {
            name: "Home",
            vk: 0x24,
        }),
        "END" => Ok(MainKey {
            name: "End",
            vk: 0x23,
        }),
        "PAGEUP" | "PAGE_UP" => Ok(MainKey {
            name: "PageUp",
            vk: 0x21,
        }),
        "PAGEDOWN" | "PAGE_DOWN" => Ok(MainKey {
            name: "PageDown",
            vk: 0x22,
        }),
        "UP" | "ARROWUP" | "ARROW_UP" => Ok(MainKey {
            name: "ArrowUp",
            vk: 0x26,
        }),
        "DOWN" | "ARROWDOWN" | "ARROW_DOWN" => Ok(MainKey {
            name: "ArrowDown",
            vk: 0x28,
        }),
        "LEFT" | "ARROWLEFT" | "ARROW_LEFT" => Ok(MainKey {
            name: "ArrowLeft",
            vk: 0x25,
        }),
        "RIGHT" | "ARROWRIGHT" | "ARROW_RIGHT" => Ok(MainKey {
            name: "ArrowRight",
            vk: 0x27,
        }),
        _ => Err(AppError::from(format!("Unsupported shortcut key: {value}"))),
    }
}

fn function_key_name(number: u32) -> &'static str {
    match number {
        1 => "F1",
        2 => "F2",
        3 => "F3",
        4 => "F4",
        5 => "F5",
        6 => "F6",
        7 => "F7",
        8 => "F8",
        9 => "F9",
        10 => "F10",
        11 => "F11",
        12 => "F12",
        13 => "F13",
        14 => "F14",
        15 => "F15",
        16 => "F16",
        17 => "F17",
        18 => "F18",
        19 => "F19",
        20 => "F20",
        21 => "F21",
        22 => "F22",
        23 => "F23",
        24 => "F24",
        _ => unreachable!("function key range is checked before conversion"),
    }
}

fn alpha_digit_key_name(byte: u8) -> &'static str {
    match byte {
        b'0' => "0",
        b'1' => "1",
        b'2' => "2",
        b'3' => "3",
        b'4' => "4",
        b'5' => "5",
        b'6' => "6",
        b'7' => "7",
        b'8' => "8",
        b'9' => "9",
        b'A' => "A",
        b'B' => "B",
        b'C' => "C",
        b'D' => "D",
        b'E' => "E",
        b'F' => "F",
        b'G' => "G",
        b'H' => "H",
        b'I' => "I",
        b'J' => "J",
        b'K' => "K",
        b'L' => "L",
        b'M' => "M",
        b'N' => "N",
        b'O' => "O",
        b'P' => "P",
        b'Q' => "Q",
        b'R' => "R",
        b'S' => "S",
        b'T' => "T",
        b'U' => "U",
        b'V' => "V",
        b'W' => "W",
        b'X' => "X",
        b'Y' => "Y",
        b'Z' => "Z",
        _ => unreachable!("alpha/digit range is checked before conversion"),
    }
}

#[cfg(target_os = "windows")]
mod windows_hook {
    use std::{
        mem, ptr,
        sync::{
            atomic::Ordering,
            mpsc::{self, Sender},
            Mutex, OnceLock,
        },
        thread,
        time::Duration,
    };

    use tauri::AppHandle;
    use windows_sys::Win32::UI::{
        Input::KeyboardAndMouse::GetAsyncKeyState,
        WindowsAndMessaging::{
            CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
            HHOOK, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN,
            WM_SYSKEYUP,
        },
    };

    use super::{Hotkey, ModifierSide, ShortcutState};
    use crate::{
        dictation,
        error::{AppError, AppResult},
    };

    const VK_LCONTROL: u32 = 0xA2;
    const VK_RCONTROL: u32 = 0xA3;
    const VK_LSHIFT: u32 = 0xA0;
    const VK_RSHIFT: u32 = 0xA1;
    const VK_LMENU: u32 = 0xA4;
    const VK_RMENU: u32 = 0xA5;
    const VK_LWIN: u32 = 0x5B;
    const VK_RWIN: u32 = 0x5C;

    enum HookEvent {
        Dictation(ShortcutState),
        Cancel,
        CopyLatest,
        PasteLatest,
        RepeatLatest,
    }

    static HOTKEY: OnceLock<Mutex<HookHotkey>> = OnceLock::new();
    static CANCEL_HOTKEY: OnceLock<Mutex<Option<HookHotkey>>> = OnceLock::new();
    static COPY_LATEST_HOTKEY: OnceLock<Mutex<Option<HookHotkey>>> = OnceLock::new();
    static PASTE_LATEST_HOTKEY: OnceLock<Mutex<Option<HookHotkey>>> = OnceLock::new();
    static REPEAT_LATEST_HOTKEY: OnceLock<Mutex<Option<HookHotkey>>> = OnceLock::new();
    static EVENT_SENDER: OnceLock<Sender<HookEvent>> = OnceLock::new();

    struct HookHotkey {
        hotkey: Hotkey,
        is_main_key_down: bool,
    }

    pub fn install(app: AppHandle, hotkey: &str) -> AppResult<()> {
        set_hotkey(hotkey)?;

        let (sender, receiver) = mpsc::channel::<HookEvent>();
        let _ = EVENT_SENDER.set(sender);

        thread::spawn(move || {
            for event in receiver {
                match event {
                    HookEvent::Dictation(state) => dictation::handle_shortcut_event(&app, state),
                    HookEvent::Cancel => dictation::handle_cancel_shortcut(&app),
                    HookEvent::CopyLatest => dictation::handle_copy_latest_shortcut(&app),
                    HookEvent::PasteLatest => dictation::handle_paste_latest_shortcut(&app),
                    HookEvent::RepeatLatest => dictation::handle_repeat_latest_shortcut(&app),
                }
            }
        });

        let (install_sender, install_receiver) = mpsc::channel::<bool>();

        thread::spawn(move || unsafe {
            let hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_proc), ptr::null_mut(), 0);

            if hook.is_null() {
                let _ = install_sender.send(false);
                return;
            }

            let _ = install_sender.send(true);
            run_message_loop();
        });

        if !install_receiver.recv().unwrap_or(false) {
            return Err(AppError::from("Could not install Windows keyboard hook"));
        }

        Ok(())
    }

    pub fn set_hotkey(value: &str) -> AppResult<()> {
        let hotkey = Hotkey::parse(value)?;
        let mutex = HOTKEY.get_or_init(|| {
            Mutex::new(HookHotkey {
                hotkey,
                is_main_key_down: false,
            })
        });
        let mut config = mutex
            .lock()
            .map_err(|_| crate::error::AppError::from("Could not lock shortcut hook state"))?;

        config.hotkey = hotkey;
        config.is_main_key_down = false;

        Ok(())
    }

    fn get_cancel_hotkey() -> &'static Mutex<Option<HookHotkey>> {
        CANCEL_HOTKEY.get_or_init(|| Mutex::new(None))
    }

    fn get_paste_latest_hotkey() -> &'static Mutex<Option<HookHotkey>> {
        PASTE_LATEST_HOTKEY.get_or_init(|| Mutex::new(None))
    }

    fn get_copy_latest_hotkey() -> &'static Mutex<Option<HookHotkey>> {
        COPY_LATEST_HOTKEY.get_or_init(|| Mutex::new(None))
    }

    fn get_repeat_latest_hotkey() -> &'static Mutex<Option<HookHotkey>> {
        REPEAT_LATEST_HOTKEY.get_or_init(|| Mutex::new(None))
    }

    fn set_optional_hotkey(
        mutex: &'static Mutex<Option<HookHotkey>>,
        value: &str,
        lock_error: &'static str,
    ) -> AppResult<()> {
        let mut hotkey = mutex.lock().map_err(|_| AppError::from(lock_error))?;

        if value.trim().is_empty() {
            *hotkey = None;
            return Ok(());
        }

        match Hotkey::parse(value) {
            Ok(parsed) => {
                *hotkey = Some(HookHotkey {
                    hotkey: parsed,
                    is_main_key_down: false,
                });
                Ok(())
            }
            Err(error) => {
                *hotkey = None;
                Err(error)
            }
        }
    }

    pub fn arm_cancel_hotkey(value: &str) -> AppResult<()> {
        set_optional_hotkey(
            get_cancel_hotkey(),
            value,
            "Could not lock cancel hotkey state",
        )
    }

    pub fn set_paste_latest_hotkey(value: &str) -> AppResult<()> {
        set_optional_hotkey(
            get_paste_latest_hotkey(),
            value,
            "Could not lock paste latest hotkey state",
        )
    }

    pub fn set_copy_latest_hotkey(value: &str) -> AppResult<()> {
        set_optional_hotkey(
            get_copy_latest_hotkey(),
            value,
            "Could not lock copy latest hotkey state",
        )
    }

    pub fn set_repeat_latest_hotkey(value: &str) -> AppResult<()> {
        set_optional_hotkey(
            get_repeat_latest_hotkey(),
            value,
            "Could not lock repeat latest hotkey state",
        )
    }

    pub fn disarm_cancel_hotkey() {
        if let Ok(mut cancel) = get_cancel_hotkey().lock() {
            *cancel = None;
        }
    }

    pub async fn wait_for_hotkey_release(value: &str) -> AppResult<()> {
        if value.trim().is_empty() {
            return Ok(());
        }

        let hotkey = Hotkey::parse(value)?;

        for _ in 0..50 {
            if !is_hotkey_still_pressed(hotkey) {
                return Ok(());
            }

            tokio::time::sleep(Duration::from_millis(10)).await;
        }

        Ok(())
    }

    unsafe extern "system" fn keyboard_proc(code: i32, w_param: usize, l_param: isize) -> isize {
        if code < 0 {
            return CallNextHookEx(ptr::null_mut::<HHOOK>() as HHOOK, code, w_param, l_param);
        }

        let event = *(l_param as *const KBDLLHOOKSTRUCT);
        let is_key_down = w_param as u32 == WM_KEYDOWN || w_param as u32 == WM_SYSKEYDOWN;
        let is_key_up = w_param as u32 == WM_KEYUP || w_param as u32 == WM_SYSKEYUP;

        if !is_key_down && !is_key_up {
            return CallNextHookEx(ptr::null_mut::<HHOOK>() as HHOOK, code, w_param, l_param);
        }

        if should_consume_event(event.vkCode, is_key_down, is_key_up) {
            return 1;
        }

        CallNextHookEx(ptr::null_mut::<HHOOK>() as HHOOK, code, w_param, l_param)
    }

    fn should_consume_event(vk_code: u32, is_key_down: bool, is_key_up: bool) -> bool {
        try_consume_dictation_event(vk_code, is_key_down, is_key_up)
            || try_consume_cancel_event(vk_code, is_key_down, is_key_up)
            || try_consume_copy_latest_event(vk_code, is_key_down, is_key_up)
            || try_consume_paste_latest_event(vk_code, is_key_down, is_key_up)
            || try_consume_repeat_latest_event(vk_code, is_key_down, is_key_up)
    }

    fn try_consume_dictation_event(vk_code: u32, is_key_down: bool, is_key_up: bool) -> bool {
        if super::IS_HOTKEY_CAPTURE_ACTIVE.load(Ordering::Relaxed) {
            return false;
        }

        let Some(config_mutex) = HOTKEY.get() else {
            return false;
        };
        let Ok(mut config) = config_mutex.lock() else {
            return false;
        };

        if vk_code != config.hotkey.main_key.vk {
            return false;
        }

        if is_key_up && config.is_main_key_down {
            config.is_main_key_down = false;
            send_hook_event(HookEvent::Dictation(ShortcutState::Released));

            return true;
        }

        if !modifiers_match(config.hotkey) {
            return false;
        }

        if is_key_down {
            if !config.is_main_key_down {
                config.is_main_key_down = true;
                send_hook_event(HookEvent::Dictation(ShortcutState::Pressed));
            }

            return true;
        }

        if is_key_up {
            config.is_main_key_down = false;
            send_hook_event(HookEvent::Dictation(ShortcutState::Released));

            return true;
        }

        false
    }

    fn try_consume_cancel_event(vk_code: u32, is_key_down: bool, is_key_up: bool) -> bool {
        try_consume_optional_event(
            get_cancel_hotkey(),
            vk_code,
            is_key_down,
            is_key_up,
            HookEvent::Cancel,
        )
    }

    fn try_consume_paste_latest_event(vk_code: u32, is_key_down: bool, is_key_up: bool) -> bool {
        if super::IS_HOTKEY_CAPTURE_ACTIVE.load(Ordering::Relaxed) {
            return false;
        }

        try_consume_optional_event(
            get_paste_latest_hotkey(),
            vk_code,
            is_key_down,
            is_key_up,
            HookEvent::PasteLatest,
        )
    }

    fn try_consume_copy_latest_event(vk_code: u32, is_key_down: bool, is_key_up: bool) -> bool {
        if super::IS_HOTKEY_CAPTURE_ACTIVE.load(Ordering::Relaxed) {
            return false;
        }

        try_consume_optional_event(
            get_copy_latest_hotkey(),
            vk_code,
            is_key_down,
            is_key_up,
            HookEvent::CopyLatest,
        )
    }

    fn try_consume_repeat_latest_event(vk_code: u32, is_key_down: bool, is_key_up: bool) -> bool {
        if super::IS_HOTKEY_CAPTURE_ACTIVE.load(Ordering::Relaxed) {
            return false;
        }

        try_consume_optional_event(
            get_repeat_latest_hotkey(),
            vk_code,
            is_key_down,
            is_key_up,
            HookEvent::RepeatLatest,
        )
    }

    fn try_consume_optional_event(
        hotkey_mutex: &'static Mutex<Option<HookHotkey>>,
        vk_code: u32,
        is_key_down: bool,
        is_key_up: bool,
        hook_event: HookEvent,
    ) -> bool {
        let Ok(mut hotkey) = hotkey_mutex.lock() else {
            return false;
        };

        let Some(ref mut config) = *hotkey else {
            return false;
        };

        if vk_code != config.hotkey.main_key.vk {
            return false;
        }

        if is_key_up && config.is_main_key_down {
            config.is_main_key_down = false;
            return true;
        }

        if !modifiers_match(config.hotkey) {
            return false;
        }

        if is_key_down {
            if !config.is_main_key_down {
                config.is_main_key_down = true;
                send_hook_event(hook_event);
            }

            return true;
        }

        false
    }

    fn modifier_side_matches(side: ModifierSide, l_vk: u32, r_vk: u32) -> bool {
        match side {
            ModifierSide::None => !is_key_down(l_vk) && !is_key_down(r_vk),
            ModifierSide::Either => is_key_down(l_vk) || is_key_down(r_vk),
            ModifierSide::Left => is_key_down(l_vk) && !is_key_down(r_vk),
            ModifierSide::Right => !is_key_down(l_vk) && is_key_down(r_vk),
        }
    }

    fn modifier_side_is_pressed(side: ModifierSide, l_vk: u32, r_vk: u32) -> bool {
        match side {
            ModifierSide::None => false,
            ModifierSide::Either => is_key_down(l_vk) || is_key_down(r_vk),
            ModifierSide::Left => is_key_down(l_vk),
            ModifierSide::Right => is_key_down(r_vk),
        }
    }

    fn modifiers_match(hotkey: Hotkey) -> bool {
        modifier_side_matches(hotkey.ctrl, VK_LCONTROL, VK_RCONTROL)
            && modifier_side_matches(hotkey.alt, VK_LMENU, VK_RMENU)
            && modifier_side_matches(hotkey.shift, VK_LSHIFT, VK_RSHIFT)
            && modifier_side_matches(hotkey.win, VK_LWIN, VK_RWIN)
    }

    fn is_hotkey_still_pressed(hotkey: Hotkey) -> bool {
        is_key_down(hotkey.main_key.vk)
            || modifier_side_is_pressed(hotkey.ctrl, VK_LCONTROL, VK_RCONTROL)
            || modifier_side_is_pressed(hotkey.alt, VK_LMENU, VK_RMENU)
            || modifier_side_is_pressed(hotkey.shift, VK_LSHIFT, VK_RSHIFT)
            || modifier_side_is_pressed(hotkey.win, VK_LWIN, VK_RWIN)
    }

    fn is_key_down(vk: u32) -> bool {
        unsafe { GetAsyncKeyState(vk as i32) & 0x8000u16 as i16 != 0 }
    }

    fn send_hook_event(event: HookEvent) {
        if let Some(sender) = EVENT_SENDER.get() {
            let _ = sender.send(event);
        }
    }

    unsafe fn run_message_loop() {
        let mut message: MSG = mem::zeroed();

        while GetMessageW(&mut message, ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
}
