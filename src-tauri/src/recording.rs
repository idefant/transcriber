use std::{
    io::Cursor,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use chrono::{DateTime, Utc};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat, Stream, StreamConfig,
};

use crate::{
    error::{AppError, AppResult},
    overlay,
};

const LEVEL_EMIT_INTERVAL: Duration = Duration::from_millis(50);

pub struct AudioRecording {
    stream: Option<Stream>,
    samples: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    channels: u16,
    started_at: DateTime<Utc>,
}

pub struct RecordedAudio {
    pub bytes: Vec<u8>,
    pub duration_ms: u64,
    pub file_name: String,
    pub started_at: DateTime<Utc>,
}

pub fn start_recording(app: tauri::AppHandle) -> AppResult<AudioRecording> {
    let host = cpal::default_host();
    let device = host
        .default_input_device()
        .ok_or("No default input device is available")?;
    let supported_config = device
        .default_input_config()
        .map_err(|error| AppError::from(format!("Could not read input device config: {error}")))?;

    let sample_format = supported_config.sample_format();
    let config = supported_config.config();
    let sample_rate = config.sample_rate.0;
    let channels = config.channels;
    let samples = Arc::new(Mutex::new(Vec::new()));
    let last_level_emit = Arc::new(Mutex::new(Instant::now() - LEVEL_EMIT_INTERVAL));

    let stream = match sample_format {
        SampleFormat::F32 => build_stream::<f32>(
            &device,
            &config,
            Arc::clone(&samples),
            Arc::clone(&last_level_emit),
            app,
        )?,
        SampleFormat::I16 => build_stream::<i16>(
            &device,
            &config,
            Arc::clone(&samples),
            Arc::clone(&last_level_emit),
            app,
        )?,
        SampleFormat::U16 => build_stream::<u16>(
            &device,
            &config,
            Arc::clone(&samples),
            Arc::clone(&last_level_emit),
            app,
        )?,
        _ => {
            return Err(AppError::from(format!(
                "Unsupported input sample format: {sample_format:?}"
            )));
        }
    };

    stream
        .play()
        .map_err(|error| AppError::from(format!("Could not start recording: {error}")))?;

    Ok(AudioRecording {
        stream: Some(stream),
        samples,
        sample_rate,
        channels,
        started_at: Utc::now(),
    })
}

impl AudioRecording {
    pub fn stop(mut self) -> AppResult<RecordedAudio> {
        self.stream.take();

        let samples = self
            .samples
            .lock()
            .map_err(|_| AppError::from("Could not read recorded audio samples"))?
            .clone();

        if samples.is_empty() {
            return Err(AppError::from("Recording did not capture any audio"));
        }

        let duration_ms = if self.channels == 0 || self.sample_rate == 0 {
            0
        } else {
            ((samples.len() as f64 / self.channels as f64) / self.sample_rate as f64 * 1000.0)
                as u64
        };

        Ok(RecordedAudio {
            bytes: encode_wav_pcm16(&samples, self.sample_rate, self.channels)?,
            duration_ms,
            file_name: "dictation.wav".to_string(),
            started_at: self.started_at,
        })
    }
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    samples: Arc<Mutex<Vec<f32>>>,
    last_level_emit: Arc<Mutex<Instant>>,
    app: tauri::AppHandle,
) -> AppResult<Stream>
where
    T: AudioSample + cpal::SizedSample,
{
    let channels = config.channels as usize;

    device
        .build_input_stream(
            config,
            move |data: &[T], _| {
                let converted: Vec<f32> = data.iter().map(AudioSample::to_f32).collect();

                if let Ok(mut target) = samples.lock() {
                    target.extend_from_slice(&converted);
                }

                emit_levels_if_due(&app, &converted, channels, &last_level_emit);
            },
            move |error| {
                eprintln!("Recording input stream error: {error}");
            },
            None,
        )
        .map_err(|error| AppError::from(format!("Could not build input stream: {error}")))
}

fn emit_levels_if_due(
    app: &tauri::AppHandle,
    samples: &[f32],
    channels: usize,
    last_level_emit: &Arc<Mutex<Instant>>,
) {
    let Ok(mut last_emit) = last_level_emit.lock() else {
        return;
    };

    if last_emit.elapsed() < LEVEL_EMIT_INTERVAL {
        return;
    }

    *last_emit = Instant::now();
    overlay::emit_mic_levels(app, calculate_channel_levels(samples, channels));
}

fn calculate_channel_levels(samples: &[f32], channels: usize) -> Vec<f32> {
    if samples.is_empty() || channels == 0 {
        return vec![0.0];
    }

    let mut sums = vec![0.0; channels];
    let mut counts = vec![0_u32; channels];

    for (index, sample) in samples.iter().enumerate() {
        let channel = index % channels;
        sums[channel] += sample.abs();
        counts[channel] += 1;
    }

    sums.into_iter()
        .zip(counts)
        .map(|(sum, count)| {
            if count == 0 {
                0.0
            } else {
                (sum / count as f32).clamp(0.0, 1.0)
            }
        })
        .collect()
}

fn encode_wav_pcm16(samples: &[f32], sample_rate: u32, channels: u16) -> AppResult<Vec<u8>> {
    let spec = hound::WavSpec {
        bits_per_sample: 16,
        channels,
        sample_format: hound::SampleFormat::Int,
        sample_rate,
    };
    let mut buffer = Cursor::new(Vec::new());

    {
        let mut writer = hound::WavWriter::new(&mut buffer, spec)
            .map_err(|error| AppError::from(format!("Could not create WAV writer: {error}")))?;

        for sample in samples {
            let pcm = (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16;

            writer
                .write_sample(pcm)
                .map_err(|error| AppError::from(format!("Could not write WAV sample: {error}")))?;
        }

        writer.finalize().map_err(|error| {
            AppError::from(format!("Could not finalize WAV recording: {error}"))
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
