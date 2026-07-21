use std::{
    env,
    io::Cursor,
    path::PathBuf,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use chrono::{DateTime, Utc};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat, Stream, StreamConfig,
};
use rubato::{FftFixedInOut, Resampler};
use silero_vad_rust::{
    get_speech_timestamps,
    silero_vad::{model::OnnxModel, utils_vad::VadParameters},
};
use tauri::Manager;

use crate::{
    debug_log,
    error::{AppError, AppResult},
    i18n,
    metrics::{RunStage, RunTimer},
    settings::{self, EffectiveUiLanguage},
};

const LEVEL_EMIT_INTERVAL: Duration = Duration::from_millis(50);

/// Тишина, которая вставляется в аудио на месте паузы. Сама пауза в аудио не
/// попадает: сколько бы она ни длилась, куски записи разделяет ровно этот
/// промежуток.
const PAUSE_GAP_DURATION: Duration = Duration::from_millis(500);

const VAD_SAMPLE_RATE: u32 = 16_000;

/// Частота дискретизации, применяемая, когда модель распознавания неизвестна:
/// при отмене записи ещё до выбора модели или при повторе старой записи.
/// Совпадает с частотой Whisper — самой распространённой у моделей речи.
const FALLBACK_OUTPUT_SAMPLE_RATE: u32 = 16_000;

/// Число каналов отправляемого и сохраняемого аудио. Речь с микрофона моно по
/// смыслу, а второй канал удваивал бы объём без пользы; модели распознавания
/// принимают только моно и сводят стерео сами.
const OUTPUT_CHANNELS: u16 = 1;
const VAD_MINIMUM_SPEECH_DURATION_MS: u32 = 300;
const VAD_MINIMUM_SILENCE_DURATION_MS: u32 = 1_200;
const VAD_SPEECH_PAD_MS: u32 = 350;

/// Поток захвата с микрофона, который создаётся заранее и остаётся на паузе,
/// чтобы диктовка могла начаться дешёвым вызовом `stream.play()` вместо
/// дорогостоящего вызова WASAPI `build_input_stream` (сотни мс) на горячем пути.
///
/// Один и тот же подготовленный рекордер переиспользуется между сессиями:
/// `start` заново его активирует, `stop_to_audio` / `abort` возвращают его
/// в состояние паузы и очищают.
pub struct PreparedRecorder {
    stream: Stream,
    shared: Arc<RecorderShared>,
    sample_rate: u32,
    channels: u16,
    device_name: String,
}

/// Состояние, общее с аудио-коллбэком. Коллбэк накапливает сэмплы только пока
/// установлен `active`, поэтому поток на паузе, но живой, не хранит данных.
struct RecorderShared {
    samples: Mutex<Vec<f32>>,
    active: AtomicBool,
    last_level_emit: Mutex<Instant>,
}

pub struct RecordedAudio {
    pub bytes: Vec<u8>,
    pub duration_ms: u64,
    pub file_name: String,
    pub started_at: DateTime<Utc>,
}

/// Создаёт (но не запускает) поток захвата для текущего устройства ввода по
/// умолчанию. Здесь выполняется вся дорогостоящая работа — перечисление
/// устройств, согласование конфигурации и `build_input_stream` — чтобы
/// `PreparedRecorder::start` оставался быстрым.
pub fn prepare_recorder(app: &tauri::AppHandle) -> AppResult<PreparedRecorder> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or_else(|| i18n::text(app, "recording-no-default-input-device"))?;
    let device_name = device.name().unwrap_or_default();
    let supported_config = device.default_input_config().map_err(|error| {
        AppError::from(i18n::text_with(
            app,
            "recording-input-device-config-read-failed",
            &[("error", error.to_string())],
        ))
    })?;

    let sample_format = supported_config.sample_format();
    let config = supported_config.config();
    let sample_rate = config.sample_rate.0;
    let channels = config.channels;
    let shared = Arc::new(RecorderShared {
        samples: Mutex::new(Vec::new()),
        active: AtomicBool::new(false),
        last_level_emit: Mutex::new(Instant::now() - LEVEL_EMIT_INTERVAL),
    });

    let stream = match sample_format {
        SampleFormat::F32 => build_stream::<f32>(&device, &config, Arc::clone(&shared), app)?,
        SampleFormat::I16 => build_stream::<i16>(&device, &config, Arc::clone(&shared), app)?,
        SampleFormat::U16 => build_stream::<u16>(&device, &config, Arc::clone(&shared), app)?,
        _ => {
            return Err(AppError::from(i18n::text_with(
                app,
                "recording-unsupported-input-sample-format",
                &[("format", format!("{sample_format:?}"))],
            )));
        }
    };

    Ok(PreparedRecorder {
        stream,
        shared,
        sample_rate,
        channels,
        device_name,
    })
}

impl PreparedRecorder {
    /// `true`, если этот рекордер был создан для устройства, которое ОС в данный
    /// момент сообщает как устройство ввода по умолчанию. Когда возвращается
    /// false, вызывающий код пересоздаёт рекордер, чтобы учесть смену
    /// устройства (например, подключение гарнитуры).
    pub fn is_for_current_default_device(&self) -> bool {
        let host = cpal::default_host();
        match host.default_input_device() {
            Some(device) => device
                .name()
                .map(|name| name == self.device_name)
                .unwrap_or(false),
            None => false,
        }
    }

    /// Повторно активирует и запускает захват. Очищает оставшиеся сэмплы и
    /// возобновляет поток на паузе — единственная работа на горячем пути
    /// начала диктовки.
    pub fn start(&self, app: &tauri::AppHandle) -> AppResult<()> {
        if let Ok(mut samples) = self.shared.samples.lock() {
            samples.clear();
        }
        if let Ok(mut last_emit) = self.shared.last_level_emit.lock() {
            *last_emit = Instant::now() - LEVEL_EMIT_INTERVAL;
        }

        self.play(app)
    }

    /// Приостанавливает захват, сохраняя уже накопленные сэмплы. Индикатор
    /// микрофона ОС гаснет, как при остановке, но запись не завершается:
    /// `resume` продолжает ту же сессию, не теряя её начало.
    pub fn pause(&self) {
        self.pause_and_deactivate();
    }

    /// Продолжает приостановленный захват. В отличие от `start`, не очищает
    /// накопленные сэмплы, поэтому речь до паузы сохраняется, а сама пауза
    /// заменяется коротким промежутком тишины.
    pub fn resume(&self, app: &tauri::AppHandle) -> AppResult<()> {
        self.append_pause_gap();
        self.play(app)
    }

    /// Дописывает `PAUSE_GAP_DURATION` тишины в конец буфера. Без этого куски
    /// записи, разделённые паузой, склеиваются встык: последнее слово до паузы
    /// и первое после неё звучат как одно, что мешает и прослушиванию, и
    /// распознаванию. Пустой буфер остаётся пустым: тишина нужна между кусками,
    /// а не в начале записи (пауза сразу после старта, до первого коллбэка).
    fn append_pause_gap(&self) {
        let Ok(mut samples) = self.shared.samples.lock() else {
            return;
        };

        if samples.is_empty() {
            return;
        }

        let gap_frames = (self.sample_rate as f64 * PAUSE_GAP_DURATION.as_secs_f64()) as usize;
        let gap_samples = gap_frames * self.channels as usize;
        let gap_end = samples.len() + gap_samples;

        samples.resize(gap_end, 0.0);
    }

    fn play(&self, app: &tauri::AppHandle) -> AppResult<()> {
        // Активируем до play, чтобы записались самые первые коллбэки.
        self.shared.active.store(true, Ordering::SeqCst);

        self.stream.play().map_err(|error| {
            self.shared.active.store(false, Ordering::SeqCst);
            AppError::from(i18n::text_with(
                app,
                "recording-start-failed",
                &[("error", error.to_string())],
            ))
        })?;

        Ok(())
    }

    /// Останавливает захват и кодирует записанное в WAV. Оставляет рекордер
    /// на паузе и пустым, чтобы его можно было переиспользовать в следующей сессии.
    /// `model_sample_rate` — частота, на которой работает выбранная модель
    /// распознавания; `None`, если модель ещё неизвестна.
    pub fn stop_to_audio(
        &self,
        app: &tauri::AppHandle,
        started_at: DateTime<Utc>,
        is_silence_trimming_enabled: bool,
        model_sample_rate: Option<u32>,
        timer: Option<&RunTimer>,
    ) -> AppResult<Option<RecordedAudio>> {
        let ui_language = settings::get_effective_ui_language(app).unwrap_or_default();
        let stop_started_at = Instant::now();

        // Сначала ставим на паузу, чтобы индикатор микрофона ОС выключился сразу.
        self.pause_and_deactivate();

        let mut samples = self
            .shared
            .samples
            .lock()
            .map(|mut guard| std::mem::take(&mut *guard))
            .map_err(|_| {
                AppError::from(i18n::text_for_language(
                    ui_language,
                    "recording-read-samples-failed",
                    &[],
                ))
            })?;

        record_stage(timer, RunStage::RecordStop, stop_started_at);

        if samples.is_empty() {
            return Err(AppError::from(i18n::text_for_language(
                ui_language,
                "recording-no-audio-captured",
                &[],
            )));
        }

        if is_silence_trimming_enabled {
            let vad_started_at = Instant::now();
            let trimmed = trim_silence(samples, self.sample_rate, self.channels, started_at, app);

            record_stage(timer, RunStage::Vad, vad_started_at);

            samples = match trimmed {
                Ok(samples) => samples,
                Err(SilenceTrimError::NoSpeech | SilenceTrimError::SpeechTooShort) => {
                    return Ok(None);
                }
                Err(SilenceTrimError::VadFailed(_)) => {
                    return Err(AppError::from(i18n::text_for_language(
                        ui_language,
                        "recording-vad-failed",
                        &[],
                    )));
                }
            };
        }

        let duration_ms = audio_duration_ms(&samples, self.sample_rate, self.channels);
        let encode_started_at = Instant::now();
        let bytes = self.encode_for_upload(&samples, model_sample_rate, ui_language)?;

        record_stage(timer, RunStage::Encode, encode_started_at);

        Ok(Some(RecordedAudio {
            bytes,
            duration_ms,
            file_name: "dictation.wav".to_string(),
            started_at,
        }))
    }

    /// Приводит запись к формату, в котором она уходит провайдеру и ложится в
    /// историю: моно, PCM16, частота модели распознавания.
    ///
    /// Нормализация идёт после ресемплинга: интерполяция может дать отсчёты
    /// чуть выше исходного пика, и выравнивать громкость до неё означало бы
    /// клиппинг на выходе.
    fn encode_for_upload(
        &self,
        samples: &[f32],
        model_sample_rate: Option<u32>,
        ui_language: EffectiveUiLanguage,
    ) -> AppResult<Vec<u8>> {
        let mono = downmix_to_mono(samples, self.channels as usize);
        let target_rate = output_sample_rate(self.sample_rate, model_sample_rate);
        // Если ресемплинг не удался, аудио уходит в исходной частоте: потерять
        // диктовку из-за неудачного пересчёта хуже, чем отправить файл крупнее.
        let (mut output, output_rate) = match resample_mono(&mono, self.sample_rate, target_rate) {
            Ok(resampled) => (resampled, target_rate),
            Err(()) => (mono, self.sample_rate),
        };

        normalize_peak(&mut output);

        encode_wav_pcm16(&output, output_rate, OUTPUT_CHANNELS, ui_language)
    }

    /// Отменяет захват без создания аудио и отбрасывает все сэмплы. Оставляет
    /// рекордер на паузе и пустым, чтобы его можно было переиспользовать
    /// в следующей сессии.
    pub fn abort(&self) {
        self.pause_and_deactivate();
        if let Ok(mut samples) = self.shared.samples.lock() {
            samples.clear();
        }
    }

    fn pause_and_deactivate(&self) {
        // Останавливаем накопление до паузы, чтобы запоздавший коллбэк не добавил сэмплы.
        self.shared.active.store(false, Ordering::SeqCst);
        let _ = self.stream.pause();
    }
}

enum SilenceTrimError {
    NoSpeech,
    SpeechTooShort,
    VadFailed(VadFailureClassification),
}

#[derive(Clone, Copy)]
enum VadFailureClassification {
    Resample,
    RuntimeUnavailable,
    ModelResourceMissing,
    ModelLoad,
    Inference,
}

impl VadFailureClassification {
    fn as_str(self) -> &'static str {
        match self {
            Self::Resample => "resampleFailed",
            Self::RuntimeUnavailable => "runtimeUnavailable",
            Self::ModelResourceMissing => "modelResourceMissing",
            Self::ModelLoad => "modelLoadFailed",
            Self::Inference => "inferenceFailed",
        }
    }
}

/// Удаляет тишину по сегментам Silero VAD до нормализации и кодирования WAV.
fn trim_silence(
    samples: Vec<f32>,
    sample_rate: u32,
    channels: u16,
    started_at: DateTime<Utc>,
    app: &tauri::AppHandle,
) -> Result<Vec<f32>, SilenceTrimError> {
    let input_duration_ms = audio_duration_ms(&samples, sample_rate, channels);
    debug_log::log_event(
        app,
        "vad.started",
        None,
        serde_json::json!({
            "recordingStartedAt": started_at.to_rfc3339(),
            "inputDurationMs": input_duration_ms,
            "sampleRate": sample_rate,
            "channels": channels,
            "inputSampleCount": samples.len(),
        }),
    );
    let channels = channels as usize;
    if sample_rate == 0 || channels == 0 {
        log_vad_result(app, "noSpeech", input_duration_ms, 0, 0, None);
        return Err(SilenceTrimError::NoSpeech);
    }
    let mono = downmix_to_mono(&samples, channels);
    let vad_audio = match resample_for_vad(&mono, sample_rate) {
        Ok(audio) => audio,
        Err(error) => {
            return Err(log_vad_failure(
                app,
                input_duration_ms,
                sample_rate,
                channels as u16,
                error,
            ));
        }
    };
    if let Err(error) = configure_onnx_runtime(app) {
        return Err(log_vad_failure(
            app,
            input_duration_ms,
            sample_rate,
            channels as u16,
            error,
        ));
    }
    let mut model = match load_silero_vad_model(app) {
        Ok(model) => model,
        Err(error) => {
            return Err(log_vad_failure(
                app,
                input_duration_ms,
                sample_rate,
                channels as u16,
                error,
            ));
        }
    };
    let parameters = VadParameters {
        sampling_rate: VAD_SAMPLE_RATE,
        min_speech_duration_ms: VAD_MINIMUM_SPEECH_DURATION_MS,
        min_silence_duration_ms: VAD_MINIMUM_SILENCE_DURATION_MS,
        speech_pad_ms: VAD_SPEECH_PAD_MS,
        return_seconds: false,
        ..Default::default()
    };
    let segments = match get_speech_timestamps(&vad_audio, &mut model, &parameters) {
        Ok(segments) => segments,
        Err(_) => {
            return Err(log_vad_failure(
                app,
                input_duration_ms,
                sample_rate,
                channels as u16,
                SilenceTrimError::VadFailed(VadFailureClassification::Inference),
            ));
        }
    };
    if segments.is_empty() {
        let error = match classify_empty_speech(app, &vad_audio) {
            Ok(error) => error,
            Err(error) => {
                return Err(log_vad_failure(
                    app,
                    input_duration_ms,
                    sample_rate,
                    channels as u16,
                    error,
                ));
            }
        };
        log_vad_result(app, error.as_str(), input_duration_ms, 0, 0, None);
        return Err(error);
    }

    let segment_count = segments.len();
    let mut trimmed = Vec::with_capacity(samples.len());
    for segment in segments {
        let start = segment.start as u64 * sample_rate as u64 / VAD_SAMPLE_RATE as u64;
        let end = (segment.end as u64 * sample_rate as u64).div_ceil(VAD_SAMPLE_RATE as u64);
        let start = start as usize * channels;
        let end = end as usize * channels;
        let start = start.min(samples.len());
        let end = end.min(samples.len());
        if start < end {
            trimmed.extend_from_slice(&samples[start..end]);
        }
    }
    if trimmed.is_empty() {
        log_vad_result(
            app,
            "speechTooShort",
            input_duration_ms,
            0,
            segment_count,
            None,
        );
        Err(SilenceTrimError::SpeechTooShort)
    } else {
        log_vad_result(
            app,
            "completed",
            input_duration_ms,
            audio_duration_ms(&trimmed, sample_rate, channels as u16),
            segment_count,
            Some(vad_audio.len()),
        );
        Ok(trimmed)
    }
}

impl SilenceTrimError {
    fn as_str(&self) -> &'static str {
        match self {
            Self::NoSpeech => "noSpeech",
            Self::SpeechTooShort => "speechTooShort",
            Self::VadFailed(classification) => classification.as_str(),
        }
    }
}

fn log_vad_failure(
    app: &tauri::AppHandle,
    input_duration_ms: u64,
    sample_rate: u32,
    channels: u16,
    error: SilenceTrimError,
) -> SilenceTrimError {
    let classification = error.as_str();
    debug_log::log_critical_event(
        app,
        "vad.failed",
        None,
        serde_json::json!({
            "classification": classification,
            "inputDurationMs": input_duration_ms,
            "sampleRate": sample_rate,
            "channels": channels,
        }),
    );
    error
}

fn log_vad_result(
    app: &tauri::AppHandle,
    outcome: &'static str,
    input_duration_ms: u64,
    output_duration_ms: u64,
    segment_count: usize,
    vad_sample_count: Option<usize>,
) {
    debug_log::log_event(
        app,
        "vad.completed",
        None,
        serde_json::json!({
            "outcome": outcome,
            "inputDurationMs": input_duration_ms,
            "outputDurationMs": output_duration_ms,
            "segmentCount": segment_count,
            "vadSampleCount": vad_sample_count,
        }),
    );
}

fn audio_duration_ms(samples: &[f32], sample_rate: u32, channels: u16) -> u64 {
    if channels == 0 || sample_rate == 0 {
        return 0;
    }

    ((samples.len() as f64 / channels as f64) / sample_rate as f64 * 1000.0) as u64
}

/// Явно выбирает ONNX Runtime, поставляемый вместе с приложением, чтобы VAD не
/// подхватил одноимённую DLL из другой программы или системного пути.
fn configure_onnx_runtime(app: &tauri::AppHandle) -> Result<(), SilenceTrimError> {
    let resource_path = app
        .path()
        .resource_dir()
        .ok()
        .map(|directory| directory.join("onnxruntime.dll"));
    let development_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("onnxruntime.dll");
    let runtime_path = resource_path
        .filter(|path| path.is_file())
        .or_else(|| {
            // В dev-режиме Tauri не копирует bundle resources рядом с бинарным файлом.
            (cfg!(debug_assertions) && development_path.is_file()).then_some(development_path)
        })
        .ok_or(SilenceTrimError::VadFailed(
            VadFailureClassification::RuntimeUnavailable,
        ))?;

    env::set_var("ORT_DYLIB_PATH", runtime_path);
    Ok(())
}

/// Загружает ONNX-модель из ресурсов приложения, не полагаясь на путь Cargo registry.
fn load_silero_vad_model(app: &tauri::AppHandle) -> Result<OnnxModel, SilenceTrimError> {
    let resource_path = app
        .path()
        .resource_dir()
        .ok()
        .map(|directory| directory.join("silero_vad.onnx"));
    let development_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources")
        .join("silero_vad.onnx");
    let model_path = resource_path
        .filter(|path| path.is_file())
        .or_else(|| {
            (cfg!(debug_assertions) && development_path.is_file()).then_some(development_path)
        })
        .ok_or(SilenceTrimError::VadFailed(
            VadFailureClassification::ModelResourceMissing,
        ))?;

    OnnxModel::from_path(model_path, true)
        .map_err(|_| SilenceTrimError::VadFailed(VadFailureClassification::ModelLoad))
}

fn classify_empty_speech(
    app: &tauri::AppHandle,
    vad_audio: &[f32],
) -> Result<SilenceTrimError, SilenceTrimError> {
    let mut model = load_silero_vad_model(app)?;
    let parameters = VadParameters {
        sampling_rate: VAD_SAMPLE_RATE,
        min_speech_duration_ms: 1,
        min_silence_duration_ms: VAD_MINIMUM_SILENCE_DURATION_MS,
        speech_pad_ms: VAD_SPEECH_PAD_MS,
        ..Default::default()
    };
    let segments = get_speech_timestamps(vad_audio, &mut model, &parameters)
        .map_err(|_| SilenceTrimError::VadFailed(VadFailureClassification::Inference))?;
    Ok(if segments.is_empty() {
        SilenceTrimError::NoSpeech
    } else {
        SilenceTrimError::SpeechTooShort
    })
}

fn resample_for_vad(samples: &[f32], source_rate: u32) -> Result<Vec<f32>, SilenceTrimError> {
    resample_mono(samples, source_rate, VAD_SAMPLE_RATE)
        .map_err(|()| SilenceTrimError::VadFailed(VadFailureClassification::Resample))
}

/// Пересчитывает моно-дорожку в другую частоту дискретизации.
///
/// Используется и для подготовки входа VAD, и для приведения записи к формату
/// отправки, поэтому целевая частота — параметр, а не константа.
fn resample_mono(samples: &[f32], source_rate: u32, target_rate: u32) -> Result<Vec<f32>, ()> {
    if source_rate == target_rate || samples.is_empty() {
        return Ok(samples.to_vec());
    }

    let mut resampler =
        FftFixedInOut::<f32>::new(source_rate as usize, target_rate as usize, 1_024, 1)
            .map_err(|_| ())?;
    let block_size = resampler.input_frames_next();
    let mut output = Vec::new();
    let mut blocks = samples.chunks_exact(block_size);

    for block in &mut blocks {
        let mut channels = resampler.process(&[block], None).map_err(|_| ())?;
        output.append(&mut channels.remove(0));
    }

    let remainder = blocks.remainder();
    if !remainder.is_empty() {
        let mut channels = resampler
            .process_partial(Some(&[remainder]), None)
            .map_err(|_| ())?;
        output.append(&mut channels.remove(0));
    }

    Ok(output)
}

/// Частота, в которую пересчитывается запись перед отправкой.
///
/// Никогда не превышает частоту устройства: апсемплинг не добавил бы модели ни
/// одной новой детали, зато увеличил бы файл и время передачи. Если модель
/// неизвестна, берётся [`FALLBACK_OUTPUT_SAMPLE_RATE`].
fn output_sample_rate(device_sample_rate: u32, model_sample_rate: Option<u32>) -> u32 {
    model_sample_rate
        .unwrap_or(FALLBACK_OUTPUT_SAMPLE_RATE)
        .min(device_sample_rate)
}

/// Сводит чередующиеся каналы в один усреднением кадра.
fn downmix_to_mono(samples: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return samples.to_vec();
    }

    samples
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect()
}

/// Записывает длительность локального этапа, если замер ведётся.
fn record_stage(timer: Option<&RunTimer>, stage: RunStage, started_at: Instant) {
    if let Some(timer) = timer {
        timer.record_stage(stage, started_at.elapsed());
    }
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    shared: Arc<RecorderShared>,
    app: &tauri::AppHandle,
) -> AppResult<Stream>
where
    T: AudioSample + cpal::SizedSample,
{
    let app_handle = app.clone();
    let channels = config.channels as usize;

    device
        .build_input_stream(
            config,
            move |data: &[T], _| {
                if !shared.active.load(Ordering::Relaxed) {
                    return;
                }

                let converted: Vec<f32> = data.iter().map(AudioSample::to_f32).collect();

                if let Ok(mut target) = shared.samples.lock() {
                    target.extend_from_slice(&converted);
                }

                emit_levels_if_due(&app_handle, &converted, channels, &shared.last_level_emit);
            },
            move |error| {
                eprintln!("Recording input stream error: {error}");
            },
            None,
        )
        .map_err(|error| {
            AppError::from(i18n::text_with(
                app,
                "recording-build-input-stream-failed",
                &[("error", error.to_string())],
            ))
        })
}

fn emit_levels_if_due(
    app: &tauri::AppHandle,
    samples: &[f32],
    channels: usize,
    last_level_emit: &Mutex<Instant>,
) {
    let Ok(mut last_emit) = last_level_emit.lock() else {
        return;
    };

    if last_emit.elapsed() < LEVEL_EMIT_INTERVAL {
        return;
    }

    *last_emit = Instant::now();
    crate::overlay::emit_mic_level(app, calculate_input_level(samples, channels));
}

fn calculate_input_level(samples: &[f32], channels: usize) -> f32 {
    if samples.is_empty() || channels == 0 {
        return 0.0;
    }

    let frame_count = samples.len() / channels;

    if frame_count == 0 {
        return 0.0;
    }

    let sum_squares = samples.iter().map(|sample| sample * sample).sum::<f32>();

    (sum_squares / samples.len() as f32).sqrt().clamp(0.0, 1.0)
}

/// Целевой пик после нормализации, выражен в линейной амплитуде (-1 дБFS).
/// Оставляет небольшой запас ниже полной шкалы вместо нормализации ровно до 1.0.
const NORMALIZE_TARGET_PEAK: f32 = 0.891_251;

/// Верхняя граница применяемого усиления (~24 дБ). Без этого ограничения почти
/// тихий буфер (микрофон выключен, речь не захвачена) нормализовал бы свой шумовой
/// порог до полной громкости вместо того, чтобы остаться без изменений.
const NORMALIZE_MAX_GAIN: f32 = 16.0;

/// Масштабирует весь буфер одним коэффициентом так, чтобы его пиковая амплитуда
/// достигла `NORMALIZE_TARGET_PEAK`. Усиливает тихие записи (например, низкий
/// уровень входного сигнала микрофона) и мягко понижает записи, уже близкие
/// к клиппингу, без внесения искажений, поскольку коэффициент выводится из
/// собственного пика записи.
fn normalize_peak(samples: &mut [f32]) {
    let peak = samples
        .iter()
        .fold(0.0_f32, |max, &sample| max.max(sample.abs()));

    if peak <= f32::EPSILON {
        return;
    }

    let gain = (NORMALIZE_TARGET_PEAK / peak).min(NORMALIZE_MAX_GAIN);

    for sample in samples.iter_mut() {
        *sample *= gain;
    }
}

fn encode_wav_pcm16(
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
    language: EffectiveUiLanguage,
) -> AppResult<Vec<u8>> {
    let spec = hound::WavSpec {
        bits_per_sample: 16,
        channels,
        sample_format: hound::SampleFormat::Int,
        sample_rate,
    };
    let mut buffer = Cursor::new(Vec::new());

    {
        let mut writer = hound::WavWriter::new(&mut buffer, spec).map_err(|error| {
            AppError::from(i18n::text_for_language(
                language,
                "recording-create-wav-writer-failed",
                &[("error", error.to_string())],
            ))
        })?;

        for sample in samples {
            let pcm = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;

            writer.write_sample(pcm).map_err(|error| {
                AppError::from(i18n::text_for_language(
                    language,
                    "recording-write-wav-sample-failed",
                    &[("error", error.to_string())],
                ))
            })?;
        }

        writer.finalize().map_err(|error| {
            AppError::from(i18n::text_for_language(
                language,
                "recording-finalize-wav-failed",
                &[("error", error.to_string())],
            ))
        })?;
    }

    Ok(buffer.into_inner())
}

trait AudioSample: Send + 'static {
    fn to_f32(&self) -> f32;
}

impl AudioSample for f32 {
    fn to_f32(&self) -> f32 {
        *self
    }
}

impl AudioSample for i16 {
    fn to_f32(&self) -> f32 {
        *self as f32 / i16::MAX as f32
    }
}

impl AudioSample for u16 {
    fn to_f32(&self) -> f32 {
        (*self as f32 - 32768.0) / 32768.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downsamples_to_model_rate() {
        assert_eq!(output_sample_rate(48_000, Some(16_000)), 16_000);
        assert_eq!(output_sample_rate(48_000, Some(24_000)), 24_000);
    }

    #[test]
    fn never_upsamples_above_device_rate() {
        assert_eq!(output_sample_rate(16_000, Some(24_000)), 16_000);
        assert_eq!(output_sample_rate(8_000, Some(16_000)), 8_000);
    }

    #[test]
    fn falls_back_when_model_is_unknown() {
        assert_eq!(
            output_sample_rate(48_000, None),
            FALLBACK_OUTPUT_SAMPLE_RATE
        );
    }

    #[test]
    fn averages_channels_when_downmixing() {
        let stereo = [1.0, 0.0, 0.5, 0.5];

        assert_eq!(downmix_to_mono(&stereo, 2), vec![0.5, 0.5]);
    }

    #[test]
    fn keeps_mono_samples_unchanged() {
        let mono = [0.25, -0.5];

        assert_eq!(downmix_to_mono(&mono, 1), vec![0.25, -0.5]);
    }

    #[test]
    fn resamples_to_expected_length() {
        let samples = vec![0.0_f32; 48_000];
        let resampled = resample_mono(&samples, 48_000, 16_000).expect("resampling should succeed");

        // FFT-ресемплер работает блоками, поэтому длина близка к трети, но не
        // обязана совпадать с ней ровно.
        assert!(
            resampled.len().abs_diff(16_000) < 1_024,
            "unexpected length: {}",
            resampled.len()
        );
    }
}
