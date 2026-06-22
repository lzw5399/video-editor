//! Desktop audio output boundary.

mod cpal_output;
mod mock_output;

use serde::{Deserialize, Serialize};

pub use audio_engine::MockAudioOutputDevice;
#[rustfmt::skip]
pub use cpal_output::CpalAudioOutputSink as
    CpalAudioOutputStream;
pub use cpal_output::{CpalAudioOutputDevice, CpalAudioOutputQueue, CpalAudioOutputSink};
pub use mock_output::create_desktop_audio_output;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DesktopAudioOutputCapabilities {
    pub backends: Vec<DesktopAudioOutputBackendCapabilities>,
    pub diagnostics: Vec<DesktopAudioOutputDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DesktopAudioOutputBackendCapabilities {
    pub backend: DesktopAudioOutputBackend,
    pub platform_api: Option<String>,
    pub status: DesktopAudioOutputCapabilityStatus,
    pub default_for_ci: bool,
    pub device_count: usize,
    pub devices: Vec<DesktopAudioDeviceSummary>,
    pub fallback_reason: Option<String>,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DesktopAudioOutputBackend {
    Mock,
    Native,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DesktopAudioOutputCapabilityStatus {
    Ready,
    Warning,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DesktopAudioDeviceSummary {
    pub device_id: String,
    pub safe_label: String,
    pub sample_rates_hz: Vec<u32>,
    pub max_channel_count: u16,
    pub default_device: bool,
    pub backend: DesktopAudioOutputBackend,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DesktopAudioOutputDiagnostic {
    pub backend: DesktopAudioOutputBackend,
    pub platform_api: Option<String>,
    pub message: String,
    pub skipped: bool,
    pub ready: bool,
}

pub fn probe_desktop_audio_output_capabilities() -> DesktopAudioOutputCapabilities {
    let mut diagnostics = Vec::new();
    let mock = mock_output::mock_capabilities();
    let native = cpal_output::probe_cpal_output_capabilities(&mut diagnostics);

    DesktopAudioOutputCapabilities {
        backends: vec![mock, native],
        diagnostics,
    }
}

pub fn native_audio_probe() -> DesktopAudioOutputDiagnostic {
    if std::env::var_os("VIDEO_EDITOR_TEST_NATIVE_AUDIO").is_none() {
        return DesktopAudioOutputDiagnostic {
            backend: DesktopAudioOutputBackend::Native,
            platform_api: cpal_output::target_platform_api(),
            message: "skipping native audio output proof; set VIDEO_EDITOR_TEST_NATIVE_AUDIO=1"
                .to_owned(),
            skipped: true,
            ready: false,
        };
    }

    cpal_output::open_native_audio_probe()
}
