use std::collections::VecDeque;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

use audio_engine::{
    AudioBufferResult, AudioOutputCapabilities, AudioOutputDevice, AudioOutputError,
    AudioOutputSink,
};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::{
    DesktopAudioDeviceSummary, DesktopAudioOutputBackend, DesktopAudioOutputBackendCapabilities,
    DesktopAudioOutputCapabilityStatus, DesktopAudioOutputDiagnostic,
};

#[derive(Debug, Clone)]
pub struct CpalAudioOutputDevice {
    device: cpal::Device,
    summary: DesktopAudioDeviceSummary,
    default_config: cpal::SupportedStreamConfig,
}

impl CpalAudioOutputDevice {
    pub fn default_output() -> Result<Self, DesktopAudioOutputDiagnostic> {
        let host = cpal::default_host();
        let Some(device) = host.default_output_device() else {
            return Err(native_diagnostic("no output device available", false));
        };
        let default_config = device.default_output_config().map_err(|error| {
            native_diagnostic(
                &format!("default output config unavailable: {error}"),
                false,
            )
        })?;
        let summary = summarize_device(&device, true).unwrap_or_else(|message| {
            fallback_summary(
                "cpal-default-output",
                &format!("Default native audio output ({message})"),
                default_config.sample_rate(),
                default_config.channels(),
                true,
            )
        });

        Ok(Self {
            device,
            summary,
            default_config,
        })
    }

    pub fn summary(&self) -> &DesktopAudioDeviceSummary {
        &self.summary
    }
}

impl AudioOutputDevice for CpalAudioOutputDevice {
    type Stream = CpalAudioOutputSink;

    fn capabilities(&self) -> AudioOutputCapabilities {
        AudioOutputCapabilities {
            device_id: self.summary.device_id.clone(),
            display_name: self.summary.safe_label.clone(),
            sample_rate_hz: self.default_config.sample_rate(),
            max_channel_count: self.default_config.channels(),
            max_frame_count: 2_400,
            mock: false,
        }
    }

    fn open_stream(
        &self,
        capabilities: &AudioOutputCapabilities,
    ) -> Result<Self::Stream, AudioOutputError> {
        validate_native_output_capabilities(capabilities, &self.capabilities())?;

        let sample_format = self.default_config.sample_format();
        let config: cpal::StreamConfig = self.default_config.clone().into();
        let presented_result_count = Arc::new(AtomicU64::new(0));
        let queued_samples = Arc::new(Mutex::new(VecDeque::new()));
        let underrun_sample_count = Arc::new(AtomicU64::new(0));
        let error_callback = |error| eprintln!("native audio output stream diagnostic: {error}");

        let stream = match sample_format {
            cpal::SampleFormat::I16 => build_queued_stream::<I16Writer>(
                &self.device,
                config,
                queued_samples.clone(),
                underrun_sample_count.clone(),
                error_callback,
            ),
            cpal::SampleFormat::U16 => build_queued_stream::<U16Writer>(
                &self.device,
                config,
                queued_samples.clone(),
                underrun_sample_count.clone(),
                error_callback,
            ),
            cpal::SampleFormat::F32 => build_queued_stream::<F32Writer>(
                &self.device,
                config,
                queued_samples.clone(),
                underrun_sample_count.clone(),
                error_callback,
            ),
            format => {
                return Err(AudioOutputError::InvalidCapabilities {
                    reason: format!("unsupported native output sample format: {format:?}"),
                });
            }
        }
        .map_err(|error| AudioOutputError::InvalidCapabilities {
            reason: format!("failed to open native output stream for {sample_format:?}: {error}"),
        })?;

        Ok(CpalAudioOutputSink {
            stream,
            presented_result_count,
            queued_samples,
            underrun_sample_count,
        })
    }
}

fn validate_native_output_capabilities(
    requested: &AudioOutputCapabilities,
    available: &AudioOutputCapabilities,
) -> Result<(), AudioOutputError> {
    if requested.sample_rate_hz == 0
        || requested.max_channel_count == 0
        || requested.max_frame_count == 0
    {
        return Err(AudioOutputError::InvalidCapabilities {
            reason: "native output requires nonzero sample rate, channel count, and frame count"
                .to_owned(),
        });
    }
    if requested.device_id != available.device_id {
        return Err(AudioOutputError::InvalidCapabilities {
            reason: "native output device does not match requested device".to_owned(),
        });
    }
    if requested.sample_rate_hz != available.sample_rate_hz {
        return Err(AudioOutputError::InvalidCapabilities {
            reason: format!(
                "native output sample rate {} Hz is unavailable; default device is {} Hz",
                requested.sample_rate_hz, available.sample_rate_hz
            ),
        });
    }
    if requested.max_channel_count > available.max_channel_count {
        return Err(AudioOutputError::InvalidCapabilities {
            reason: format!(
                "native output channel count {} exceeds available {}",
                requested.max_channel_count, available.max_channel_count
            ),
        });
    }
    if requested.max_frame_count > available.max_frame_count {
        return Err(AudioOutputError::InvalidCapabilities {
            reason: format!(
                "native output frame count {} exceeds available {}",
                requested.max_frame_count, available.max_frame_count
            ),
        });
    }
    Ok(())
}

pub struct CpalAudioOutputSink {
    stream: cpal::Stream,
    queued_samples: Arc<Mutex<VecDeque<f32>>>,
    presented_result_count: Arc<AtomicU64>,
    underrun_sample_count: Arc<AtomicU64>,
}

impl CpalAudioOutputSink {
    pub fn enqueue_f32_interleaved(&mut self, samples: &[f32]) -> Result<(), AudioOutputError> {
        if samples.is_empty() {
            return Err(AudioOutputError::InvalidCapabilities {
                reason: "native output requires non-empty PCM samples".to_owned(),
            });
        }
        let mut queue =
            self.queued_samples
                .lock()
                .map_err(|_| AudioOutputError::InvalidCapabilities {
                    reason: "native output PCM queue lock failed".to_owned(),
                })?;
        let max_samples = 48_000usize * 2 * 8;
        for sample in samples {
            if queue.len() >= max_samples {
                let _ = queue.pop_front();
            }
            queue.push_back(sample.clamp(-1.0, 1.0));
        }
        Ok(())
    }

    pub fn start(&self) -> Result<(), AudioOutputError> {
        self.stream
            .play()
            .map_err(|error| AudioOutputError::InvalidCapabilities {
                reason: format!("failed to start native output stream: {error}"),
            })
    }

    pub fn queued_sample_count(&self) -> usize {
        self.queued_samples
            .lock()
            .map(|queue| queue.len())
            .unwrap_or_default()
    }

    pub fn underrun_sample_count(&self) -> u64 {
        self.underrun_sample_count.load(Ordering::Relaxed)
    }
}

impl AudioOutputSink for CpalAudioOutputSink {
    fn present(&mut self, result: &AudioBufferResult) -> Result<(), AudioOutputError> {
        if result.presented {
            self.start()?;
        }
        self.presented_result_count.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    fn presented_result_count(&self) -> u64 {
        self.presented_result_count.load(Ordering::Relaxed)
    }
}

pub(crate) fn target_platform_api() -> Option<String> {
    if cfg!(target_os = "macos") {
        Some("CoreAudio".to_owned())
    } else if cfg!(windows) {
        Some("WASAPI".to_owned())
    } else {
        None
    }
}

pub(crate) fn probe_cpal_output_capabilities(
    diagnostics: &mut Vec<DesktopAudioOutputDiagnostic>,
) -> DesktopAudioOutputBackendCapabilities {
    if !cfg!(any(target_os = "macos", windows)) {
        return DesktopAudioOutputBackendCapabilities {
            backend: DesktopAudioOutputBackend::Native,
            platform_api: target_platform_api(),
            status: DesktopAudioOutputCapabilityStatus::Unsupported,
            default_for_ci: false,
            device_count: 0,
            devices: Vec::new(),
            fallback_reason: Some("unsupported platform".to_owned()),
            diagnostic: Some(
                "Native audio output proof targets macOS CoreAudio and Windows WASAPI only."
                    .to_owned(),
            ),
        };
    }

    let host = cpal::default_host();
    let devices = match host.output_devices() {
        Ok(devices) => devices
            .enumerate()
            .filter_map(|(index, device)| summarize_device(&device, index == 0).ok())
            .collect::<Vec<_>>(),
        Err(error) => {
            diagnostics.push(native_diagnostic(
                &format!("native output device enumeration failed: {error}"),
                false,
            ));
            Vec::new()
        }
    };
    let status = if devices.is_empty() {
        DesktopAudioOutputCapabilityStatus::Warning
    } else {
        DesktopAudioOutputCapabilityStatus::Ready
    };

    DesktopAudioOutputBackendCapabilities {
        backend: DesktopAudioOutputBackend::Native,
        platform_api: target_platform_api(),
        status,
        default_for_ci: false,
        device_count: devices.len(),
        devices,
        fallback_reason: if status == DesktopAudioOutputCapabilityStatus::Ready {
            None
        } else {
            Some("no output device available".to_owned())
        },
        diagnostic: Some(match target_platform_api().as_deref() {
            Some("CoreAudio") => {
                "macOS native output is represented through CPAL CoreAudio.".to_owned()
            }
            Some("WASAPI") => {
                "Windows native output is represented through CPAL WASAPI.".to_owned()
            }
            _ => "Native output is not a supported Phase 15 target on this platform.".to_owned(),
        }),
    }
}

pub(crate) fn open_native_audio_probe() -> DesktopAudioOutputDiagnostic {
    let Ok(device) = CpalAudioOutputDevice::default_output() else {
        return native_diagnostic("no output device available", false);
    };
    match device.open_stream(&device.capabilities()) {
        Ok(_) => native_diagnostic("native audio output stream opened successfully", true),
        Err(error) => {
            native_diagnostic(&format!("native audio output unavailable: {error}"), false)
        }
    }
}

fn summarize_device(
    device: &cpal::Device,
    default_device: bool,
) -> Result<DesktopAudioDeviceSummary, String> {
    let name = device.to_string();
    let configs = device
        .supported_output_configs()
        .map_err(|error| error.to_string())?;
    let mut sample_rates = Vec::new();
    let mut max_channel_count = 0;

    for config in configs {
        let min = config.min_sample_rate();
        let max = config.max_sample_rate();
        for rate in [44_100, 48_000, 96_000] {
            if rate >= min && rate <= max && !sample_rates.contains(&rate) {
                sample_rates.push(rate);
            }
        }
        if !sample_rates.contains(&max) {
            sample_rates.push(max);
        }
        max_channel_count = max_channel_count.max(config.channels());
    }
    sample_rates.sort_unstable();
    sample_rates.dedup();

    Ok(DesktopAudioDeviceSummary {
        device_id: safe_device_id(&name),
        safe_label: safe_device_label(&name),
        sample_rates_hz: sample_rates,
        max_channel_count,
        default_device,
        backend: DesktopAudioOutputBackend::Native,
    })
}

fn fallback_summary(
    device_id: &str,
    safe_label: &str,
    sample_rate_hz: u32,
    max_channel_count: u16,
    default_device: bool,
) -> DesktopAudioDeviceSummary {
    DesktopAudioDeviceSummary {
        device_id: device_id.to_owned(),
        safe_label: safe_label.to_owned(),
        sample_rates_hz: vec![sample_rate_hz],
        max_channel_count,
        default_device,
        backend: DesktopAudioOutputBackend::Native,
    }
}

fn safe_device_id(name: &str) -> String {
    let mut id = name
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .take(24)
        .collect::<String>();
    if id.is_empty() {
        id = "native-audio-output".to_owned();
    }
    id
}

fn safe_device_label(name: &str) -> String {
    name.chars().take(80).collect()
}

fn native_diagnostic(message: &str, ready: bool) -> DesktopAudioOutputDiagnostic {
    DesktopAudioOutputDiagnostic {
        backend: DesktopAudioOutputBackend::Native,
        platform_api: target_platform_api(),
        message: message.to_owned(),
        skipped: false,
        ready,
    }
}

trait QueuedSampleWriter {
    type Sample: cpal::Sample + cpal::SizedSample;

    fn equilibrium() -> Self::Sample;
    fn from_f32(value: f32) -> Self::Sample;
}

struct F32Writer;
struct I16Writer;
struct U16Writer;

impl QueuedSampleWriter for F32Writer {
    type Sample = f32;

    fn equilibrium() -> Self::Sample {
        0.0
    }

    fn from_f32(value: f32) -> Self::Sample {
        value.clamp(-1.0, 1.0)
    }
}

impl QueuedSampleWriter for I16Writer {
    type Sample = i16;

    fn equilibrium() -> Self::Sample {
        0
    }

    fn from_f32(value: f32) -> Self::Sample {
        (value.clamp(-1.0, 1.0) * f32::from(i16::MAX)).round() as i16
    }
}

impl QueuedSampleWriter for U16Writer {
    type Sample = u16;

    fn equilibrium() -> Self::Sample {
        u16::MAX / 2
    }

    fn from_f32(value: f32) -> Self::Sample {
        (((value.clamp(-1.0, 1.0) + 1.0) * 0.5) * f32::from(u16::MAX)).round() as u16
    }
}

fn build_queued_stream<W>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    queued_samples: Arc<Mutex<VecDeque<f32>>>,
    underrun_sample_count: Arc<AtomicU64>,
    error_callback: impl FnMut(cpal::Error) + Send + 'static,
) -> Result<cpal::Stream, cpal::Error>
where
    W: QueuedSampleWriter + Send + 'static,
{
    device.build_output_stream(
        config,
        move |data: &mut [W::Sample], _| {
            let mut underruns = 0u64;
            let mut queue = queued_samples.lock().ok();
            for sample in data.iter_mut() {
                let value = queue.as_mut().and_then(|queue| queue.pop_front());
                *sample = value.map(W::from_f32).unwrap_or_else(|| {
                    underruns = underruns.saturating_add(1);
                    W::equilibrium()
                });
            }
            if underruns > 0 {
                underrun_sample_count.fetch_add(underruns, Ordering::Relaxed);
            }
        },
        error_callback,
        None,
    )
}

#[cfg(test)]
mod tests {
    use audio_engine::AudioOutputCapabilities;

    use super::validate_native_output_capabilities;

    fn capabilities() -> AudioOutputCapabilities {
        AudioOutputCapabilities {
            device_id: "native-device".to_owned(),
            display_name: "Native Device".to_owned(),
            sample_rate_hz: 48_000,
            max_channel_count: 2,
            max_frame_count: 2_400,
            mock: false,
        }
    }

    #[test]
    fn native_output_capability_validation_rejects_mismatched_requests() {
        let available = capabilities();
        assert!(validate_native_output_capabilities(&available, &available).is_ok());

        let mut wrong_rate = available.clone();
        wrong_rate.sample_rate_hz = 44_100;
        assert!(validate_native_output_capabilities(&wrong_rate, &available).is_err());

        let mut too_many_channels = available.clone();
        too_many_channels.max_channel_count = 8;
        assert!(validate_native_output_capabilities(&too_many_channels, &available).is_err());

        let mut too_many_frames = available.clone();
        too_many_frames.max_frame_count = 4_800;
        assert!(validate_native_output_capabilities(&too_many_frames, &available).is_err());

        let mut wrong_device = available.clone();
        wrong_device.device_id = "other-device".to_owned();
        assert!(validate_native_output_capabilities(&wrong_device, &available).is_err());
    }
}
