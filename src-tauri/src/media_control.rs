#[cfg(target_os = "windows")]
use crate::error::{AppError, AppResult};

#[cfg(target_os = "windows")]
use std::{
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

#[cfg(target_os = "windows")]
use windows::{
    core::AgileReference,
    Media::Control::{
        GlobalSystemMediaTransportControlsSession,
        GlobalSystemMediaTransportControlsSessionManager,
        GlobalSystemMediaTransportControlsSessionPlaybackStatus,
    },
};

/// Сколько ждём, пока приложение действительно остановит воспроизведение. Команда паузы
/// доставляется по IPC, и плееры реагируют не мгновенно (Spotify — порядка 100–300 мс).
/// Потолок нужен, чтобы неотвечающее приложение не подвесило старт записи навсегда.
#[cfg(target_os = "windows")]
const PAUSE_CONFIRM_TIMEOUT: Duration = Duration::from_millis(500);

#[cfg(target_os = "windows")]
const PAUSE_POLL_INTERVAL: Duration = Duration::from_millis(25);

/// RAII-guard, который возобновляет медиа-сессии, поставленные на паузу на время записи.
///
/// Возобновляются только те сессии, которые поставил на паузу сам guard и которые всё ещё
/// стоят на паузе: если пользователь за время записи сам нажал play или переключил трек,
/// перебивать его не нужно.
pub struct MediaPauseGuard {
    #[cfg(target_os = "windows")]
    paused_ids: Vec<String>,
}

impl Drop for MediaPauseGuard {
    fn drop(&mut self) {
        #[cfg(target_os = "windows")]
        if let Err(error) = resume_sessions(&self.paused_ids) {
            eprintln!("Failed to resume media sessions: {}", error.into_message());
        }
    }
}

/// Идентификаторы медиа-сессий, которые сейчас играют и умеют вставать на паузу.
///
/// Учитываются только приложения, публикующие себя в системном медиа-транспорте (плееры,
/// браузеры). Игры, звонки и системные уведомления сюда не попадают — их воспроизведение
/// остановить нельзя.
#[cfg(target_os = "windows")]
pub fn playing_sessions() -> Vec<String> {
    let result = with_media_com(|| {
        let manager = session_manager()?;
        let mut ids = Vec::new();

        for session in manager.GetSessions()? {
            // Сессия могла закрыться прямо сейчас — пропускаем её, а не роняем весь опрос.
            let Ok(info) = session.GetPlaybackInfo() else {
                continue;
            };

            if info.PlaybackStatus().ok()
                != Some(GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing)
            {
                continue;
            }

            let can_pause = info
                .Controls()
                .and_then(|controls| controls.IsPauseEnabled())
                .unwrap_or(false);

            if !can_pause {
                continue;
            }

            if let Ok(id) = session.SourceAppUserModelId() {
                ids.push(id.to_string());
            }
        }

        Ok(ids)
    });

    result.unwrap_or_else(|error| {
        eprintln!("Failed to inspect media sessions: {}", error.into_message());
        Vec::new()
    })
}

/// Ставит перечисленные сессии на паузу и ждёт, пока воспроизведение действительно
/// остановится (но не дольше [`PAUSE_CONFIRM_TIMEOUT`]).
///
/// Ожидание обязательно: если вернуть управление сразу, музыка успеет попасть в первые
/// сотни миллисекунд записи. Возвращает `None`, если паузить оказалось нечего.
#[cfg(target_os = "windows")]
pub fn pause_sessions(ids: &[String]) -> Option<MediaPauseGuard> {
    if ids.is_empty() {
        return None;
    }

    let paused_ids = with_media_com(|| {
        let manager = session_manager()?;
        let mut paused_ids = Vec::new();

        for id in ids {
            let Some(session) = find_session(&manager, id)? else {
                continue;
            };

            let is_paused = session
                .TryPauseAsync()
                .and_then(|operation| operation.get())
                .unwrap_or(false);

            if is_paused {
                paused_ids.push(id.clone());
            }
        }

        wait_until_paused(&manager, &paused_ids);

        Ok(paused_ids)
    });

    match paused_ids {
        Ok(paused_ids) if paused_ids.is_empty() => None,
        Ok(paused_ids) => Some(MediaPauseGuard { paused_ids }),
        Err(error) => {
            eprintln!("Failed to pause media sessions: {}", error.into_message());
            None
        }
    }
}

/// Оплачивает первый `RequestAsync` заранее: он поднимает инфраструктуру WinRT и стоит
/// сотни миллисекунд, а на старте диктовки этого времени нет. Выполняется в фоновом потоке;
/// ошибка не фатальна — менеджер будет запрошен по требованию.
#[cfg(target_os = "windows")]
pub fn prewarm() {
    std::thread::spawn(|| {
        if let Err(error) = with_media_com(|| session_manager().map(|_| ())) {
            eprintln!("Media session prewarm failed: {}", error.into_message());
        }
    });
}

/// Возвращает сессии, поставленные на паузу этим guard'ом, к воспроизведению.
#[cfg(target_os = "windows")]
fn resume_sessions(ids: &[String]) -> AppResult<()> {
    if ids.is_empty() {
        return Ok(());
    }

    with_media_com(|| {
        let manager = session_manager()?;

        for id in ids {
            let Some(session) = find_session(&manager, id)? else {
                continue;
            };

            let Ok(info) = session.GetPlaybackInfo() else {
                continue;
            };

            // Пользователь мог сам продолжить воспроизведение или переключить трек — не перебиваем.
            if info.PlaybackStatus().ok()
                != Some(GlobalSystemMediaTransportControlsSessionPlaybackStatus::Paused)
            {
                continue;
            }

            let can_play = info
                .Controls()
                .and_then(|controls| controls.IsPlayEnabled())
                .unwrap_or(false);

            if !can_play {
                continue;
            }

            let _ = session.TryPlayAsync().and_then(|operation| operation.get());
        }

        Ok(())
    })
}

/// Ждёт, пока все указанные сессии перестанут играть. Возвращает управление досрочно,
/// как только это произошло.
#[cfg(target_os = "windows")]
fn wait_until_paused(manager: &GlobalSystemMediaTransportControlsSessionManager, ids: &[String]) {
    if ids.is_empty() {
        return;
    }

    let deadline = Instant::now() + PAUSE_CONFIRM_TIMEOUT;

    loop {
        let all_paused = ids.iter().all(|id| !is_playing(manager, id));

        if all_paused || Instant::now() >= deadline {
            return;
        }

        std::thread::sleep(PAUSE_POLL_INTERVAL);
    }
}

/// Играет ли сессия прямо сейчас. Исчезнувшая сессия играющей не считается.
#[cfg(target_os = "windows")]
fn is_playing(manager: &GlobalSystemMediaTransportControlsSessionManager, id: &str) -> bool {
    let Ok(Some(session)) = find_session(manager, id) else {
        return false;
    };

    session
        .GetPlaybackInfo()
        .and_then(|info| info.PlaybackStatus())
        .map(|status| status == GlobalSystemMediaTransportControlsSessionPlaybackStatus::Playing)
        .unwrap_or(false)
}

/// Ищет сессию по идентификатору приложения. Список сессий перечитывается на каждый вызов:
/// сессия могла закрыться, пока шла запись.
#[cfg(target_os = "windows")]
fn find_session(
    manager: &GlobalSystemMediaTransportControlsSessionManager,
    id: &str,
) -> windows::core::Result<Option<GlobalSystemMediaTransportControlsSession>> {
    for session in manager.GetSessions()? {
        if session
            .SourceAppUserModelId()
            .is_ok_and(|session_id| session_id == id)
        {
            return Ok(Some(session));
        }
    }

    Ok(None)
}

/// Менеджер сессий, закэшированный между вызовами. Хранится через `AgileReference`, потому
/// что COM-объект переживает поток, в котором был создан, и должен быть `Send`.
#[cfg(target_os = "windows")]
fn session_manager() -> windows::core::Result<GlobalSystemMediaTransportControlsSessionManager> {
    static MANAGER: OnceLock<
        Mutex<Option<AgileReference<GlobalSystemMediaTransportControlsSessionManager>>>,
    > = OnceLock::new();

    // Отравленный мьютекс не повод отказывать: просто работаем без кэша.
    let Ok(mut cached) = MANAGER.get_or_init(|| Mutex::new(None)).lock() else {
        return request_session_manager();
    };

    if let Some(reference) = cached.as_ref() {
        return reference.resolve();
    }

    let manager = request_session_manager()?;
    *cached = Some(AgileReference::new(&manager)?);

    Ok(manager)
}

#[cfg(target_os = "windows")]
fn request_session_manager(
) -> windows::core::Result<GlobalSystemMediaTransportControlsSessionManager> {
    GlobalSystemMediaTransportControlsSessionManager::RequestAsync()?.get()
}

/// Выполняет `f` в MTA-апартаменте COM: блокирующий `.get()` на WinRT-операции внутри STA
/// привёл бы к взаимной блокировке.
///
/// Повторяет схему `audio_mute::with_endpoint_volume`, включая `RPC_E_CHANGED_MODE`: чужой
/// апартамент — не ошибка, COM доступен, просто вызывать `CoUninitialize` нам нельзя.
#[cfg(target_os = "windows")]
fn with_media_com<F, R>(f: F) -> AppResult<R>
where
    F: FnOnce() -> windows::core::Result<R>,
{
    use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED};

    let com_hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    let should_uninit = com_hr.is_ok();

    let result = f();

    if should_uninit {
        unsafe { CoUninitialize() };
    }

    result.map_err(|e| AppError::from(format!("System media error: {e}")))
}

#[cfg(not(target_os = "windows"))]
pub fn playing_sessions() -> Vec<String> {
    Vec::new()
}

#[cfg(not(target_os = "windows"))]
pub fn pause_sessions(_ids: &[String]) -> Option<MediaPauseGuard> {
    None
}

#[cfg(not(target_os = "windows"))]
pub fn prewarm() {}
