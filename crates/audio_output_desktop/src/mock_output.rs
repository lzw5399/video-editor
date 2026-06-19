use audio_engine::MockAudioOutputDevice;

use crate::{
    DesktopAudioDeviceSummary, DesktopAudioOutputBackend, DesktopAudioOutputBackendCapabilities,
    DesktopAudioOutputCapabilityStatus,
};

pub fn create_desktop_audio_output() -> MockAudioOutputDevice {
    MockAudioOutputDevice::default()
}

pub(crate) fn mock_capabilities() -> DesktopAudioOutputBackendCapabilities {
    DesktopAudioOutputBackendCapabilities {
        backend: DesktopAudioOutputBackend::Mock,
        platform_api: None,
        status: DesktopAudioOutputCapabilityStatus::Ready,
        default_for_ci: true,
        device_count: 1,
        devices: vec![DesktopAudioDeviceSummary {
            device_id: "mock-audio-output".to_owned(),
            safe_label: "Mock audio output".to_owned(),
            sample_rates_hz: vec![48_000],
            max_channel_count: 2,
            default_device: true,
            backend: DesktopAudioOutputBackend::Mock,
        }],
        fallback_reason: None,
        diagnostic: Some(
            "Mock output is the default CI-safe backend and requires no physical device."
                .to_owned(),
        ),
    }
}
