//! Rust-owned audio engine contracts.

pub mod dsp_timeline;
pub mod mix_intent;

pub use dsp_timeline::{
    DspEffectSlotClassification, DspEffectSlotSupport, DspEvaluationDiagnostic, DspFadeEnvelope,
    DspGainEnvelope, DspGainPoint, DspMixClassification, DspPanEnvelope, DspSegment,
    DspTimelineConfig, DspTimelineError, DspTimelinePlan, DspTrack, evaluate_dsp_timeline,
};
pub use mix_intent::{AudioMixClassification, AudioMixIntent, AudioMixSegment, AudioMixSummary};
