use audio_engine::{AudioOutputDevice, AudioOutputStream};
use audio_output_desktop::{
    DesktopAudioOutputBackend, DesktopAudioOutputCapabilityStatus, MockAudioOutputDevice,
    create_desktop_audio_output, probe_desktop_audio_output_capabilities,
};

#[test]
fn audio_output_capabilities_report_mock_readiness_without_native_device() {
    let report = probe_desktop_audio_output_capabilities();
    let mock = report
        .backends
        .iter()
        .find(|backend| backend.backend == DesktopAudioOutputBackend::Mock)
        .expect("mock backend should always be reported for CI");

    assert_eq!(mock.status, DesktopAudioOutputCapabilityStatus::Ready);
    assert!(mock.default_for_ci);
    assert_eq!(mock.device_count, 1);
    assert!(mock.devices[0].safe_label.contains("Mock"));
    assert!(mock.devices[0].sample_rates_hz.contains(&48_000));
    assert_eq!(mock.devices[0].max_channel_count, 2);
    assert!(mock.fallback_reason.is_none());
}

#[test]
fn audio_output_capabilities_report_platform_domains_without_false_native_readiness() {
    let report = probe_desktop_audio_output_capabilities();
    let native = report
        .backends
        .iter()
        .find(|backend| backend.backend == DesktopAudioOutputBackend::Native)
        .expect("native backend domain should always be reported");

    #[cfg(target_os = "macos")]
    {
        assert_eq!(native.platform_api, Some("CoreAudio".to_owned()));
        assert_ne!(
            native.status,
            DesktopAudioOutputCapabilityStatus::Unsupported
        );
    }

    #[cfg(windows)]
    {
        assert_eq!(native.platform_api, Some("WASAPI".to_owned()));
        assert_ne!(
            native.status,
            DesktopAudioOutputCapabilityStatus::Unsupported
        );
    }

    #[cfg(not(any(target_os = "macos", windows)))]
    {
        assert_eq!(
            native.status,
            DesktopAudioOutputCapabilityStatus::Unsupported
        );
        assert!(
            native
                .fallback_reason
                .as_deref()
                .unwrap_or_default()
                .contains("unsupported")
        );
        assert_eq!(native.device_count, 0);
    }
}

#[test]
fn audio_output_capabilities_create_desktop_audio_output_defaults_to_mock_for_generic_ci() {
    let output = create_desktop_audio_output();
    let capabilities = output.capabilities();

    assert!(capabilities.mock);
    assert_eq!(capabilities.sample_rate_hz, 48_000);

    let stream = output
        .open_stream(&capabilities)
        .expect("mock output stream should open");
    assert_eq!(stream.presented_result_count(), 0);
}

#[test]
fn audio_output_capabilities_public_report_contains_only_safe_summaries() {
    let report = probe_desktop_audio_output_capabilities();
    let joined_labels = report
        .backends
        .iter()
        .flat_map(|backend| backend.devices.iter())
        .map(|device| device.safe_label.as_str())
        .collect::<Vec<_>>()
        .join(" ");

    assert!(!joined_labels.contains("deviceHandle"));
    assert!(!joined_labels.contains("outputDeviceHandle"));
    assert!(!joined_labels.contains("native pointer"));
    assert!(!joined_labels.contains("raw buffer"));
}

#[test]
fn audio_output_capabilities_mock_output_device_is_reexported_from_desktop_boundary() {
    let output = MockAudioOutputDevice::default();
    let capabilities = output.capabilities();

    assert!(capabilities.mock);
    assert_eq!(capabilities.device_id, "mock-audio-output");
}
