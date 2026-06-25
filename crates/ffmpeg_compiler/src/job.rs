use std::collections::BTreeMap;
use std::error::Error;
use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};

use draft_model::{
    BUNDLED_SERIF_TEXT_FONT_RELATIVE_PATH, BUNDLED_TEXT_FONT_FAMILY,
    BUNDLED_TEXT_FONT_LICENSE_SPDX, BUNDLED_TEXT_FONT_REF, BUNDLED_TEXT_FONT_RELATIVE_PATH,
    MaterialId, MaterialKind, Microseconds, RationalFrameRate, SegmentId,
};
use render_graph::{
    ExportMp4Preset, PreviewFrameFormat, RenderAudioCodec, RenderAudioMixDiagnostic,
    RenderCanvasDiagnostic, RenderContainer, RenderFilterIntent, RenderGraphPlan,
    RenderIntentSupport, RenderOutputProfile, RenderTransitionIntent, RenderVideoCodec,
    RenderVisualDiagnostic,
};
use serde::{Deserialize, Serialize};

use crate::ass::{TextRenderCapability, generate_ass_sidecars};
use crate::filters::generate_filter_script;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompileContext {
    pub output_path: String,
    pub artifact_dir: String,
    pub capabilities: CompilerCapabilities,
}

impl CompileContext {
    pub fn new(output_path: impl AsRef<Path>, artifact_dir: impl AsRef<Path>) -> Self {
        Self {
            output_path: path_to_string(output_path.as_ref()),
            artifact_dir: path_to_string(artifact_dir.as_ref()),
            capabilities: CompilerCapabilities::default(),
        }
    }

    pub fn with_capabilities(mut self, capabilities: CompilerCapabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn with_output_path(mut self, output_path: impl AsRef<Path>) -> Self {
        self.output_path = path_to_string(output_path.as_ref());
        self
    }

    pub fn artifact_path(&self, file_name: &str) -> String {
        let mut path = PathBuf::from(&self.artifact_dir);
        path.push(file_name);
        path_to_string(&path)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompilerCapabilities {
    pub supports_h264_encoder: bool,
    pub supports_aac_encoder: bool,
    pub text: TextRenderCapability,
}

impl CompilerCapabilities {
    pub fn all_available_for_tests() -> Self {
        Self {
            supports_h264_encoder: true,
            supports_aac_encoder: true,
            text: TextRenderCapability {
                supports_ass_filter: true,
                supports_subtitles_filter: true,
                env_text_font_path: Some("/fonts/PingFang.ttc".to_owned()),
                available_font_paths: vec![
                    BUNDLED_TEXT_FONT_RELATIVE_PATH.to_owned(),
                    BUNDLED_SERIF_TEXT_FONT_RELATIVE_PATH.to_owned(),
                    "/fonts/PingFang.ttc".to_owned(),
                    "/System/Library/Fonts/PingFang.ttc".to_owned(),
                    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf".to_owned(),
                ],
                bundled_font_ref: Some(BUNDLED_TEXT_FONT_REF.to_owned()),
                bundled_font_family: Some(BUNDLED_TEXT_FONT_FAMILY.to_owned()),
                bundled_font_path: Some(BUNDLED_TEXT_FONT_RELATIVE_PATH.to_owned()),
                bundled_font_license: Some(BUNDLED_TEXT_FONT_LICENSE_SPDX.to_owned()),
            },
        }
    }

    pub fn with_text(mut self, text: TextRenderCapability) -> Self {
        self.text = text;
        self
    }

    pub fn with_h264_encoder(mut self, supported: bool) -> Self {
        self.supports_h264_encoder = supported;
        self
    }
}

impl Default for CompilerCapabilities {
    fn default() -> Self {
        Self {
            supports_h264_encoder: true,
            supports_aac_encoder: true,
            text: TextRenderCapability::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FfmpegJob {
    pub job_id: String,
    pub output_kind: FfmpegOutputKind,
    pub output_path: String,
    pub inputs: Vec<FfmpegInput>,
    pub sidecars: Vec<FfmpegSidecar>,
    pub filter_script: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub filter_script_diagnostics: Vec<RenderAudioMixDiagnostic>,
    pub encode_settings: EncodeSettings,
    pub validation: OutputValidationExpectation,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub canvas_diagnostics: Vec<RenderCanvasDiagnostic>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub visual_diagnostics: Vec<RenderVisualDiagnostic>,
    #[serde(serialize_with = "serialize_os_args")]
    pub args: Vec<OsString>,
}

impl FfmpegJob {
    pub fn args_as_strings(&self) -> Vec<String> {
        self.args
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FfmpegInput {
    pub input_index: u32,
    pub material_id: MaterialId,
    pub material_kind: MaterialKind,
    pub uri: String,
    pub ffmpeg_path: String,
    pub has_video: bool,
    pub has_audio: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FfmpegSidecar {
    pub sidecar_id: String,
    pub kind: FfmpegSidecarKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub segment_id: Option<SegmentId>,
    pub path: String,
    pub contents: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FfmpegSidecarKind {
    FilterScript,
    AssSubtitle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FfmpegOutputKind {
    PreviewFramePng,
    PreviewFrameJpeg,
    PreviewSegmentMp4,
    ExportMp4,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EncodeSettings {
    pub container: Option<String>,
    pub video_codec: Option<String>,
    pub audio_codec: Option<String>,
    pub crf: Option<u8>,
    pub audio_bitrate_kbps: Option<u32>,
    pub pixel_format: Option<String>,
    pub dimensions: OutputDimensionsSnapshot,
    pub frame_rate: RationalFrameRate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OutputDimensionsSnapshot {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OutputValidationExpectation {
    pub must_exist: bool,
    pub must_be_non_empty: bool,
    pub expected_duration: Microseconds,
    pub expected_frame_rate: RationalFrameRate,
    pub expected_width: u32,
    pub expected_height: u32,
    pub expect_audio_stream: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FfmpegCompileError {
    pub kind: FfmpegCompileErrorKind,
    pub message: String,
    pub remediation: String,
    pub material_id: Option<MaterialId>,
}

impl FfmpegCompileError {
    pub fn new(
        kind: FfmpegCompileErrorKind,
        message: impl Into<String>,
        remediation: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            message: message.into(),
            remediation: remediation.into(),
            material_id: None,
        }
    }

    pub fn with_material_id(mut self, material_id: MaterialId) -> Self {
        self.material_id = Some(material_id);
        self
    }
}

impl fmt::Display for FfmpegCompileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}: {}", self.kind, self.message)
    }
}

impl Error for FfmpegCompileError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FfmpegCompileErrorKind {
    MissingOutputPath,
    UnsupportedEncoder,
    MissingTextFilterSupport,
    UnsupportedTextResource,
    UnsupportedAudioAutomation,
    MissingTextFont,
    MissingInputMaterial,
}

pub fn compile_ffmpeg_job(
    plan: &RenderGraphPlan,
    context: &CompileContext,
) -> Result<FfmpegJob, FfmpegCompileError> {
    if context.output_path.trim().is_empty() {
        return Err(FfmpegCompileError::new(
            FfmpegCompileErrorKind::MissingOutputPath,
            "FFmpeg job output path must not be empty",
            "Choose a derived preview/export output path before compiling the job.",
        ));
    }

    validate_profile_capabilities(plan, context)?;

    let job_id = stable_job_id(
        plan.graph.draft_id.as_str(),
        profile_id(&plan.output_profile),
    );
    let inputs = collect_inputs(plan)?;
    let ass_sidecars = generate_ass_sidecars(plan, context, &job_id)?;
    let filter = generate_filter_script(plan, context, &inputs, &ass_sidecars, &job_id)?;

    let mut sidecars = vec![FfmpegSidecar {
        sidecar_id: format!("{job_id}-filter"),
        kind: FfmpegSidecarKind::FilterScript,
        segment_id: None,
        path: filter.path.clone(),
        contents: filter.contents.clone(),
    }];
    sidecars.extend(ass_sidecars);

    let encode_settings = encode_settings(&plan.output_profile);
    let validation = output_validation(&plan.output_profile, filter.has_audio_output);
    let output_kind = output_kind(&plan.output_profile);
    let args = build_args(
        &inputs,
        &sidecars[0],
        &context.output_path,
        output_kind,
        &encode_settings,
        validation.expected_duration,
        filter.has_audio_output,
    );

    let mut visual_diagnostics = plan.graph.visual_diagnostics.clone();
    visual_diagnostics.extend(effect_capability_diagnostics(plan));

    Ok(FfmpegJob {
        job_id,
        output_kind,
        output_path: context.output_path.clone(),
        inputs,
        sidecars,
        filter_script: filter.contents,
        filter_script_diagnostics: filter.diagnostics,
        encode_settings,
        validation,
        canvas_diagnostics: plan.graph.canvas.diagnostics.clone(),
        visual_diagnostics,
        args,
    })
}

fn effect_capability_diagnostics(plan: &RenderGraphPlan) -> Vec<RenderVisualDiagnostic> {
    let mut diagnostics = Vec::new();
    for layer in &plan.graph.video_layers {
        diagnostics.extend(layer.filters.iter().filter_map(|filter| {
            filter_export_diagnostic(
                &layer.track_id,
                &layer.segment_id,
                &layer.material_id,
                filter,
            )
        }));
        if let Some(transition) = &layer.transition {
            if let Some(diagnostic) = transition_export_diagnostic(
                &layer.track_id,
                &layer.segment_id,
                &layer.material_id,
                transition,
            ) {
                diagnostics.push(diagnostic);
            }
        }
    }
    for overlay in &plan.graph.text_overlays {
        diagnostics.extend(overlay.filters.iter().filter_map(|filter| {
            filter_export_diagnostic(
                &overlay.overlay.track_id,
                &overlay.overlay.segment_id,
                &overlay.material_id,
                filter,
            )
        }));
        if let Some(transition) = &overlay.transition {
            if let Some(diagnostic) = transition_export_diagnostic(
                &overlay.overlay.track_id,
                &overlay.overlay.segment_id,
                &overlay.material_id,
                transition,
            ) {
                diagnostics.push(diagnostic);
            }
        }
    }
    for mix in &plan.graph.audio_mixes {
        diagnostics.extend(mix.filters.iter().filter_map(|filter| {
            filter_export_diagnostic(&mix.track_id, &mix.segment_id, &mix.material_id, filter)
        }));
    }
    diagnostics
}

fn filter_export_diagnostic(
    track_id: &draft_model::TrackId,
    segment_id: &SegmentId,
    material_id: &MaterialId,
    filter: &RenderFilterIntent,
) -> Option<RenderVisualDiagnostic> {
    export_support_diagnostic(
        track_id,
        segment_id,
        material_id,
        "filter",
        filter.support,
        &filter.reason,
    )
}

fn transition_export_diagnostic(
    track_id: &draft_model::TrackId,
    segment_id: &SegmentId,
    material_id: &MaterialId,
    transition: &RenderTransitionIntent,
) -> Option<RenderVisualDiagnostic> {
    export_support_diagnostic(
        track_id,
        segment_id,
        material_id,
        "transition",
        transition.support,
        &transition.reason,
    )
}

fn export_support_diagnostic(
    track_id: &draft_model::TrackId,
    segment_id: &SegmentId,
    material_id: &MaterialId,
    property: &str,
    support: RenderIntentSupport,
    reason: &str,
) -> Option<RenderVisualDiagnostic> {
    if support == RenderIntentSupport::Supported {
        return None;
    }
    Some(RenderVisualDiagnostic {
        track_id: track_id.clone(),
        segment_id: segment_id.clone(),
        material_id: material_id.clone(),
        property: property.to_owned(),
        support,
        reason: reason.to_owned(),
    })
}

pub fn input_index_by_material(inputs: &[FfmpegInput]) -> BTreeMap<MaterialId, u32> {
    inputs
        .iter()
        .map(|input| (input.material_id.clone(), input.input_index))
        .collect()
}

pub fn format_seconds(value: Microseconds) -> String {
    let whole = value.get() / 1_000_000;
    let micros = value.get() % 1_000_000;
    format!("{whole}.{micros:06}")
}

pub fn sanitize_id(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                character
            } else {
                '-'
            }
        })
        .collect()
}

fn collect_inputs(plan: &RenderGraphPlan) -> Result<Vec<FfmpegInput>, FfmpegCompileError> {
    plan.graph
        .materials
        .iter()
        .filter(|material| material.has_video || material.has_audio)
        .enumerate()
        .map(|(input_index, material)| {
            let input_index = u32::try_from(input_index).map_err(|_| {
                FfmpegCompileError::new(
                    FfmpegCompileErrorKind::MissingInputMaterial,
                    "too many FFmpeg input materials to index deterministically",
                    "Reduce the number of render materials in the graph.",
                )
            })?;
            Ok(FfmpegInput {
                input_index,
                material_id: material.material_id.clone(),
                material_kind: material.kind,
                uri: material.uri.clone(),
                ffmpeg_path: ffmpeg_path_from_uri(&material.uri),
                has_video: material.has_video,
                has_audio: material.has_audio,
            })
        })
        .collect()
}

fn validate_profile_capabilities(
    plan: &RenderGraphPlan,
    context: &CompileContext,
) -> Result<(), FfmpegCompileError> {
    match &plan.output_profile {
        RenderOutputProfile::PreviewFrame { .. } => Ok(()),
        RenderOutputProfile::PreviewSegment { .. } | RenderOutputProfile::ExportMp4 { .. } => {
            if !context.capabilities.supports_h264_encoder {
                return Err(FfmpegCompileError::new(
                    FfmpegCompileErrorKind::UnsupportedEncoder,
                    "H.264 encoder is not available for MP4 output",
                    "Install or select an FFmpeg runtime with libx264 or a supported hardware H.264 encoder.",
                ));
            }
            if !context.capabilities.supports_aac_encoder {
                return Err(FfmpegCompileError::new(
                    FfmpegCompileErrorKind::UnsupportedEncoder,
                    "AAC encoder is not available for MP4 output",
                    "Install or select an FFmpeg runtime with AAC encoder support.",
                ));
            }
            Ok(())
        }
    }
}

fn build_args(
    inputs: &[FfmpegInput],
    filter_script: &FfmpegSidecar,
    output_path: &str,
    output_kind: FfmpegOutputKind,
    encode_settings: &EncodeSettings,
    output_duration: Microseconds,
    has_audio_output: bool,
) -> Vec<OsString> {
    let mut args = vec![OsString::from("-y")];
    for input in inputs {
        args.push(OsString::from("-i"));
        args.push(OsString::from(&input.ffmpeg_path));
    }
    args.extend([
        OsString::from("-filter_complex_script"),
        OsString::from(&filter_script.path),
        OsString::from("-map"),
        OsString::from("[vout]"),
    ]);

    match output_kind {
        FfmpegOutputKind::PreviewFramePng | FfmpegOutputKind::PreviewFrameJpeg => {
            args.extend([
                OsString::from("-frames:v"),
                OsString::from("1"),
                OsString::from("-f"),
                OsString::from("image2"),
                OsString::from("-c:v"),
                OsString::from(match output_kind {
                    FfmpegOutputKind::PreviewFramePng => "png",
                    FfmpegOutputKind::PreviewFrameJpeg => "mjpeg",
                    _ => unreachable!("covered by outer match"),
                }),
            ]);
        }
        FfmpegOutputKind::PreviewSegmentMp4 | FfmpegOutputKind::ExportMp4 => {
            if has_audio_output {
                args.extend([OsString::from("-map"), OsString::from("[aout]")]);
            }
            args.extend([
                OsString::from("-c:v"),
                OsString::from(encode_settings.video_codec.as_deref().unwrap_or("libx264")),
                OsString::from("-pix_fmt"),
                OsString::from(encode_settings.pixel_format.as_deref().unwrap_or("yuv420p")),
                OsString::from("-crf"),
                OsString::from(encode_settings.crf.unwrap_or(23).to_string()),
            ]);
            if has_audio_output {
                args.extend([
                    OsString::from("-c:a"),
                    OsString::from(encode_settings.audio_codec.as_deref().unwrap_or("aac")),
                    OsString::from("-b:a"),
                    OsString::from(format!(
                        "{}k",
                        encode_settings.audio_bitrate_kbps.unwrap_or(128)
                    )),
                ]);
            }
            args.extend([
                OsString::from("-t"),
                OsString::from(format_seconds(output_duration)),
            ]);
            args.extend([OsString::from("-movflags"), OsString::from("+faststart")]);
        }
    }

    args.push(OsString::from(output_path));
    args
}

fn output_kind(profile: &RenderOutputProfile) -> FfmpegOutputKind {
    match profile {
        RenderOutputProfile::PreviewFrame { format, .. } => match format {
            PreviewFrameFormat::Png => FfmpegOutputKind::PreviewFramePng,
            PreviewFrameFormat::Jpeg => FfmpegOutputKind::PreviewFrameJpeg,
        },
        RenderOutputProfile::PreviewSegment { .. } => FfmpegOutputKind::PreviewSegmentMp4,
        RenderOutputProfile::ExportMp4 { .. } => FfmpegOutputKind::ExportMp4,
    }
}

fn profile_id(profile: &RenderOutputProfile) -> &str {
    match profile {
        RenderOutputProfile::PreviewFrame { profile_id, .. }
        | RenderOutputProfile::PreviewSegment { profile_id, .. }
        | RenderOutputProfile::ExportMp4 { profile_id, .. } => profile_id,
    }
}

fn encode_settings(profile: &RenderOutputProfile) -> EncodeSettings {
    match profile {
        RenderOutputProfile::PreviewFrame {
            dimensions,
            frame_rate,
            ..
        } => EncodeSettings {
            container: None,
            video_codec: None,
            audio_codec: None,
            crf: None,
            audio_bitrate_kbps: None,
            pixel_format: None,
            dimensions: OutputDimensionsSnapshot {
                width: dimensions.width,
                height: dimensions.height,
            },
            frame_rate: frame_rate.clone(),
        },
        RenderOutputProfile::PreviewSegment {
            dimensions,
            frame_rate,
            container,
            video_codec,
            audio_codec,
            ..
        } => EncodeSettings {
            container: Some(container_name(*container).to_owned()),
            video_codec: Some(video_codec_name(*video_codec).to_owned()),
            audio_codec: Some(audio_codec_name(*audio_codec).to_owned()),
            crf: Some(28),
            audio_bitrate_kbps: Some(128),
            pixel_format: Some("yuv420p".to_owned()),
            dimensions: OutputDimensionsSnapshot {
                width: dimensions.width,
                height: dimensions.height,
            },
            frame_rate: frame_rate.clone(),
        },
        RenderOutputProfile::ExportMp4 {
            dimensions,
            frame_rate,
            preset,
            ..
        } => encode_settings_from_preset(dimensions.width, dimensions.height, frame_rate, preset),
    }
}

fn encode_settings_from_preset(
    width: u32,
    height: u32,
    frame_rate: &RationalFrameRate,
    preset: &ExportMp4Preset,
) -> EncodeSettings {
    EncodeSettings {
        container: Some(container_name(preset.container).to_owned()),
        video_codec: Some(video_codec_name(preset.video_codec).to_owned()),
        audio_codec: Some(audio_codec_name(preset.audio_codec).to_owned()),
        crf: Some(preset.crf),
        audio_bitrate_kbps: Some(preset.audio_bitrate_kbps),
        pixel_format: Some("yuv420p".to_owned()),
        dimensions: OutputDimensionsSnapshot { width, height },
        frame_rate: frame_rate.clone(),
    }
}

fn output_validation(
    profile: &RenderOutputProfile,
    has_audio_output: bool,
) -> OutputValidationExpectation {
    match profile {
        RenderOutputProfile::PreviewFrame {
            dimensions,
            frame_rate,
            target_timerange,
            ..
        }
        | RenderOutputProfile::PreviewSegment {
            dimensions,
            frame_rate,
            target_timerange,
            ..
        }
        | RenderOutputProfile::ExportMp4 {
            dimensions,
            frame_rate,
            target_timerange,
            ..
        } => OutputValidationExpectation {
            must_exist: true,
            must_be_non_empty: true,
            expected_duration: target_timerange.duration,
            expected_frame_rate: frame_rate.clone(),
            expected_width: dimensions.width,
            expected_height: dimensions.height,
            expect_audio_stream: has_audio_output
                && !matches!(profile, RenderOutputProfile::PreviewFrame { .. }),
        },
    }
}

fn stable_job_id(draft_id: &str, profile_id: &str) -> String {
    sanitize_id(&format!("{draft_id}-{profile_id}"))
}

fn container_name(container: RenderContainer) -> &'static str {
    match container {
        RenderContainer::Mp4 => "mp4",
    }
}

fn video_codec_name(codec: RenderVideoCodec) -> &'static str {
    match codec {
        RenderVideoCodec::H264 => "libx264",
    }
}

fn audio_codec_name(codec: RenderAudioCodec) -> &'static str {
    match codec {
        RenderAudioCodec::Aac => "aac",
    }
}

fn ffmpeg_path_from_uri(uri: &str) -> String {
    uri.strip_prefix("file://").unwrap_or(uri).to_owned()
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn serialize_os_args<S>(args: &[OsString], serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let values = args
        .iter()
        .map(|value| value.to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    values.serialize(serializer)
}
