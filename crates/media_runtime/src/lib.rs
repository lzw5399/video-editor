//! FFmpeg process runtime boundary.
//!
//! This crate owns the service boundary for FFmpeg and ffprobe execution. Pure
//! draft and timeline semantic crates must not depend on this trait.

use std::ffi::OsString;
use std::path::Path;
use std::process::Output;

mod capabilities;
mod color;
mod decoder;
mod discovery;
mod error;
mod fallback;
mod frame;
mod job;
mod media_io;
mod probe;
mod process;
mod texture;
mod validate;

pub use capabilities::{
    CodecCapability, FallbackDecodePathCapability, FallbackLadderCapability,
    MacosMediaIoCapabilities, PixelFormatCapability, RuntimeBinaryCapability, RuntimeCapabilities,
    RuntimeCapabilityReport, RuntimeCapabilityStatus, RuntimeFeatureCapability,
    RuntimeFontCapability, RuntimeLicensePosture, RuntimeMediaIoCapabilities,
    TextureInteropCapability, WindowsMediaIoCapabilities, probe_runtime_capabilities,
};
pub use color::{
    ColorDiagnostic, ColorMatrix, ColorPrimaries, ColorRange, ColorTransfer, VideoColorMetadata,
    VideoPixelFormat,
};
pub use decoder::{
    AudioDecodeRequest, AudioDecoder, DecodeError, DecodeErrorKind, VideoDecodeRequest,
    VideoDecoder,
};
pub use discovery::{
    BUNDLED_FFMPEG_DIR_ENV, BinaryKind, DiscoveredBinary, DiscoverySource,
    MAX_STDERR_SUMMARY_BYTES, RuntimeConfig, discover_bundled_runtime_config,
    discover_runtime_config, probe_binary_version, probe_binary_version_with_timeout,
    resolve_binary,
};
pub use error::{DiscoveryError, DiscoveryErrorKind};
pub use fallback::{
    MediaIoFallbackCandidate, MediaIoFallbackDiagnostic, MediaIoFallbackReason,
    MediaIoFallbackSelection, SelectedDecodePath, media_io_fallback_ladder,
    select_media_io_fallback,
};
pub use frame::{
    CpuFrameHandle, DecodedAudioFrame, DecodedVideoFrame, FrameDimensions, FrameHandleId,
    FrameLeaseId, FrameLeaseRequest, FramePool, FramePoolCloseReport, FramePoolError,
    FramePoolErrorKind, FramePoolLimits, FrameReleaseDiagnostic, FrameStorageKind,
    FrameStorageRequest, PlatformFrameHandle, VideoFrameStorage,
};
pub use job::{
    CancelToken, FfmpegJobEvent, FfmpegJobId, FfmpegJobResult, FfmpegJobState, FfmpegProgress,
    FfmpegRuntimeError, FfmpegRuntimeErrorKind, FfmpegRuntimeJob, parse_progress_lines,
    run_export_job,
};
pub use media_io::{
    MediaIoError, MediaIoErrorKind, MediaOpenRequest, MediaProbeReport, MediaProbeRequest,
    MediaProbeService, MediaReader, MediaSession, MediaSessionId, MediaStreamInfo, MediaStreamKind,
    StreamId,
};
pub use probe::{
    MaterialProbeAudio, MaterialProbeError, MaterialProbeErrorKind, MaterialProbeKind,
    MaterialProbeMetadata, MaterialProbeStatus, RationalFrameRate, probe_material_metadata,
};
pub use process::{DEFAULT_PROCESS_TIMEOUT, run_process_with_timeout};
pub use texture::{
    NativeTextureLease, NativeTextureLeaseError, NativeTextureLeaseErrorKind,
    NativeTextureLeaseRegistry, NativeTextureLeaseResourceKind, RuntimeDeviceId, TextureBackend,
    TextureHandle, TextureHandleId,
};
pub use validate::{
    OutputValidationError, OutputValidationErrorKind, OutputValidationExpectation,
    OutputValidationReport, validate_rendered_output,
};

/// Service-boundary trait for executing FFmpeg-family binaries.
///
/// Implementations decide how to launch processes for a given platform. The
/// trait is intentionally narrow in Phase 1: it establishes ownership of the
/// runtime boundary without implementing discovery or render behavior.
pub trait FfmpegExecutor {
    /// Stable label for diagnostics and future compatibility reports.
    fn executor_name(&self) -> &'static str;

    /// Returns whether this executor can attempt to run a binary at `binary`.
    fn can_execute(&self, binary: &Path) -> bool;

    /// Run a version probe with explicit process arguments.
    fn run_version_probe(&self, binary: &Path) -> std::io::Result<Output>;

    /// Run an FFmpeg-family process with explicit process arguments.
    fn run(&self, binary: &Path, args: &[OsString]) -> std::io::Result<Output>;
}
