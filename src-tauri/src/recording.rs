use std::{
    io::Cursor,
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

use crate::{
    error::{AppError, AppResult},
    i18n,
    settings::{self, EffectiveUiLanguage},
};

const LEVEL_EMIT_INTERVAL: Duration = Duration::from_millis(50);

/// A microphone capture stream that is built ahead of time and kept paused so a
/// dictation can start with only a cheap `stream.play()` instead of paying the
/// expensive WASAPI `build_input_stream` cost (hundreds of ms) on the hot path.
///
/// The same prepared recorder is reused across sessions: `start` re-arms it,
/// `stop_to_audio` / `abort` return it to the paused, empty state.
pub struct PreparedRecorder {
    stream: Stream,
    shared: Arc<RecorderShared>,
    sample_rate: u32,
    channels: u16,
    device_name: String,
}

/// State shared with the audio callback. The callback only accumulates samples
/// while `active` is set, so a paused-but-alive stream keeps no data.
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

/// Build (but do not start) a capture stream for the current default input
/// device. This performs all the expensive work — device enumeration, config
/// negotiation and `build_input_stream` — so `PreparedRecorder::start` stays fast.
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
    /// True when this recorder was built for the device the OS currently reports
    /// as the default input. When it returns false the caller rebuilds so a
    /// device change (e.g. plugging in a headset) is honored.
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

    /// Re-arm and start capturing. Clears any leftover samples and resumes the
    /// paused stream — the only work on the dictation-start hot path.
    pub fn start(&self, app: &tauri::AppHandle) -> AppResult<()> {
        if let Ok(mut samples) = self.shared.samples.lock() {
            samples.clear();
        }
        if let Ok(mut last_emit) = self.shared.last_level_emit.lock() {
            *last_emit = Instant::now() - LEVEL_EMIT_INTERVAL;
        }

        // Arm before play so the very first callbacks are recorded.
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

    /// Stop capturing and encode what was recorded to WAV. Leaves the recorder
    /// paused and empty so it can be reused for the next session.
    pub fn stop_to_audio(
        &self,
        app: &tauri::AppHandle,
        started_at: DateTime<Utc>,
    ) -> AppResult<RecordedAudio> {
        let ui_language = settings::get_effective_ui_language(app).unwrap_or_default();

        // Pause first so the OS microphone indicator turns off immediately.
        self.pause_and_deactivate();

        let samples = self
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

        if samples.is_empty() {
            return Err(AppError::from(i18n::text_for_language(
                ui_language,
                "recording-no-audio-captured",
                &[],
            )));
        }

        let duration_ms = if self.channels == 0 || self.sample_rate == 0 {
            0
        } else {
            ((samples.len() as f64 / self.channels as f64) / self.sample_rate as f64 * 1000.0)
                as u64
        };

        Ok(RecordedAudio {
            bytes: encode_wav_pcm16(&samples, self.sample_rate, self.channels, ui_language)?,
            duration_ms,
            file_name: "dictation.wav".to_string(),
            started_at,
        })
    }

    /// Cancel capturing without producing audio and discard any samples. Leaves
    /// the recorder paused and empty so it can be reused for the next session.
    pub fn abort(&self) {
        self.pause_and_deactivate();
        if let Ok(mut samples) = self.shared.samples.lock() {
            samples.clear();
        }
    }

    fn pause_and_deactivate(&self) {
        // Stop accumulating before pausing so no late callback appends samples.
        self.shared.active.store(false, Ordering::SeqCst);
        let _ = self.stream.pause();
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
