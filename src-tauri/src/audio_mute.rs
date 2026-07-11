use crate::error::{AppError, AppResult};

/// RAII-guard, который заглушает используемое по умолчанию устройство вывода звука при создании
/// и восстанавливает его предыдущее состояние заглушения при удалении (drop).
pub struct OutputMuteGuard {
    #[cfg(target_os = "windows")]
    previous_muted: bool,
}

impl OutputMuteGuard {
    #[cfg(target_os = "windows")]
    pub fn new() -> AppResult<Self> {
        let previous_muted = unsafe { get_default_endpoint_mute() }?;
        unsafe { set_default_endpoint_mute(true) }?;
        Ok(Self { previous_muted })
    }

    #[cfg(not(target_os = "windows"))]
    pub fn new() -> AppResult<Self> {
        Ok(Self {})
    }
}

impl Drop for OutputMuteGuard {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        // Восстанавливаем звук, только если заглушили его сами.
        if !self.previous_muted {
            let _ = unsafe { set_default_endpoint_mute(false) };
        }
    }
}

#[cfg(target_os = "windows")]
unsafe fn with_endpoint_volume<F, R>(f: F) -> AppResult<R>
where
    F: FnOnce(
        &windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume,
    ) -> windows::core::Result<R>,
{
    use windows::Win32::Media::Audio::Endpoints::IAudioEndpointVolume;
    use windows::Win32::Media::Audio::{
        eConsole, eRender, IMMDeviceEnumerator, MMDeviceEnumerator,
    };
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_ALL, COINIT_MULTITHREADED,
    };

    let com_hr = CoInitializeEx(None, COINIT_MULTITHREADED);
    // should_uninit равен true, когда мы успешно вошли в COM-апартамент (S_OK или S_FALSE).
    // Если у потока уже есть другой апартамент (RPC_E_CHANGED_MODE), com_hr.is_ok()
    // возвращает false — COM всё ещё доступен, но вызывать CoUninitialize нельзя.
    let should_uninit = com_hr.is_ok();

    let result = (|| -> windows::core::Result<R> {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
        let volume: IAudioEndpointVolume = device.Activate(CLSCTX_ALL, None)?;
        f(&volume)
    })();

    if should_uninit {
        CoUninitialize();
    }

    result.map_err(|e| AppError::from(format!("System audio error: {e}")))
}

#[cfg(target_os = "windows")]
unsafe fn get_default_endpoint_mute() -> AppResult<bool> {
    with_endpoint_volume(|vol| vol.GetMute().map(|b| b.as_bool()))
}

#[cfg(target_os = "windows")]
unsafe fn set_default_endpoint_mute(mute: bool) -> AppResult<()> {
    use windows::core::GUID;
    with_endpoint_volume(|vol| vol.SetMute(mute, std::ptr::null::<GUID>()))
}
