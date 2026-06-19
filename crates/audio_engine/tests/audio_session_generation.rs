use audio_engine::{
    AudioBufferRequest, AudioOutputDevice, AudioOutputStream, AudioPreviewRuntime,
    AudioPreviewSessionConfig, AudioPreviewStatusLabel, MockAudioOutputDevice,
};
use draft_model::{Microseconds, RationalFrameRate};
use realtime_preview_runtime::{PlaybackRate, PlaybackState};

#[test]
fn audio_session_generation_reuses_timeline_clock_events() {
    let mut runtime = AudioPreviewRuntime::new();
    let session_id = runtime.create_session(session_config()).unwrap();

    let initial = runtime.status(session_id).unwrap();
    assert_eq!(initial.playback_generation.get(), 0);
    assert_eq!(initial.playback_state, PlaybackState::Stopped);

    let after_seek = runtime
        .seek(session_id, Microseconds::new(1_000_000))
        .unwrap();
    assert_eq!(after_seek.get(), 1);
    assert_eq!(
        runtime.status(session_id).unwrap().playback_generation,
        after_seek
    );
    assert_eq!(
        runtime.status(session_id).unwrap().playback_generation,
        after_seek
    );

    let after_pause = runtime.pause(session_id).unwrap();
    assert_eq!(after_pause.get(), 2);
    let after_resume = runtime.resume(session_id).unwrap();
    assert_eq!(after_resume.get(), 3);
    let after_edit = runtime.accepted_edit(session_id).unwrap();
    assert_eq!(after_edit.get(), 4);
    let after_reload = runtime.draft_reloaded(session_id).unwrap();
    assert_eq!(after_reload.get(), 5);
    let after_relink = runtime.material_relinked(session_id).unwrap();
    assert_eq!(after_relink.get(), 6);

    let status = runtime.status(session_id).unwrap();
    assert_eq!(status.playback_generation, after_relink);
    assert_eq!(status.playback_state, PlaybackState::Stopped);
    assert_eq!(status.target_time, Microseconds::ZERO);
}

#[test]
fn audio_session_generation_rejects_stale_and_canceled_buffers_with_telemetry() {
    let mut runtime = AudioPreviewRuntime::new();
    let session_id = runtime.create_session(session_config()).unwrap();
    let current_generation = runtime.status(session_id).unwrap().playback_generation;

    let presented = runtime
        .request_buffer(session_id, buffer_request(current_generation))
        .unwrap();
    assert!(presented.presented);
    assert_eq!(
        presented.presented_frame_count,
        presented.requested_frame_count
    );

    runtime
        .seek(session_id, Microseconds::new(500_000))
        .unwrap();
    let stale = runtime
        .request_buffer(session_id, buffer_request(current_generation))
        .unwrap();
    assert!(!stale.presented);
    assert!(stale.stale_rejected);
    assert_eq!(stale.presented_frame_count, 0);
    assert!(stale.diagnostics.iter().any(|diagnostic| diagnostic.stale));

    let token = runtime.next_cancellation_token(session_id).unwrap();
    runtime.cancel_request(session_id, token).unwrap();
    let generation = runtime.status(session_id).unwrap().playback_generation;
    let mut canceled_request = buffer_request(generation);
    canceled_request.cancellation_token = Some(token);
    let canceled = runtime
        .request_buffer(session_id, canceled_request)
        .unwrap();
    assert!(!canceled.presented);
    assert!(canceled.canceled);
    assert!(
        canceled
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.canceled)
    );

    let telemetry = runtime.telemetry(session_id).unwrap();
    assert_eq!(telemetry.presented_buffer_count, 1);
    assert_eq!(telemetry.stale_rejected_count, 1);
    assert_eq!(telemetry.canceled_buffer_count, 1);
    assert_eq!(telemetry.generation, generation);
}

#[test]
fn audio_session_generation_bounds_buffer_contracts_and_uses_mock_output_only() {
    let mut runtime = AudioPreviewRuntime::new();
    let session_id = runtime.create_session(session_config()).unwrap();
    let generation = runtime.status(session_id).unwrap().playback_generation;

    let oversized = AudioBufferRequest {
        target_time: Microseconds::new(250_000),
        playback_generation: generation,
        requested_frame_count: 96_000,
        channel_count: 8,
        sample_rate_hz: 48_000,
        max_buffer_duration_microseconds: Microseconds::new(2_000_000),
        cancellation_token: None,
    };

    let result = runtime.request_buffer(session_id, oversized).unwrap();
    assert!(!result.presented);
    assert_eq!(result.requested_frame_count, 96_000);
    assert_eq!(result.presented_frame_count, 0);
    assert_eq!(
        result.max_buffer_duration_microseconds,
        Microseconds::new(50_000)
    );
    assert_eq!(result.status_label, AudioPreviewStatusLabel::Rejected);
    assert!(
        result
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.bounded)
    );

    let device = MockAudioOutputDevice::default();
    let capabilities = device.capabilities();
    assert_eq!(capabilities.max_channel_count, 2);
    assert_eq!(capabilities.max_frame_count, 2_400);
    let mut stream = device.open_stream(&capabilities).unwrap();
    stream.present(&result).unwrap();
    assert_eq!(stream.presented_result_count(), 1);
}

fn session_config() -> AudioPreviewSessionConfig {
    AudioPreviewSessionConfig {
        session_label: "audio-session-test".to_owned(),
        frame_rate: RationalFrameRate::new(30, 1),
        playback_rate: PlaybackRate::normal(),
        sample_rate_hz: 48_000,
        max_buffer_duration_microseconds: Microseconds::new(50_000),
        max_channel_count: 2,
        max_frame_count: 2_400,
    }
}

fn buffer_request(
    playback_generation: realtime_preview_runtime::PlaybackGeneration,
) -> AudioBufferRequest {
    AudioBufferRequest {
        target_time: Microseconds::new(250_000),
        playback_generation,
        requested_frame_count: 1_024,
        channel_count: 2,
        sample_rate_hz: 48_000,
        max_buffer_duration_microseconds: Microseconds::new(50_000),
        cancellation_token: None,
    }
}
