use std::ffi::OsString;
use std::io;
use std::path::Path;
use std::process::Output;
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
    CpuVideoFrame, DecodedVideoFrameCache, FrameColorInfo, PlaybackGeneration,
    PreviewCancellationToken, PreviewRequestMode, SoftwareVideoFrameProvider,
};

#[test]
fn cancellation_telemetry_rejects_supported_and_fallback_results_without_artifacts() {
    let temp = tempfile::tempdir().expect("cache temp dir");
    let executor = CountingPreviewExecutor::new();
    let config =
        RealtimePreviewServiceConfig::new(temp.path(), "/bin/ffmpeg").with_mock_realtime_backend();
    let token = PreviewCancellationToken::new(7);
    config
        .cancel_request(token)
        .expect("test cancellation token is registered");
    let mut provider = SoftwareVideoFrameProvider::new(h264_cache());

    let response = request_realtime_preview_frame(
        &executor,
        &config,
        &RealtimePreviewFrameServiceRequest {
            draft: video_draft(),
            target_time: Microseconds::ZERO,
            playback_generation: PlaybackGeneration::initial(),
            mode: PreviewRequestMode::Seek,
            cancellation_token: Some(token),
        },
        &mut provider,
    )
    .expect("canceled supported request returns telemetry");

    assert!(response.realtime.canceled);
    assert!(!response.realtime.presented);
    assert!(response.artifact.is_none());
    assert_eq!(response.realtime.telemetry.canceled_request_count, 1);
    assert_eq!(executor.calls(), 0);
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
        "counting-cancel-preview-executor"
    }

    fn can_execute(&self, _binary: &Path) -> bool {
        true
    }

    fn run_version_probe(&self, binary: &Path) -> io::Result<Output> {
        self.run(binary, &[])
    }

    fn run(&self, _binary: &Path, _args: &[OsString]) -> io::Result<Output> {
        *self.calls.lock().expect("call count lock") += 1;
        panic!("canceled preview requests must not generate artifacts");
    }
}

fn video_draft() -> Draft {
    let mut draft = Draft::new("draft-cancel-telemetry", "Cancel Telemetry");
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
