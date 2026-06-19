use std::sync::{
    Arc,
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
        if capabilities.sample_rate_hz == 0 || capabilities.max_channel_count == 0 {
            return Err(AudioOutputError::InvalidCapabilities {
                reason: "native output requires nonzero sample rate and channel count".to_owned(),
            });
        }

        let sample_format = self.default_config.sample_format();
        let config: cpal::StreamConfig = self.default_config.clone().into();
        let presented_result_count = Arc::new(AtomicU64::new(0));
        let callback_count = Arc::clone(&presented_result_count);
        let error_callback = |error| eprintln!("native audio output stream diagnostic: {error}");

        let stream = match sample_format {
            cpal::SampleFormat::I8 => {
                build_silent_stream::<i8>(&self.device, config, callback_count, error_callback)
            }
            cpal::SampleFormat::I16 => {
                build_silent_stream::<i16>(&self.device, config, callback_count, error_callback)
            }
            cpal::SampleFormat::I24 => build_silent_stream::<cpal::I24>(
                &self.device,
                config,
                callback_count,
                error_callback,
            ),
            cpal::SampleFormat::I32 => {
                build_silent_stream::<i32>(&self.device, config, callback_count, error_callback)
            }
            cpal::SampleFormat::I64 => {
                build_silent_stream::<i64>(&self.device, config, callback_count, error_callback)
            }
            cpal::SampleFormat::U8 => {
                build_silent_stream::<u8>(&self.device, config, callback_count, error_callback)
            }
            cpal::SampleFormat::U16 => {
                build_silent_stream::<u16>(&self.device, config, callback_count, error_callback)
            }
            cpal::SampleFormat::U24 => build_silent_stream::<cpal::U24>(
                &self.device,
                config,
                callback_count,
                error_callback,
            ),
            cpal::SampleFormat::U32 => {
                build_silent_stream::<u32>(&self.device, config, callback_count, error_callback)
            }
            cpal::SampleFormat::U64 => {
                build_silent_stream::<u64>(&self.device, config, callback_count, error_callback)
            }
            cpal::SampleFormat::F32 => {
                build_silent_stream::<f32>(&self.device, config, callback_count, error_callback)
            }
            cpal::SampleFormat::F64 => {
                build_silent_stream::<f64>(&self.device, config, callback_count, error_callback)
            }
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
        })
    }
}

pub struct CpalAudioOutputSink {
    stream: cpal::Stream,
    presented_result_count: Arc<AtomicU64>,
}

impl AudioOutputSink for CpalAudioOutputSink {
    fn present(&mut self, result: &AudioBufferResult) -> Result<(), AudioOutputError> {
        if result.presented {
            self.stream
                .play()
                .map_err(|error| AudioOutputError::InvalidCapabilities {
                    reason: format!("failed to start native output stream: {error}"),
                })?;
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

fn build_silent_stream<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    callback_count: Arc<AtomicU64>,
    error_callback: impl FnMut(cpal::Error) + Send + 'static,
) -> Result<cpal::Stream, cpal::Error>
where
    T: cpal::Sample + cpal::SizedSample,
{
    device.build_output_stream(
        config,
        move |data: &mut [T], _| {
            for sample in data.iter_mut() {
                *sample = cpal::Sample::EQUILIBRIUM;
            }
            callback_count.fetch_add(1, Ordering::Relaxed);
        },
        error_callback,
        None,
    )
}
