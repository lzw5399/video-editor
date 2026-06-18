use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Output};
use std::sync::Mutex;

use draft_model::{
    Draft, Material, MaterialId, MaterialKind, Microseconds, RationalFrameRate, Segment,
    SourceTimerange, TargetTimerange, Track, TrackKind,
};
use media_runtime::FfmpegExecutor;
use preview_service::{
    RealtimePreviewFrameServiceRequest, RealtimePreviewServiceConfig,
    request_realtime_preview_frame,
};
use realtime_preview_runtime::{
    CpuVideoFrame, DecodedVideoFrameCache, FrameColorInfo, PlaybackGeneration, PreviewRequestMode,
    RealtimePreviewFallbackReason, SoftwareVideoFrameProvider,
};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

#[test]
fn fallback_ladder_reports_no_adapter_surface_and_frame_provider_reasons() {
    let no_adapter = fallback_response(
        RealtimePreviewServiceConfig::new(temp_path(), "/bin/ffmpeg")
            .with_runtime_backend_available(false),
        SoftwareVideoFrameProvider::new(h264_cache()),
    );
    assert_eq!(
        no_adapter.fallback_decision.reason,
        Some(RealtimePreviewFallbackReason::NoGpuAdapter)
    );
    assert_eq!(
        no_adapter.realtime.fallback,
        Some(RealtimePreviewFallbackReason::FfmpegArtifactGenerated)
    );

    let no_surface = fallback_response(
        RealtimePreviewServiceConfig::new(temp_path(), "/bin/ffmpeg").with_surface_available(false),
        SoftwareVideoFrameProvider::new(h264_cache()),
    );
    assert_eq!(
        no_surface.fallback_decision.reason,
        Some(RealtimePreviewFallbackReason::SurfaceUnavailable)
    );

    let no_frame = fallback_response(
        RealtimePreviewServiceConfig::new(temp_path(), "/bin/ffmpeg").with_mock_realtime_backend(),
        SoftwareVideoFrameProvider::new(DecodedVideoFrameCache::new()),
    );
    assert_eq!(
        no_frame.fallback_decision.reason,
        Some(RealtimePreviewFallbackReason::FrameProviderUnavailable)
    );
}

#[test]
fn fallback_ladder_distinguishes_cache_hit_from_ffmpeg_generation() {
    let temp = tempfile::tempdir().expect("cache temp dir");
    let executor = CountingPreviewExecutor::new();
    let config =
        RealtimePreviewServiceConfig::new(temp.path(), "/bin/ffmpeg").with_surface_available(false);
    let mut provider = SoftwareVideoFrameProvider::new(h264_cache());
    let request = service_request();

    let generated = request_realtime_preview_frame(&executor, &config, &request, &mut provider)
        .expect("first unsupported request generates fallback artifact");
    let cached = request_realtime_preview_frame(&executor, &config, &request, &mut provider)
        .expect("second unsupported request reuses fallback artifact");

    assert_eq!(
        generated.realtime.fallback,
        Some(RealtimePreviewFallbackReason::FfmpegArtifactGenerated)
    );
    assert_eq!(
        cached.fallback_decision.reason,
        Some(RealtimePreviewFallbackReason::PreviewArtifactCacheHit)
    );
    assert_eq!(
        cached.realtime.fallback,
        Some(RealtimePreviewFallbackReason::PreviewArtifactCacheHit)
    );
    assert_eq!(cached.realtime.telemetry.cache_hit_count, 1);
    assert_eq!(executor.calls(), 1);
}

fn fallback_response(
    config: RealtimePreviewServiceConfig,
    mut provider: SoftwareVideoFrameProvider,
) -> preview_service::RealtimePreviewServiceFrameResponse {
    let executor = CountingPreviewExecutor::new();
    request_realtime_preview_frame(&executor, &config, &service_request(), &mut provider)
        .expect("fallback request should return response")
}

fn temp_path() -> PathBuf {
    tempfile::tempdir().expect("cache temp dir").keep()
}

struct CountingPreviewExecutor {
    calls: Mutex<usize>,
}

impl CountingPreviewExecutor {
    fn new() -> Self {
        Self {
            calls: Mutex::new(0),
        }
    }

    fn calls(&self) -> usize {
        *self.calls.lock().expect("call count lock")
    }
}

impl FfmpegExecutor for CountingPreviewExecutor {
    fn executor_name(&self) -> &'static str {
        "counting-fallback-preview-executor"
    }

    fn can_execute(&self, _binary: &Path) -> bool {
        true
    }

    fn run_version_probe(&self, binary: &Path) -> io::Result<Output> {
        self.run(binary, &[])
    }

    fn run(&self, _binary: &Path, args: &[OsString]) -> io::Result<Output> {
        *self.calls.lock().expect("call count lock") += 1;
        let output_path = args
            .last()
            .map(PathBuf::from)
            .expect("preview args should end with output path");
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(output_path, b"fallback artifact")?;
        Ok(Output {
            status: success_status(),
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    }
}

fn service_request() -> RealtimePreviewFrameServiceRequest {
    RealtimePreviewFrameServiceRequest {
        draft: video_draft(),
        target_time: Microseconds::ZERO,
        playback_generation: PlaybackGeneration::initial(),
        mode: PreviewRequestMode::FirstFrame,
        cancellation_token: None,
    }
}

fn video_draft() -> Draft {
    let mut draft = Draft::new("draft-fallback-ladder", "Fallback Ladder");
    draft.materials = vec![video_material("h264-material")];
    let mut video_track = Track::new("video-track", TrackKind::Video, "视频");
    video_track
        .segments
        .push(segment("video-a", "h264-material", 0, 0, 1_000_000));
    draft.tracks = vec![video_track];
    draft
}

fn video_material(material_id: &str) -> Material {
    let mut material = Material::new(
        material_id,
        MaterialKind::Video,
        "file:///media/generated-h264.mp4",
        material_id,
    );
    material.metadata.duration = Some(Microseconds::new(1_000_000));
    material.metadata.width = Some(2);
    material.metadata.height = Some(1);
    material.metadata.frame_rate = Some(RationalFrameRate::new(10, 1));
    material.metadata.has_video = true;
    material
}

fn segment(
    segment_id: &str,
    material_id: &str,
    source_start: u64,
    target_start: u64,
    duration: u64,
) -> Segment {
    Segment::new(
        segment_id,
        material_id,
        SourceTimerange::new(Microseconds::new(source_start), Microseconds::new(duration)),
        TargetTimerange::new(Microseconds::new(target_start), Microseconds::new(duration)),
    )
}

fn h264_cache() -> DecodedVideoFrameCache {
    let material_id = MaterialId::new("h264-material");
    let generation = PlaybackGeneration::initial();
    let mut cache = DecodedVideoFrameCache::new();
    cache
        .insert_h264_frames(
            material_id.clone(),
            RationalFrameRate::new(10, 1),
            1,
            vec![(0, rgba_frame(&material_id, 0, generation, [255, 0, 0, 255]))],
        )
        .expect("seeded H.264 cache is valid");
    cache
}

fn rgba_frame(
    material_id: &MaterialId,
    source_position_micros: u64,
    playback_generation: PlaybackGeneration,
    rgba: [u8; 4],
) -> CpuVideoFrame {
    let mut pixels = Vec::new();
    pixels.extend_from_slice(&rgba);
    pixels.extend_from_slice(&rgba);
    CpuVideoFrame::new(
        material_id.clone(),
        Microseconds::new(source_position_micros),
        playback_generation,
        2,
        1,
        8,
        FrameColorInfo::srgb_rgba8(),
        pixels,
    )
    .expect("test frame is valid")
}

#[cfg(unix)]
fn success_status() -> ExitStatus {
    ExitStatus::from_raw(0)
}
