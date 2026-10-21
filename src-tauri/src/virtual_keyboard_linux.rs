//! Отправка сочетаний со стандартными evdev-кодами, включая системные бинды Hyprland.

use crate::error::{AppError, AppResult};
use std::{
    fs::File,
    io::Write,
    os::fd::{AsFd, FromRawFd},
};
use wayland_client::{
    delegate_noop,
    globals::{registry_queue_init, GlobalListContents},
    protocol::{wl_registry, wl_seat},
    Connection, Dispatch, EventQueue, QueueHandle,
};
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::{
    zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1,
    zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
};

// Сжатая карта wtype назначает первой клавише код 1 (Escape в evdev).
// Полная стандартная карта сохраняет одинаковый смысл кодов для клиента и compositor.
const KEYMAP: &[u8] = b"xkb_keymap {\n\
    xkb_keycodes { include \"evdev+aliases(qwerty)\" };\n\
    xkb_types { include \"complete\" };\n\
    xkb_compatibility { include \"complete\" };\n\
    xkb_symbols { include \"pc+us+inet(evdev)\" };\n\
};\n\0";

struct KeyboardState;

impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for KeyboardState {
    fn event(
        _: &mut Self,
        _: &wl_registry::WlRegistry,
        _: wl_registry::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

delegate_noop!(KeyboardState: ignore wl_seat::WlSeat);
delegate_noop!(KeyboardState: ignore ZwpVirtualKeyboardManagerV1);
delegate_noop!(KeyboardState: ignore ZwpVirtualKeyboardV1);

fn keyboard_error(error: impl std::fmt::Display) -> AppError {
    AppError::from(format!("Wayland virtual keyboard: {error}"))
}

/** Создаётся до замены буфера: отказ протокола не затрагивает пользовательские данные. */
pub struct VirtualKeyboard {
    queue: EventQueue<KeyboardState>,
    keyboard: ZwpVirtualKeyboardV1,
}

impl VirtualKeyboard {
    /** Публикует полную US-карту с evdev-кодами, не отправляя ни одного нажатия. */
    pub fn new() -> AppResult<Self> {
        let connection = Connection::connect_to_env().map_err(keyboard_error)?;
        let (globals, mut queue) =
            registry_queue_init::<KeyboardState>(&connection).map_err(keyboard_error)?;
        let handle = queue.handle();
        let seat: wl_seat::WlSeat = globals.bind(&handle, 1..=1, ()).map_err(keyboard_error)?;
        let manager: ZwpVirtualKeyboardManagerV1 =
            globals.bind(&handle, 1..=1, ()).map_err(keyboard_error)?;
        let keyboard = manager.create_virtual_keyboard(&seat, &handle, ());
        let fd = unsafe { libc::memfd_create(c"transcriber-keymap".as_ptr(), libc::MFD_CLOEXEC) };
        if fd < 0 {
            return Err(std::io::Error::last_os_error().into());
        }
        // fd принадлежит File и закрывается на любом пути выхода.
        let mut keymap = unsafe { File::from_raw_fd(fd) };
        keymap.write_all(KEYMAP)?;
        keyboard.keymap(1, keymap.as_fd(), KEYMAP.len() as u32);
        queue
            .roundtrip(&mut KeyboardState)
            .map_err(keyboard_error)?;
        Ok(Self { queue, keyboard })
    }

    /** Отпускает основную клавишу и модификаторы перед уничтожением устройства. */
    pub fn send(mut self, modifiers: u32, keycode: u32) -> AppResult<()> {
        self.keyboard.modifiers(modifiers, 0, 0, 0);
        self.keyboard.key(0, keycode, 1);
        self.queue
            .roundtrip(&mut KeyboardState)
            .map_err(keyboard_error)?;
        self.keyboard.key(0, keycode, 0);
        self.keyboard.modifiers(0, 0, 0, 0);
        self.queue
            .roundtrip(&mut KeyboardState)
            .map_err(keyboard_error)?;
        self.keyboard.destroy();
        self.queue
            .roundtrip(&mut KeyboardState)
            .map_err(keyboard_error)?;
        Ok(())
    }
}
