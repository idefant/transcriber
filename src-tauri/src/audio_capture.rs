//! Захват с микрофона напрямую через WASAPI.
//!
//! Модуль заменил `cpal` ради одного вызова, которого `cpal` не даёт:
//! `IAudioClient2::SetClientProperties` с категорией `AudioCategory_Speech`
//! до `IAudioClient::Initialize`. Обоснование и замеры — в
//! `docs/development/speech-audio-category.md`.
//!
//! Весь COM-объект живёт внутри одного выделенного потока: он же входит в
//! апартамент MTA, создаёт клиента, запускает и останавливает захват и читает
//! пакеты. Наружу торчат только команды через канал, поэтому ни один
//! COM-указатель не переезжает между потоками.

use std::{
    ffi::c_void,
    fmt, ptr,
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, JoinHandle},
};

use windows::{
    core::{w, Interface},
    Win32::{
        Foundation::{CloseHandle, E_FAIL, HANDLE, WAIT_EVENT, WAIT_OBJECT_0},
        Media::{
            Audio::{
                eCapture, eConsole, AudioCategory_Speech, AudioClientProperties,
                IAudioCaptureClient, IAudioClient, IAudioClient2, IMMDevice, IMMDeviceEnumerator,
                MMDeviceEnumerator, AUDCLNT_BUFFERFLAGS_SILENT, AUDCLNT_SHAREMODE_SHARED,
                AUDCLNT_STREAMFLAGS_EVENTCALLBACK, AUDCLNT_STREAMOPTIONS_NONE, WAVEFORMATEX,
                WAVEFORMATEXTENSIBLE, WAVE_FORMAT_PCM,
            },
            KernelStreaming::{KSDATAFORMAT_SUBTYPE_PCM, WAVE_FORMAT_EXTENSIBLE},
            Multimedia::{KSDATAFORMAT_SUBTYPE_IEEE_FLOAT, WAVE_FORMAT_IEEE_FLOAT},
        },
        System::{
            Com::{
                CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_ALL,
                COINIT_MULTITHREADED,
            },
            Threading::{
                AvRevertMmThreadCharacteristics, AvSetMmThreadCharacteristicsW, CreateEventW,
                SetEvent, WaitForMultipleObjects, INFINITE,
            },
        },
    },
};

/// Формат, в котором устройство отдаёт кадры вызывающему коду. Сэмплы всегда
/// приводятся к `f32`, поэтому здесь нет разрядности — только раскладка кадра.
#[derive(Clone, Copy)]
pub struct CaptureFormat {
    pub sample_rate: u32,
    pub channels: u16,
}

/// Причина, по которой захват не удалось открыть или продолжить. Вызывающий код
/// переводит её в локализованное сообщение, поэтому варианты различают именно
/// те случаи, для которых есть отдельные тексты.
pub enum CaptureError {
    /// Система не сообщает устройство ввода по умолчанию.
    NoDefaultDevice,
    /// Устройство отдаёт кадры в разрядности, которую мы не приводим к `f32`.
    /// Строка — техническое описание формата для сообщения об ошибке.
    UnsupportedSampleFormat(String),
    /// ОС отказала в создании потока захвата.
    ThreadStartFailed(String),
    /// Поток захвата завершился: либо не смог открыть клиента, либо упал позже.
    CaptureThreadGone,
    /// Ошибка WASAPI или COM.
    Windows(windows::core::Error),
}

impl fmt::Display for CaptureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoDefaultDevice => formatter.write_str("no default input device"),
            Self::UnsupportedSampleFormat(format) => {
                write!(formatter, "unsupported format: {format}")
            }
            Self::ThreadStartFailed(error) => {
                write!(formatter, "capture thread not started: {error}")
            }
            Self::CaptureThreadGone => formatter.write_str("capture thread is gone"),
            Self::Windows(error) => write!(formatter, "{error}"),
        }
    }
}

/// Открытый, но ещё не запущенный поток захвата.
///
/// Устройство удерживается инициализированным всё время жизни значения, а
/// индикатор микрофона ОС загорается только между [`CaptureStream::start`] и
/// [`CaptureStream::stop`] — этим и пользуется прогрев записи.
pub struct CaptureStream {
    commands: Sender<Command>,
    wakeup: OwnedEvent,
    format: CaptureFormat,
    device_id: String,
    speech_category_applied: bool,
    thread: Option<JoinHandle<()>>,
}

impl CaptureStream {
    pub fn format(&self) -> CaptureFormat {
        self.format
    }

    /// Идентификатор конечной точки, для которой открыт поток. Сравнивается с
    /// текущим устройством по умолчанию, чтобы заметить смену микрофона.
    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    /// `false`, если драйвер отказался принять речевую категорию и поток открыт
    /// обычным путём. Нужно только для диагностических логов.
    pub fn speech_category_applied(&self) -> bool {
        self.speech_category_applied
    }

    /// Запускает захват. Возвращается после того, как поток захвата
    /// действительно вызвал `IAudioClient::Start`, поэтому ошибка старта видна
    /// вызывающему коду сразу, а не теряется в фоне.
    pub fn start(&self) -> Result<(), CaptureError> {
        self.request(Command::Start)
    }

    /// Останавливает захват, сохраняя клиента инициализированным. Индикатор
    /// микрофона ОС гаснет, а следующий [`CaptureStream::start`] снова дёшев.
    pub fn stop(&self) -> Result<(), CaptureError> {
        self.request(Command::Stop)
    }

    fn request(
        &self,
        build: fn(Sender<Result<(), windows::core::Error>>) -> Command,
    ) -> Result<(), CaptureError> {
        let (reply_sender, reply_receiver) = mpsc::channel();

        self.commands
            .send(build(reply_sender))
            .map_err(|_| CaptureError::CaptureThreadGone)?;
        self.wake_capture_thread();

        match reply_receiver.recv() {
            Ok(result) => result.map_err(CaptureError::Windows),
            Err(_) => Err(CaptureError::CaptureThreadGone),
        }
    }

    fn wake_capture_thread(&self) {
        // Поток спит на событии устройства; без пробуждения команда пролежала бы
        // в очереди до следующего пакета, а у остановленного потока — вечно.
        let _ = unsafe { SetEvent(self.wakeup.0) };
    }
}

impl Drop for CaptureStream {
    fn drop(&mut self) {
        let _ = self.commands.send(Command::Terminate);
        self.wake_capture_thread();

        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

/// Идентификатор текущего устройства ввода по умолчанию либо `None`, если
/// устройства нет или его не удалось опросить.
pub fn default_capture_device_id() -> Option<String> {
    let apartment = ComApartment::enter().ok()?;
    let id = unsafe { default_capture_device() }
        .ok()
        .and_then(|device| unsafe { device_id(&device) }.ok());

    drop(apartment);
    id
}

/// Открывает поток захвата для устройства ввода по умолчанию.
///
/// `on_samples` вызывается из потока захвата с чередующимися кадрами в `f32` и
/// должен возвращаться быстро: пока он работает, буфер устройства не читается.
pub fn open_capture_stream<F>(on_samples: F) -> Result<CaptureStream, CaptureError>
where
    F: FnMut(&[f32]) + Send + 'static,
{
    let wakeup = OwnedEvent::create().map_err(CaptureError::Windows)?;
    let wakeup_for_thread = SharedHandle(wakeup.0);
    let (command_sender, command_receiver) = mpsc::channel();
    let (ready_sender, ready_receiver) = mpsc::channel();

    let thread = thread::Builder::new()
        .name("audio-capture".to_string())
        .spawn(move || {
            run_capture_thread(
                wakeup_for_thread,
                command_receiver,
                ready_sender,
                on_samples,
            );
        })
        .map_err(|error| CaptureError::ThreadStartFailed(error.to_string()))?;

    match ready_receiver.recv() {
        Ok(Ok(opened)) => Ok(CaptureStream {
            commands: command_sender,
            wakeup,
            format: opened.format,
            device_id: opened.device_id,
            speech_category_applied: opened.speech_category_applied,
            thread: Some(thread),
        }),
        Ok(Err(error)) => {
            let _ = thread.join();
            Err(error)
        }
        Err(_) => {
            let _ = thread.join();
            Err(CaptureError::CaptureThreadGone)
        }
    }
}

enum Command {
    Start(Sender<Result<(), windows::core::Error>>),
    Stop(Sender<Result<(), windows::core::Error>>),
    Terminate,
}

/// То, что поток захвата сообщает о себе после успешного открытия клиента.
struct OpenedInfo {
    format: CaptureFormat,
    device_id: String,
    speech_category_applied: bool,
}

/// Инициализированный клиент вместе со всем, что нужно циклу чтения.
struct OpenedClient {
    client: IAudioClient,
    capture: IAudioCaptureClient,
    event: OwnedEvent,
    format: CaptureFormat,
    sample_kind: SampleKind,
    device_id: String,
    speech_category_applied: bool,
}

fn run_capture_thread<F>(
    wakeup: SharedHandle,
    commands: Receiver<Command>,
    ready: Sender<Result<OpenedInfo, CaptureError>>,
    mut on_samples: F,
) where
    F: FnMut(&[f32]) + Send + 'static,
{
    // Апартамент объявлен первым, поэтому освобождается последним — уже после
    // того, как все COM-объекты потока уничтожены.
    let _apartment = match ComApartment::enter() {
        Ok(apartment) => apartment,
        Err(error) => {
            let _ = ready.send(Err(CaptureError::Windows(error)));
            return;
        }
    };

    let opened = match unsafe { open_client() } {
        Ok(opened) => opened,
        Err(error) => {
            let _ = ready.send(Err(error));
            return;
        }
    };

    let announcement = OpenedInfo {
        format: opened.format,
        device_id: opened.device_id.clone(),
        speech_category_applied: opened.speech_category_applied,
    };

    if ready.send(Ok(announcement)).is_err() {
        return;
    }

    let _priority = MmThreadPriority::acquire();
    let handles = [wakeup.0, opened.event.0];
    let mut buffer: Vec<f32> = Vec::new();
    let mut running = false;

    loop {
        let mut terminated = false;

        while let Ok(command) = commands.try_recv() {
            match command {
                Command::Start(reply) => {
                    let result = unsafe { opened.client.Start() };
                    running |= result.is_ok();
                    let _ = reply.send(result);
                }
                Command::Stop(reply) => {
                    let result = unsafe { opened.client.Stop() };
                    if result.is_ok() {
                        running = false;
                        // Сбрасываем непрочитанный хвост: иначе кадры, снятые до
                        // паузы, догнали бы запись уже после промежутка тишины.
                        let _ = unsafe { opened.client.Reset() };
                    }
                    let _ = reply.send(result);
                }
                Command::Terminate => terminated = true,
            }
        }

        if terminated {
            break;
        }

        let signalled = unsafe { WaitForMultipleObjects(&handles, false, INFINITE) };

        if signalled == WAIT_OBJECT_0 {
            continue;
        }

        if signalled != WAIT_EVENT(WAIT_OBJECT_0.0 + 1) {
            eprintln!("Audio capture wait failed: {}", signalled.0);
            break;
        }

        if !running {
            continue;
        }

        if let Err(error) = unsafe { read_packets(&opened, &mut buffer, &mut on_samples) } {
            eprintln!("Audio capture read failed: {error}");
            break;
        }
    }

    if running {
        let _ = unsafe { opened.client.Stop() };
    }
}

/// Вычитывает все готовые пакеты. Возвращается, когда устройству больше нечего
/// отдать, а не после первого пакета: за одним сигналом события их может быть
/// несколько.
unsafe fn read_packets<F>(
    opened: &OpenedClient,
    buffer: &mut Vec<f32>,
    on_samples: &mut F,
) -> windows::core::Result<()>
where
    F: FnMut(&[f32]),
{
    loop {
        let available = opened.capture.GetNextPacketSize()?;

        if available == 0 {
            return Ok(());
        }

        let mut data: *mut u8 = ptr::null_mut();
        let mut frames: u32 = 0;
        let mut flags: u32 = 0;

        opened
            .capture
            .GetBuffer(&mut data, &mut frames, &mut flags, None, None)?;

        let sample_count = frames as usize * opened.format.channels as usize;

        buffer.clear();

        if flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0 {
            // При этом флаге содержимое буфера не определено, и вместо него
            // положено считать тишину.
            buffer.resize(sample_count, 0.0);
        } else {
            convert_samples(data, sample_count, opened.sample_kind, buffer);
        }

        on_samples(buffer);

        opened.capture.ReleaseBuffer(frames)?;
    }
}

unsafe fn open_client() -> Result<OpenedClient, CaptureError> {
    let device = default_capture_device()?;
    let device_id = device_id(&device).map_err(CaptureError::Windows)?;
    let mut last_error = None;

    // Речевая категория — основной путь; если драйвер её не принимает, запись
    // всё равно должна состояться, поэтому вторая попытка идёт без неё.
    for use_speech_category in [true, false] {
        match initialize_client(&device, use_speech_category) {
            Ok(mut opened) => {
                opened.device_id = device_id;
                return Ok(opened);
            }
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error.unwrap_or(CaptureError::NoDefaultDevice))
}

unsafe fn initialize_client(
    device: &IMMDevice,
    use_speech_category: bool,
) -> Result<OpenedClient, CaptureError> {
    let client: IAudioClient = device
        .Activate(CLSCTX_ALL, None)
        .map_err(CaptureError::Windows)?;

    if use_speech_category {
        set_speech_category(&client).map_err(CaptureError::Windows)?;
    }

    // Формат читается после установки категории: драйвер вправе предложить для
    // речевого потока не тот формат, что для потока общего назначения.
    let format = MixFormat::get(&client).map_err(CaptureError::Windows)?;
    let header = format.header();
    let sample_kind = detect_sample_kind(&format)?;

    client
        .Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
            // Нулевая длительность буфера означает период движка по умолчанию.
            0,
            0,
            format.as_ptr(),
            None,
        )
        .map_err(CaptureError::Windows)?;

    let event = OwnedEvent::create().map_err(CaptureError::Windows)?;

    client
        .SetEventHandle(event.0)
        .map_err(CaptureError::Windows)?;

    let capture = client
        .GetService::<IAudioCaptureClient>()
        .map_err(CaptureError::Windows)?;

    Ok(OpenedClient {
        client,
        capture,
        event,
        format: CaptureFormat {
            sample_rate: header.nSamplesPerSec,
            channels: header.nChannels,
        },
        sample_kind,
        device_id: String::new(),
        speech_category_applied: use_speech_category,
    })
}

/// Помечает поток как речевой до `Initialize`.
///
/// На части ноутбуков обработка микрофона (APO поверх драйвера) для потоков
/// общего назначения включает агрессивное эхоподавление и отдаёт битовые нули,
/// пока считает, что говорит не пользователь. Речевая категория переводит ту же
/// обработку в режим, рассчитанный на диктовку.
unsafe fn set_speech_category(client: &IAudioClient) -> windows::core::Result<()> {
    let client: IAudioClient2 = client.cast()?;
    let properties = AudioClientProperties {
        cbSize: size_of::<AudioClientProperties>() as u32,
        bIsOffload: false.into(),
        eCategory: AudioCategory_Speech,
        Options: AUDCLNT_STREAMOPTIONS_NONE,
    };

    client.SetClientProperties(&properties)
}

unsafe fn default_capture_device() -> Result<IMMDevice, CaptureError> {
    let enumerator: IMMDeviceEnumerator =
        CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(CaptureError::Windows)?;

    enumerator
        .GetDefaultAudioEndpoint(eCapture, eConsole)
        .map_err(|_| CaptureError::NoDefaultDevice)
}

unsafe fn device_id(device: &IMMDevice) -> windows::core::Result<String> {
    let id = device.GetId()?;
    let value = id.to_string();

    CoTaskMemFree(Some(id.0 as *const c_void));
    value.map_err(|_| windows::core::Error::from(E_FAIL))
}

#[derive(Clone, Copy)]
enum SampleKind {
    F32,
    F64,
    I16,
    I32,
}

unsafe fn detect_sample_kind(format: &MixFormat) -> Result<SampleKind, CaptureError> {
    let header = format.header();
    let bits = header.wBitsPerSample;
    let tag = if header.wFormatTag as u32 == WAVE_FORMAT_EXTENSIBLE {
        let extensible = ptr::read_unaligned(format.as_ptr() as *const WAVEFORMATEXTENSIBLE);
        let sub_format = extensible.SubFormat;

        if sub_format == KSDATAFORMAT_SUBTYPE_IEEE_FLOAT {
            WAVE_FORMAT_IEEE_FLOAT
        } else if sub_format == KSDATAFORMAT_SUBTYPE_PCM {
            WAVE_FORMAT_PCM
        } else {
            return Err(CaptureError::UnsupportedSampleFormat(format!(
                "subformat {sub_format:?}, {bits} bits"
            )));
        }
    } else {
        header.wFormatTag as u32
    };

    match (tag, bits) {
        (WAVE_FORMAT_IEEE_FLOAT, 32) => Ok(SampleKind::F32),
        (WAVE_FORMAT_IEEE_FLOAT, 64) => Ok(SampleKind::F64),
        (WAVE_FORMAT_PCM, 16) => Ok(SampleKind::I16),
        (WAVE_FORMAT_PCM, 32) => Ok(SampleKind::I32),
        _ => Err(CaptureError::UnsupportedSampleFormat(format!(
            "tag {tag}, {bits} bits"
        ))),
    }
}

/// Приводит кадры устройства к `f32`. Чтение невыровненное: буфер WASAPI
/// приходит сырым указателем, и полагаться на его выравнивание незачем.
unsafe fn convert_samples(
    data: *const u8,
    sample_count: usize,
    kind: SampleKind,
    out: &mut Vec<f32>,
) {
    out.reserve(sample_count);

    match kind {
        SampleKind::F32 => {
            let data = data as *const f32;
            for index in 0..sample_count {
                out.push(ptr::read_unaligned(data.add(index)));
            }
        }
        SampleKind::F64 => {
            let data = data as *const f64;
            for index in 0..sample_count {
                out.push(ptr::read_unaligned(data.add(index)) as f32);
            }
        }
        SampleKind::I16 => {
            let data = data as *const i16;
            for index in 0..sample_count {
                out.push(ptr::read_unaligned(data.add(index)) as f32 / i16::MAX as f32);
            }
        }
        SampleKind::I32 => {
            let data = data as *const i32;
            for index in 0..sample_count {
                out.push(ptr::read_unaligned(data.add(index)) as f32 / i32::MAX as f32);
            }
        }
    }
}

/// Владелец формата, выданного `GetMixFormat`: память под него выделяет COM,
/// и освободить её должен вызывающий код.
struct MixFormat(*mut WAVEFORMATEX);

impl MixFormat {
    unsafe fn get(client: &IAudioClient) -> windows::core::Result<Self> {
        Ok(Self(client.GetMixFormat()?))
    }

    fn as_ptr(&self) -> *const WAVEFORMATEX {
        self.0
    }

    fn header(&self) -> WAVEFORMATEX {
        unsafe { ptr::read_unaligned(self.0) }
    }
}

impl Drop for MixFormat {
    fn drop(&mut self) {
        unsafe { CoTaskMemFree(Some(self.0 as *const c_void)) };
    }
}

/// Событие, которым владеет создавший его код.
struct OwnedEvent(HANDLE);

impl OwnedEvent {
    fn create() -> windows::core::Result<Self> {
        // Сбрасываемое автоматически и изначально несигнальное: ровно то, чего
        // ждёт `SetEventHandle`, и что нужно для пробуждения по команде.
        let handle = unsafe { CreateEventW(None, false, false, None) }?;

        Ok(Self(handle))
    }
}

impl Drop for OwnedEvent {
    fn drop(&mut self) {
        let _ = unsafe { CloseHandle(self.0) };
    }
}

// SAFETY: дескриптор события принадлежит процессу, а не потоку, поэтому и
// сигналить, и закрывать его можно из любого потока.
unsafe impl Send for OwnedEvent {}

/// Копия дескриптора события для потока захвата: закрывает его владелец,
/// поэтому здесь нет `Drop`.
struct SharedHandle(HANDLE);

// SAFETY: дескриптор события Windows допускает обращение из любого потока, а
// закрывается он только владеющим `OwnedEvent` — после join потока захвата.
unsafe impl Send for SharedHandle {}

/// Пребывание потока в апартаменте COM.
struct ComApartment {
    should_uninitialize: bool,
}

impl ComApartment {
    fn enter() -> windows::core::Result<Self> {
        let result = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };

        // RPC_E_CHANGED_MODE означает, что поток уже в другом апартаменте: COM
        // доступен, но выходить из него не наше дело.
        Ok(Self {
            should_uninitialize: result.is_ok(),
        })
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        if self.should_uninitialize {
            unsafe { CoUninitialize() };
        }
    }
}

/// Повышение класса планирования потока через MMCSS: без него захват страдает
/// от пропусков буфера под нагрузкой.
struct MmThreadPriority(HANDLE);

impl MmThreadPriority {
    fn acquire() -> Option<Self> {
        let mut task_index = 0u32;
        let handle = unsafe { AvSetMmThreadCharacteristicsW(w!("Audio"), &mut task_index) }.ok()?;

        Some(Self(handle))
    }
}

impl Drop for MmThreadPriority {
    fn drop(&mut self) {
        let _ = unsafe { AvRevertMmThreadCharacteristics(self.0) };
    }
}
