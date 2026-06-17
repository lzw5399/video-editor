use draft_model::{Microseconds, RationalFrameRate, TargetTimerange};
use serde::{Deserialize, Serialize};

use crate::{RenderGraph, RenderGraphError, RenderGraphErrorKind};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderGraphPlan {
    pub graph: RenderGraph,
    pub output_profile: RenderOutputProfile,
}

impl RenderGraphPlan {
    pub fn new(
        graph: RenderGraph,
        output_profile: RenderOutputProfile,
    ) -> Result<Self, RenderGraphError> {
        output_profile.validate()?;
        Ok(Self {
            graph,
            output_profile,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum RenderOutputProfile {
    PreviewFrame {
        profile_id: String,
        dimensions: OutputDimensions,
        frame_rate: RationalFrameRate,
        target_timerange: TargetTimerange,
        format: PreviewFrameFormat,
        validation_hints: Vec<String>,
    },
    PreviewSegment {
        profile_id: String,
        dimensions: OutputDimensions,
        frame_rate: RationalFrameRate,
        target_timerange: TargetTimerange,
        container: RenderContainer,
        video_codec: RenderVideoCodec,
        audio_codec: RenderAudioCodec,
        preset_id: String,
        validation_hints: Vec<String>,
    },
    ExportMp4 {
        profile_id: String,
        dimensions: OutputDimensions,
        frame_rate: RationalFrameRate,
        target_timerange: TargetTimerange,
        preset: ExportMp4Preset,
        validation_hints: Vec<String>,
    },
}

impl RenderOutputProfile {
    pub fn preview_frame_png(
        dimensions: OutputDimensions,
        frame_rate: RationalFrameRate,
        target_timerange: TargetTimerange,
    ) -> Self {
        Self::preview_frame(
            "preview-frame-png",
            dimensions,
            frame_rate,
            target_timerange,
            PreviewFrameFormat::Png,
        )
    }

    pub fn preview_frame(
        profile_id: impl Into<String>,
        dimensions: OutputDimensions,
        frame_rate: RationalFrameRate,
        target_timerange: TargetTimerange,
        format: PreviewFrameFormat,
    ) -> Self {
        Self::PreviewFrame {
            profile_id: profile_id.into(),
            dimensions,
            frame_rate,
            target_timerange,
            format,
            validation_hints: vec![
                "single-frame still output".to_owned(),
                "preserve alpha only if compiler/runtime supports it".to_owned(),
            ],
        }
    }

    pub fn preview_segment_mp4(
        dimensions: OutputDimensions,
        frame_rate: RationalFrameRate,
        target_timerange: TargetTimerange,
    ) -> Self {
        Self::PreviewSegment {
            profile_id: "preview-segment-mp4-h264".to_owned(),
            dimensions,
            frame_rate,
            target_timerange,
            container: RenderContainer::Mp4,
            video_codec: RenderVideoCodec::H264,
            audio_codec: RenderAudioCodec::Aac,
            preset_id: "preview-segment-balanced".to_owned(),
            validation_hints: vec![
                "short derived preview cache artifact".to_owned(),
                "compiled through the same render graph as export".to_owned(),
            ],
        }
    }

    pub fn export_mp4(
        dimensions: OutputDimensions,
        frame_rate: RationalFrameRate,
        target_timerange: TargetTimerange,
        preset: ExportMp4Preset,
    ) -> Self {
        Self::ExportMp4 {
            profile_id: "export-mp4-h264-balanced".to_owned(),
            dimensions,
            frame_rate,
            target_timerange,
            preset,
            validation_hints: vec![
                "validate file exists and is non-empty".to_owned(),
                "validate duration, fps, resolution, and audio stream with ffprobe".to_owned(),
            ],
        }
    }

    pub fn validate(&self) -> Result<(), RenderGraphError> {
        match self {
            Self::PreviewFrame {
                profile_id,
                dimensions,
                frame_rate,
                target_timerange,
                format: _,
                validation_hints: _,
            } => validate_common_profile(profile_id, dimensions, frame_rate, target_timerange),
            Self::PreviewSegment {
                profile_id,
                dimensions,
                frame_rate,
                target_timerange,
                container: _,
                video_codec: _,
                audio_codec: _,
                preset_id,
                validation_hints: _,
            } => {
                validate_common_profile(profile_id, dimensions, frame_rate, target_timerange)?;
                validate_non_empty_id("preview segment presetId", preset_id)
            }
            Self::ExportMp4 {
                profile_id,
                dimensions,
                frame_rate,
                target_timerange,
                preset,
                validation_hints: _,
            } => {
                validate_common_profile(profile_id, dimensions, frame_rate, target_timerange)?;
                preset.validate()
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OutputDimensions {
    pub width: u32,
    pub height: u32,
}

impl OutputDimensions {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    fn validate(self) -> Result<(), RenderGraphError> {
        if self.width == 0 || self.height == 0 {
            return Err(unsupported_profile(
                "output profile dimensions width and height must be greater than zero",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PreviewFrameFormat {
    Png,
    Jpeg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RenderContainer {
    Mp4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RenderVideoCodec {
    H264,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RenderAudioCodec {
    Aac,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportMp4Preset {
    pub preset_id: String,
    pub container: RenderContainer,
    pub video_codec: RenderVideoCodec,
    pub audio_codec: RenderAudioCodec,
    pub crf: u8,
    pub audio_bitrate_kbps: u32,
}

impl ExportMp4Preset {
    pub fn h264_aac_balanced() -> Self {
        Self {
            preset_id: "h264-aac-balanced".to_owned(),
            container: RenderContainer::Mp4,
            video_codec: RenderVideoCodec::H264,
            audio_codec: RenderAudioCodec::Aac,
            crf: 20,
            audio_bitrate_kbps: 192,
        }
    }

    fn validate(&self) -> Result<(), RenderGraphError> {
        validate_non_empty_id("export MP4 presetId", &self.preset_id)?;
        if !(1..=51).contains(&self.crf) {
            return Err(unsupported_profile(
                "export MP4 CRF must be within the supported H.264 range 1..=51",
            ));
        }
        if self.audio_bitrate_kbps == 0 {
            return Err(unsupported_profile(
                "export MP4 audio bitrate must be greater than zero",
            ));
        }
        Ok(())
    }
}

fn validate_common_profile(
    profile_id: &str,
    dimensions: &OutputDimensions,
    frame_rate: &RationalFrameRate,
    target_timerange: &TargetTimerange,
) -> Result<(), RenderGraphError> {
    validate_non_empty_id("output profileId", profile_id)?;
    dimensions.validate()?;
    if frame_rate.numerator == 0 || frame_rate.denominator == 0 {
        return Err(unsupported_profile(
            "output profile frameRate numerator and denominator must be greater than zero",
        ));
    }
    if target_timerange.duration == Microseconds::ZERO {
        return Err(unsupported_profile(
            "output profile targetTimerange duration must be greater than zero",
        ));
    }
    Ok(())
}

fn validate_non_empty_id(label: &str, value: &str) -> Result<(), RenderGraphError> {
    if value.trim().is_empty() {
        return Err(unsupported_profile(format!("{label} must not be empty")));
    }
    Ok(())
}

fn unsupported_profile(message: impl Into<String>) -> RenderGraphError {
    RenderGraphError {
        kind: RenderGraphErrorKind::UnsupportedProfileSetting,
        track_id: None,
        segment_id: None,
        material_id: None,
        message: message.into(),
    }
}
