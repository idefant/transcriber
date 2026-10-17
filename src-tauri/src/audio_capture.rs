//! Платформенный захват микрофона с единым жизненным циклом.

#[cfg(target_os = "windows")]
#[path = "audio_capture_windows.rs"]
mod platform;
#[cfg(target_os = "linux")]
#[path = "audio_capture_linux.rs"]
mod platform;

pub use platform::*;
