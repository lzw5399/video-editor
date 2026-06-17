//! FFmpeg command-plan compiler boundary.
//!
//! This crate will compile typed render graph intents into FFmpeg inputs, filter
//! scripts, subtitle artifacts, and encode argument plans. It must not own draft
//! editing behavior, UI state, or process execution.

pub mod ass;
pub mod filters;
pub mod job;

pub use ass::{ResolvedTextFont, TextRenderCapability, generate_ass_sidecars};
pub use filters::{GeneratedFilterScript, generate_filter_script};
pub use job::{
    CompileContext, CompilerCapabilities, EncodeSettings, FfmpegCompileError,
    FfmpegCompileErrorKind, FfmpegInput, FfmpegJob, FfmpegOutputKind, FfmpegSidecar,
    FfmpegSidecarKind, OutputValidationExpectation, compile_ffmpeg_job,
};

/// Boundary marker for FFmpeg compilation planning.
pub const FFMPEG_COMPILER_BOUNDARY: &str = "render-graph-to-ffmpeg-plan";
