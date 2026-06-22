use crate::error::{AppError, AppResult};

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
struct Hotkey {
    ctrl: bool,
    alt: bool,
    shift: bool,
    win: bool,
    main_key: MainKey,
}

#[derive(Clone, Copy)]
struct MainKey {
    name: &'static str,
    vk: u32,
}

impl Hotkey {
    fn parse(value: &str) -> AppResult<Self> {
        let mut ctrl = false;
        let mut alt = false;
        let mut shift = false;
        let mut win = false;
        let mut main_key = None;

        for part in value
            .split('+')
            .map(str::trim)
            .filter(|part| !part.is_empty())
        {
            match part.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => ctrl = true,
                "alt" | "option" => alt = true,
                "shift" => shift = true,
                "win" | "windows" | "meta" | "super" | "cmd" | "command" => win = true,
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

        if self.ctrl {
            parts.push("Ctrl");
        }

        if self.alt {
            parts.push("Alt");
        }

        if self.shift {
            parts.push("Shift");
        }

        if self.win {
            parts.push("Win");
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
            mpsc::{self, Sender},
            Mutex, OnceLock,
        },
        thread,
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

    use super::{Hotkey, ShortcutState};
    use crate::{
        dictation,
        error::{AppError, AppResult},
    };

    const VK_CONTROL: u32 = 0x11;
    const VK_MENU: u32 = 0x12;
    const VK_SHIFT: u32 = 0x10;
    const VK_LWIN: u32 = 0x5B;
    const VK_RWIN: u32 = 0x5C;

    enum HookEvent {
        Dictation(ShortcutState),
        Cancel,
    }

    static HOTKEY: OnceLock<Mutex<HookHotkey>> = OnceLock::new();
    static CANCEL_HOTKEY: OnceLock<Mutex<Option<HookHotkey>>> = OnceLock::new();
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

    pub fn arm_cancel_hotkey(value: &str) -> AppResult<()> {
        let mut cancel = get_cancel_hotkey()
            .lock()
            .map_err(|_| AppError::from("Could not lock cancel hotkey state"))?;

        if value.trim().is_empty() {
            *cancel = None;
            return Ok(());
        }

        match Hotkey::parse(value) {
            Ok(hotkey) => {
                *cancel = Some(HookHotkey {
                    hotkey,
                    is_main_key_down: false,
                });
                Ok(())
            }
            Err(e) => {
                *cancel = None;
                Err(e)
            }
        }
    }

    pub fn disarm_cancel_hotkey() {
        if let Ok(mut cancel) = get_cancel_hotkey().lock() {
            *cancel = None;
        }
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
    }

    fn try_consume_dictation_event(vk_code: u32, is_key_down: bool, is_key_up: bool) -> bool {
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
        let Ok(mut cancel) = get_cancel_hotkey().lock() else {
            return false;
        };

        let Some(ref mut config) = *cancel else {
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
                send_hook_event(HookEvent::Cancel);
            }

            return true;
        }

        false
    }

    fn modifiers_match(hotkey: Hotkey) -> bool {
        is_key_down(VK_CONTROL) == hotkey.ctrl
            && is_key_down(VK_MENU) == hotkey.alt
            && is_key_down(VK_SHIFT) == hotkey.shift
            && (is_key_down(VK_LWIN) || is_key_down(VK_RWIN)) == hotkey.win
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
