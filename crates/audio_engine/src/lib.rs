//! Rust-owned audio engine contracts.

pub mod dsp_timeline;
pub mod mix_intent;
pub mod output;
pub mod session;
pub mod telemetry;

pub use dsp_timeline::{
    DspEffectSlotClassification, DspEffectSlotSupport, DspEvaluationDiagnostic, DspFadeEnvelope,
    DspGainEnvelope, DspGainPoint, DspMixClassification, DspPanEnvelope, DspSegment,
    DspTimelineConfig, DspTimelineError, DspTimelinePlan, DspTrack, evaluate_dsp_timeline,
};
pub use mix_intent::{AudioMixClassification, AudioMixIntent, AudioMixSegment, AudioMixSummary};
pub use output::AudioOutputSink as AudioOutputStream;
pub use output::MockAudioOutputSink as MockAudioOutputStream;
pub use output::{
    AudioOutputCapabilities, AudioOutputDevice, AudioOutputError, AudioOutputSink,
    MockAudioOutputDevice, MockAudioOutputSink,
};
pub use session::{
    AudioBufferRequest, AudioBufferResult, AudioCancellationToken, AudioPreviewDiagnostic,
    AudioPreviewError, AudioPreviewRuntime, AudioPreviewSessionConfig, AudioPreviewSessionId,
    AudioPreviewStatus, AudioPreviewStatusLabel,
};
pub use telemetry::AudioPreviewTelemetry;
