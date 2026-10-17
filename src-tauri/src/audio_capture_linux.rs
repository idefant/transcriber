//! Захват PipeWire через штатный клиент. Остановка закрывает источник, а не
//! оставляет молча работающий микрофон между сессиями или во время паузы.

use std::{
    fmt,
    io::Read,
    process::{Child, Command, Stdio},
    sync::{mpsc, Arc, Mutex},
    thread::{self, JoinHandle},
    time::Duration,
};

/** PipeWire сам преобразует частоту и раскладку устройства в этот формат. */
#[derive(Clone, Copy)]
pub struct CaptureFormat {
    pub sample_rate: u32,
    pub channels: u16,
}

/** Отказ запуска или преждевременное завершение захвата. */
pub enum CaptureError {
    NoDefaultDevice,
    #[allow(dead_code)]
    UnsupportedSampleFormat(String),
    PipeWire(String),
}

impl fmt::Display for CaptureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoDefaultDevice => formatter.write_str("no default PipeWire input device"),
            Self::UnsupportedSampleFormat(format) => {
                write!(formatter, "unsupported format: {format}")
            }
            Self::PipeWire(error) => write!(formatter, "PipeWire capture: {error}"),
        }
    }
}

struct RunningCapture {
    child: Child,
    reader: JoinHandle<Result<(), String>>,
}

/** Подготовленный захват не держит микрофон открытым. Процесс создаётся на start. */
pub struct CaptureStream {
    device_id: String,
    samples: Arc<Mutex<dyn FnMut(&[f32]) + Send>>,
    running: Mutex<Option<RunningCapture>>,
}

impl CaptureStream {
    /** Частота в герцах; сэмплы f32, один канал. */
    pub fn format(&self) -> CaptureFormat {
        CaptureFormat {
            sample_rate: 48_000,
            channels: 1,
        }
    }

    /** Имя источника PipeWire, зафиксированное при подготовке. */
    pub fn device_id(&self) -> &str {
        &self.device_id
    }

    /** Речевая категория WASAPI не применяется на Linux. */
    pub fn speech_category_applied(&self) -> bool {
        false
    }

    /** Возвращается после первого блока данных или ошибки; ожидание ограничено. */
    pub fn start(&self) -> Result<(), CaptureError> {
        let mut running = self
            .running
            .lock()
            .map_err(|e| CaptureError::PipeWire(e.to_string()))?;
        if running.is_some() {
            return Ok(());
        }
        let mut child = Command::new("pw-record")
            .args([
                "--raw",
                "--format=f32",
                "--rate=48000",
                "--channels=1",
                "--latency=20ms",
                "--media-role=Communication",
                "--target",
                &self.device_id,
                "-",
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| CaptureError::PipeWire(e.to_string()))?;
        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| CaptureError::PipeWire("missing stdout".into()))?;
        let samples = Arc::clone(&self.samples);
        let (ready, first_block) = mpsc::channel();
        let reader = thread::spawn(move || {
            let mut bytes = [0u8; 4096];
            let mut pending = Vec::with_capacity(4100);
            let mut announced = false;
            loop {
                let count = stdout.read(&mut bytes).map_err(|e| e.to_string())?;
                if count == 0 {
                    return Err("pw-record exited or microphone disconnected".into());
                }
                pending.extend_from_slice(&bytes[..count]);
                let complete = pending.len() / 4 * 4;
                let data: Vec<f32> = pending[..complete]
                    .chunks_exact(4)
                    .map(|b| f32::from_ne_bytes([b[0], b[1], b[2], b[3]]))
                    .collect();
                pending.drain(..complete);
                if data.is_empty() {
                    continue;
                }
                samples.lock().map_err(|e| e.to_string())?(&data);
                if !announced {
                    let _ = ready.send(());
                    announced = true;
                }
            }
        });
        if first_block.recv_timeout(Duration::from_secs(3)).is_err() {
            let _ = child.kill();
            let _ = child.wait();
            let detail = reader
                .join()
                .ok()
                .and_then(Result::err)
                .unwrap_or_else(|| "no samples within 3 seconds".into());
            return Err(CaptureError::PipeWire(detail));
        }
        *running = Some(RunningCapture { child, reader });
        Ok(())
    }

    /** Закрывает клиент и дожидается чтения хвоста, чтобы кадры не попадали в следующую сессию. */
    pub fn stop(&self) -> Result<(), CaptureError> {
        let mut running = self
            .running
            .lock()
            .map_err(|e| CaptureError::PipeWire(e.to_string()))?;
        let Some(mut capture) = running.take() else {
            return Ok(());
        };
        let already_exited = capture
            .child
            .try_wait()
            .map_err(|e| CaptureError::PipeWire(e.to_string()))?
            .is_some();
        let reader_finished = capture.reader.is_finished();
        let _ = capture.child.kill();
        let _ = capture.child.wait();
        let result = capture.reader.join();
        if already_exited || reader_finished {
            return Err(CaptureError::PipeWire(
                result
                    .ok()
                    .and_then(Result::err)
                    .unwrap_or_else(|| "capture thread stopped".into()),
            ));
        }
        Ok(())
    }
}

impl Drop for CaptureStream {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

/** Читает имя текущего default source; номер объекта не переживает перезапуск PipeWire. */
pub fn default_capture_device_id() -> Option<String> {
    let result = Command::new("wpctl")
        .args(["inspect", "@DEFAULT_AUDIO_SOURCE@"])
        .output()
        .ok()?;
    if !result.status.success() {
        return None;
    }
    String::from_utf8_lossy(&result.stdout)
        .lines()
        .find_map(|line| {
            let (key, value) = line
                .trim()
                .trim_start_matches('*')
                .trim()
                .split_once(" = ")?;
            (key == "node.name").then(|| value.trim_matches('"').to_owned())
        })
}

/** Подготавливает источник, не начиная запись и не захватывая приватные данные. */
pub fn open_capture_stream<F>(on_samples: F) -> Result<CaptureStream, CaptureError>
where
    F: FnMut(&[f32]) + Send + 'static,
{
    Ok(CaptureStream {
        device_id: default_capture_device_id().ok_or(CaptureError::NoDefaultDevice)?,
        samples: Arc::new(Mutex::new(on_samples)),
        running: Mutex::new(None),
    })
}
