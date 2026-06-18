use std::ffi::OsString;
use std::io;
use std::path::Path;
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
    RealtimePreviewBackendUsed, SoftwareVideoFrameProvider,
};

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

#[test]
fn realtime_backend_no_ffmpeg_supported_graph_does_not_call_ffmpeg_executor() {
    let temp = tempfile::tempdir().expect("cache temp dir");
    let executor = PanickingPreviewExecutor::new();
    let config =
        RealtimePreviewServiceConfig::new(temp.path(), "/bin/ffmpeg").with_mock_realtime_backend();
    let mut provider = SoftwareVideoFrameProvider::new(decoded_cache("video-material"));

    for (target_time, mode) in [
        (Microseconds::ZERO, PreviewRequestMode::FirstFrame),
        (Microseconds::new(100_000), PreviewRequestMode::Seek),
        (Microseconds::new(200_000), PreviewRequestMode::Scrub),
    ] {
        let response = request_realtime_preview_frame(
            &executor,
            &config,
            &RealtimePreviewFrameServiceRequest {
                draft: video_draft("draft-realtime-no-ffmpeg"),
                target_time,
                playback_generation: PlaybackGeneration::initial(),
                mode,
                cancellation_token: None,
            },
            &mut provider,
        )
        .expect("supported preview request should route through realtime runtime");

        assert!(matches!(
            response.realtime.backend,
            RealtimePreviewBackendUsed::Mock
                | RealtimePreviewBackendUsed::Gpu
                | RealtimePreviewBackendUsed::Offscreen
        ));
        assert!(response.realtime.presented);
        assert!(response.artifact.is_none());
        assert!(response.ffmpeg_job.is_none());
    }

    assert_eq!(executor.calls(), 0);
}

struct PanickingPreviewExecutor {
    calls: Mutex<usize>,
}

impl PanickingPreviewExecutor {
    fn new() -> Self {
        Self {
            calls: Mutex::new(0),
        }
    }

    fn calls(&self) -> usize {
        *self.calls.lock().expect("call count lock")
    }
}

impl FfmpegExecutor for PanickingPreviewExecutor {
    fn executor_name(&self) -> &'static str {
        "panicking-preview-executor"
    }

    fn can_execute(&self, _binary: &Path) -> bool {
        true
    }

    fn run_version_probe(&self, binary: &Path) -> io::Result<Output> {
        self.run(binary, &[])
    }

    fn run(&self, _binary: &Path, _args: &[OsString]) -> io::Result<Output> {
        *self.calls.lock().expect("call count lock") += 1;
        panic!("supported realtime preview path must not invoke FFmpeg");
    }
}

fn video_draft(draft_id: &str) -> Draft {
    let mut draft = Draft::new(draft_id, "Realtime Preview");
    draft.materials = vec![video_material("video-material")];

    let mut video_track = Track::new("video-track", TrackKind::Video, "视频");
    video_track
        .segments
        .push(segment("video-a", "video-material", 0, 0, 1_000_000));
    draft.tracks = vec![video_track];
    draft
}

fn video_material(material_id: &str) -> Material {
    let mut material = Material::new(
        material_id,
        MaterialKind::Video,
        "file:///media/video.mp4",
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

fn decoded_cache(material_id: &str) -> DecodedVideoFrameCache {
    super_decoded_cache(material_id)
}

fn super_decoded_cache(material_id: &str) -> DecodedVideoFrameCache {
    let material_id = MaterialId::new(material_id);
    let generation = PlaybackGeneration::initial();
    let mut cache = DecodedVideoFrameCache::new();
    cache
        .insert_h264_frames(
            material_id.clone(),
            RationalFrameRate::new(10, 1),
            4,
            vec![
                (0, rgba_frame(&material_id, 0, generation, [255, 0, 0, 255])),
                (
                    1,
                    rgba_frame(&material_id, 100_000, generation, [0, 255, 0, 255]),
                ),
                (
                    2,
                    rgba_frame(&material_id, 200_000, generation, [0, 0, 255, 255]),
                ),
            ],
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
fn _success_status() -> ExitStatus {
    ExitStatus::from_raw(0)
}
