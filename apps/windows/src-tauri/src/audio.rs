use chrono::{DateTime, Utc};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, SampleFormat, Stream, StreamConfig,
};
use serde::{Deserialize, Serialize};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};
use thiserror::Error;
use uuid::Uuid;

pub const TARGET_SAMPLE_RATE: u32 = 16_000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AudioInputDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
    pub channels: Vec<u16>,
    pub min_sample_rate: Option<u32>,
    pub max_sample_rate: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct AudioLevel {
    pub peak: f32,
    pub rms: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RecordingSnapshot {
    pub id: Uuid,
    pub started_at: DateTime<Utc>,
    pub duration_ms: u64,
    pub sample_rate: u32,
    pub level: AudioLevel,
}

#[derive(Debug, Clone)]
pub struct AudioBuffer {
    pub sample_rate: u32,
    pub channels: u16,
    pub samples: Vec<f32>,
}

impl AudioBuffer {
    pub fn duration_ms(&self) -> u64 {
        if self.sample_rate == 0 || self.channels == 0 {
            return 0;
        }

        let frames = self.samples.len() / self.channels as usize;
        ((frames as f64 / self.sample_rate as f64) * 1000.0).round() as u64
    }

    pub fn to_mono_f32_16khz(&self) -> Vec<f32> {
        resample_mono(
            &self.samples,
            self.sample_rate,
            self.channels,
            TARGET_SAMPLE_RATE,
        )
    }

    pub fn to_pcm16_16khz(&self) -> Vec<i16> {
        self.to_mono_f32_16khz()
            .into_iter()
            .map(|sample| {
                let clamped = sample.clamp(-1.0, 1.0);
                (clamped * i16::MAX as f32) as i16
            })
            .collect()
    }
}

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("failed to enumerate input devices: {0}")]
    Enumerate(#[source] cpal::DevicesError),
    #[error("failed to read input device name: {0}")]
    DeviceName(#[source] cpal::DeviceNameError),
    #[error("no input device is available")]
    NoInputDevice,
    #[error("input device not found: {0}")]
    DeviceNotFound(String),
    #[error("failed to read default input config: {0}")]
    DefaultConfig(#[source] cpal::DefaultStreamConfigError),
    #[error("failed to inspect supported input config: {0}")]
    SupportedConfig(#[source] cpal::SupportedStreamConfigsError),
    #[error("input sample format is not supported: {0:?}")]
    UnsupportedSampleFormat(SampleFormat),
    #[error("failed to build input stream: {0}")]
    BuildStream(#[source] cpal::BuildStreamError),
    #[error("failed to start input stream: {0}")]
    PlayStream(#[source] cpal::PlayStreamError),
    #[error("a recording is already active")]
    AlreadyRecording,
    #[error("there is no active recording")]
    NotRecording,
    #[error("audio state lock was poisoned")]
    LockPoisoned,
}

struct ActiveRecording {
    id: Uuid,
    started_at: DateTime<Utc>,
    started_instant: Instant,
    sample_rate: u32,
    channels: u16,
    samples: Arc<Mutex<Vec<f32>>>,
    level: Arc<Mutex<AudioLevel>>,
    _stream: Stream,
}

#[derive(Default)]
pub struct AudioCaptureService {
    active: Mutex<Option<ActiveRecording>>,
}

impl AudioCaptureService {
    pub fn list_input_devices(&self) -> Result<Vec<AudioInputDevice>, AudioError> {
        let host = cpal::default_host();
        let default_name = host
            .default_input_device()
            .and_then(|device| device.name().ok());
        let mut devices = Vec::new();

        for device in host.input_devices().map_err(AudioError::Enumerate)? {
            let name = device.name().map_err(AudioError::DeviceName)?;
            let supported = supported_summary(&device)?;
            devices.push(AudioInputDevice {
                id: name.clone(),
                name: name.clone(),
                is_default: default_name.as_deref() == Some(name.as_str()),
                channels: supported.channels,
                min_sample_rate: supported.min_sample_rate,
                max_sample_rate: supported.max_sample_rate,
            });
        }

        Ok(devices)
    }

    pub fn start(&self, input_device_id: Option<&str>) -> Result<RecordingSnapshot, AudioError> {
        let mut active = self.active.lock().map_err(|_| AudioError::LockPoisoned)?;
        if active.is_some() {
            return Err(AudioError::AlreadyRecording);
        }

        let host = cpal::default_host();
        let device = select_input_device(&host, input_device_id)?;
        let supported_config = device
            .default_input_config()
            .map_err(AudioError::DefaultConfig)?;
        let sample_format = supported_config.sample_format();
        let config = supported_config.config();
        let sample_rate = config.sample_rate.0;
        let channels = config.channels;
        let samples = Arc::new(Mutex::new(Vec::new()));
        let level = Arc::new(Mutex::new(AudioLevel::default()));
        let err_fn = |error| tracing::error!(%error, "input stream error");

        let stream = match sample_format {
            SampleFormat::F32 => build_stream::<f32>(&device, &config, &samples, &level, err_fn),
            SampleFormat::I16 => build_stream::<i16>(&device, &config, &samples, &level, err_fn),
            SampleFormat::U16 => build_stream::<u16>(&device, &config, &samples, &level, err_fn),
            other => Err(AudioError::UnsupportedSampleFormat(other)),
        }?;

        stream.play().map_err(AudioError::PlayStream)?;

        let recording = ActiveRecording {
            id: Uuid::new_v4(),
            started_at: Utc::now(),
            started_instant: Instant::now(),
            sample_rate,
            channels: 1,
            samples,
            level,
            _stream: stream,
        };
        let snapshot = recording.snapshot()?;
        *active = Some(recording);

        Ok(snapshot)
    }

    pub fn snapshot(&self) -> Result<Option<RecordingSnapshot>, AudioError> {
        let active = self.active.lock().map_err(|_| AudioError::LockPoisoned)?;
        active.as_ref().map(ActiveRecording::snapshot).transpose()
    }

    pub fn stop(&self) -> Result<AudioBuffer, AudioError> {
        let mut active = self.active.lock().map_err(|_| AudioError::LockPoisoned)?;
        let recording = active.take().ok_or(AudioError::NotRecording)?;
        let samples = recording
            .samples
            .lock()
            .map_err(|_| AudioError::LockPoisoned)?
            .clone();

        Ok(AudioBuffer {
            sample_rate: recording.sample_rate,
            channels: recording.channels,
            samples,
        })
    }

    pub fn cancel(&self) -> Result<(), AudioError> {
        let mut active = self.active.lock().map_err(|_| AudioError::LockPoisoned)?;
        if active.take().is_none() {
            return Err(AudioError::NotRecording);
        }
        Ok(())
    }
}

impl ActiveRecording {
    fn snapshot(&self) -> Result<RecordingSnapshot, AudioError> {
        let level = *self.level.lock().map_err(|_| AudioError::LockPoisoned)?;

        Ok(RecordingSnapshot {
            id: self.id,
            started_at: self.started_at,
            duration_ms: self.started_instant.elapsed().as_millis() as u64,
            sample_rate: self.sample_rate,
            level,
        })
    }
}

#[derive(Default)]
struct SupportedSummary {
    channels: Vec<u16>,
    min_sample_rate: Option<u32>,
    max_sample_rate: Option<u32>,
}

fn supported_summary(device: &Device) -> Result<SupportedSummary, AudioError> {
    let mut summary = SupportedSummary::default();

    for config in device
        .supported_input_configs()
        .map_err(AudioError::SupportedConfig)?
    {
        if !summary.channels.contains(&config.channels()) {
            summary.channels.push(config.channels());
        }

        let min_rate = config.min_sample_rate().0;
        let max_rate = config.max_sample_rate().0;
        summary.min_sample_rate = Some(
            summary
                .min_sample_rate
                .map_or(min_rate, |v| v.min(min_rate)),
        );
        summary.max_sample_rate = Some(
            summary
                .max_sample_rate
                .map_or(max_rate, |v| v.max(max_rate)),
        );
    }

    summary.channels.sort_unstable();
    Ok(summary)
}

fn select_input_device(
    host: &cpal::Host,
    input_device_id: Option<&str>,
) -> Result<Device, AudioError> {
    if let Some(id) = input_device_id.filter(|id| !id.trim().is_empty()) {
        for device in host.input_devices().map_err(AudioError::Enumerate)? {
            let name = device.name().map_err(AudioError::DeviceName)?;
            if name == id {
                return Ok(device);
            }
        }
        return Err(AudioError::DeviceNotFound(id.to_string()));
    }

    host.default_input_device().ok_or(AudioError::NoInputDevice)
}

fn build_stream<T>(
    device: &Device,
    config: &StreamConfig,
    samples: &Arc<Mutex<Vec<f32>>>,
    level: &Arc<Mutex<AudioLevel>>,
    err_fn: impl Fn(cpal::StreamError) + Send + 'static,
) -> Result<Stream, AudioError>
where
    T: InputSample,
{
    let samples = Arc::clone(samples);
    let level = Arc::clone(level);
    let channels = config.channels as usize;

    device
        .build_input_stream(
            config,
            move |input: &[T], _| {
                collect_input(input, channels, &samples, &level);
            },
            err_fn,
            None,
        )
        .map_err(AudioError::BuildStream)
}

trait InputSample: Copy + cpal::SizedSample + Send + 'static {
    fn to_f32(self) -> f32;
}

impl InputSample for f32 {
    fn to_f32(self) -> f32 {
        self
    }
}

impl InputSample for i16 {
    fn to_f32(self) -> f32 {
        self as f32 / i16::MAX as f32
    }
}

impl InputSample for u16 {
    fn to_f32(self) -> f32 {
        (self as f32 / u16::MAX as f32) * 2.0 - 1.0
    }
}

fn collect_input<T: InputSample>(
    input: &[T],
    channels: usize,
    samples: &Arc<Mutex<Vec<f32>>>,
    level: &Arc<Mutex<AudioLevel>>,
) {
    if channels == 0 {
        return;
    }

    let mut converted = Vec::with_capacity(input.len() / channels);
    let mut sum_squares = 0.0_f32;
    let mut peak = 0.0_f32;

    for frame in input.chunks(channels) {
        let mono = frame.iter().map(|sample| (*sample).to_f32()).sum::<f32>() / frame.len() as f32;
        let mono = mono.clamp(-1.0, 1.0);
        peak = peak.max(mono.abs());
        sum_squares += mono * mono;
        converted.push(mono);
    }

    if let Ok(mut guard) = samples.lock() {
        guard.extend(converted.iter().copied());
    }

    if let Ok(mut guard) = level.lock() {
        let rms = if converted.is_empty() {
            0.0
        } else {
            (sum_squares / converted.len() as f32).sqrt()
        };
        *guard = AudioLevel { peak, rms };
    }
}

pub fn resample_mono(
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
    target_rate: u32,
) -> Vec<f32> {
    if samples.is_empty() || sample_rate == 0 || channels == 0 || target_rate == 0 {
        return Vec::new();
    }

    let channels = channels as usize;
    let source_frames = samples.len() / channels;
    if source_frames == 0 {
        return Vec::new();
    }

    let mut mono = Vec::with_capacity(source_frames);
    for frame in samples.chunks(channels).take(source_frames) {
        mono.push(frame.iter().copied().sum::<f32>() / frame.len() as f32);
    }

    if sample_rate == target_rate {
        return mono;
    }

    let output_frames =
        ((source_frames as f64 / sample_rate as f64) * target_rate as f64).round() as usize;
    let mut output = Vec::with_capacity(output_frames);

    for index in 0..output_frames {
        let source_pos = index as f64 * sample_rate as f64 / target_rate as f64;
        let left = source_pos.floor() as usize;
        let right = (left + 1).min(mono.len() - 1);
        let fraction = (source_pos - left as f64) as f32;
        let value = mono[left] * (1.0 - fraction) + mono[right] * fraction;
        output.push(value);
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resample_downmixes_to_target_rate() {
        let mut stereo_48k = Vec::with_capacity(96_000);
        for _ in 0..48_000 {
            stereo_48k.push(0.25_f32);
            stereo_48k.push(0.75_f32);
        }
        let resampled = resample_mono(&stereo_48k, 48_000, 2, TARGET_SAMPLE_RATE);

        assert_eq!(resampled.len(), TARGET_SAMPLE_RATE as usize);
        assert!(resampled.iter().all(|sample| (*sample - 0.5).abs() < 0.001));
    }

    #[test]
    fn audio_buffer_duration_uses_source_rate_and_channels() {
        let buffer = AudioBuffer {
            sample_rate: 48_000,
            channels: 2,
            samples: vec![0.0; 96_000],
        };

        assert_eq!(buffer.duration_ms(), 1000);
    }
}
