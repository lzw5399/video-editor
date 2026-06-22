use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Output};
use std::sync::Mutex;

use bindings_node::execute_command;
use bindings_node::preview_export_service::{
    PreviewCacheInvalidationCommand, PreviewFrameArtifactRequest, PreviewSegmentArtifactRequest,
    invalidate_preview_cache_command, request_preview_frame_with_executor,
    request_preview_segment_with_executor,
};
use bindings_node::realtime_preview_service::{
    RealtimePreviewBindingRegistry, RealtimePreviewFrameBindingRequest,
    RealtimePreviewSessionBindingConfig,
};
use draft_model::{
    CommandErrorKind, DirtyDomain, DirtyRange, DirtyRangeSource, Draft, Material, MaterialId,
    MaterialKind, Microseconds, PreviewCacheEntryRef, PreviewOutputProfile, Segment,
    SourceTimerange, TargetTimerange, TextAlignment, TextBox, TextLayoutRegion, TextSegment,
    TextSegmentSource, TextStyle, TextWrapping, Track, TrackKind,
};
use media_runtime::FfmpegExecutor;
use preview_service::PreviewServiceConfig;
use realtime_preview_runtime::{
    PreviewCancellationToken, PreviewRequestMode, RealtimePreviewFallbackReason,
};
use serde_json::json;

#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;

#[test]
fn preview_commands_request_frame_and_segment_through_preview_service_adapter() {
    let temp = tempfile::tempdir().expect("tempdir");
    let executor = FakePreviewExecutor::successful();
    let config = PreviewServiceConfig::new(temp.path(), "/bin/ffmpeg");
    let draft = preview_draft();

    let frame = request_preview_frame_with_executor(
        &executor,
        &config,
        PreviewFrameArtifactRequest {
            draft: draft.clone(),
            target_time: Microseconds::new(500_000),
        },
    )
    .expect("frame preview should route through preview_service");

    let segment = request_preview_segment_with_executor(
        &executor,
        &config,
        PreviewSegmentArtifactRequest {
            draft,
            target_timerange: TargetTimerange::new(
                Microseconds::new(500_000),
                Microseconds::new(100_000),
            ),
        },
    )
    .expect("segment preview should route through preview_service");

    assert_eq!(frame.profile, PreviewOutputProfile::FramePng);
    assert_eq!(frame.mime_type, "image/png");
    assert_eq!(frame.status, draft_model::PreviewStatus::Generated);
    assert_eq!(segment.profile, PreviewOutputProfile::SegmentMp4);
    assert_eq!(segment.mime_type, "video/mp4");
    assert_eq!(executor.calls(), 2);
}

#[test]
fn preview_commands_resolve_project_local_artifact_root_without_renderer_cache_root() {
    let temp = tempfile::tempdir().expect("tempdir");
    let bundle_path = temp.path().join("draft.veproj");
    let executor = FakePreviewExecutor::successful();
    let config = PreviewServiceConfig::new(temp.path().join("fallback-cache"), "/bin/ffmpeg")
        .with_project_artifact_root(&bundle_path);
    let draft = preview_draft();

    let frame = request_preview_frame_with_executor(
        &executor,
        &config,
        PreviewFrameArtifactRequest {
            draft,
            target_time: Microseconds::new(500_000),
        },
    )
    .expect("project-local preview should not require renderer cache root");

    assert!(frame.path.contains("draft.veproj/derived/blobs/preview"));
    assert!(!frame.path.contains("fallback-cache"));
    assert_eq!(executor.calls(), 1);
}

#[test]
fn preview_commands_invalidate_cache_without_mutating_draft() {
    let payload = PreviewCacheInvalidationCommand {
        entries: vec![
            cache_entry_ref(
                "video.png",
                PreviewOutputProfile::FramePng,
                0,
                200_000,
                &["video"],
            ),
            cache_entry_ref(
                "text.png",
                PreviewOutputProfile::FramePng,
                400_000,
                100_000,
                &["text"],
            ),
            cache_entry_ref(
                "audio.mp4",
                PreviewOutputProfile::SegmentMp4,
                800_000,
                200_000,
                &["audio"],
            ),
        ],
        changed_ranges: vec![DirtyRange {
            target_timerange: TargetTimerange::new(
                Microseconds::new(450_000),
                Microseconds::new(50_000),
            ),
            source: DirtyRangeSource::Current,
        }],
        changed_material_ids: vec![MaterialId::new("audio")],
        changed_graph_node_ids: Vec::new(),
        changed_domains: vec![DirtyDomain::PreviewCache],
        runtime_capability_fingerprint: None,
        output_profile_fingerprint: None,
        full_draft: false,
        reason: "accepted edit".to_owned(),
        artifact_schema_version: 0,
        generator_version: String::new(),
    };

    let response = invalidate_preview_cache_command(payload);

    assert_eq!(response.invalidated_count, 2);
    assert_eq!(response.retained_count, 1);
}

#[test]
fn preview_commands_transport_v2_dirty_facts_without_renderer_owned_overrides() {
    let response = invalidate_preview_cache_command(PreviewCacheInvalidationCommand {
        entries: vec![
            cache_entry_ref_with_metadata(
                "video.png",
                0,
                200_000,
                &["video"],
                &["draft:draft-preview:track:video-track:segment:video-a:video"],
                "video-semantic-v1",
            ),
            cache_entry_ref_with_metadata(
                "text.png",
                400_000,
                100_000,
                &["text"],
                &["draft:draft-preview:track:text-track:segment:text-a:text"],
                "text-semantic-v1",
            ),
        ],
        changed_ranges: vec![DirtyRange {
            target_timerange: TargetTimerange::new(
                Microseconds::new(450_000),
                Microseconds::new(50_000),
            ),
            source: DirtyRangeSource::Current,
        }],
        changed_material_ids: Vec::new(),
        changed_graph_node_ids: vec![
            "draft:draft-preview:track:text-track:segment:text-a:text".to_owned(),
        ],
        changed_domains: vec![DirtyDomain::Text, DirtyDomain::PreviewCache],
        runtime_capability_fingerprint: Some("runtime-v1".to_owned()),
        output_profile_fingerprint: Some("profile-v1".to_owned()),
        full_draft: false,
        reason: "accepted text edit".to_owned(),
        artifact_schema_version: 2,
        generator_version: "preview-cache-generator-v2".to_owned(),
    });

    assert_eq!(response.invalidated_count, 1);
    assert_eq!(response.retained_count, 1);
    assert_eq!(
        response.changed_graph_node_ids,
        vec!["draft:draft-preview:track:text-track:segment:text-a:text"]
    );
    assert_eq!(response.dirty_ranges[0].source, DirtyRangeSource::Current);
    assert_eq!(
        response.runtime_capability_fingerprint.as_deref(),
        Some("runtime-v1")
    );
    assert_eq!(response.generator_version, "preview-cache-generator-v2");

    let export_only = invalidate_preview_cache_command(PreviewCacheInvalidationCommand {
        entries: vec![cache_entry_ref_with_metadata(
            "text.png",
            400_000,
            100_000,
            &["text"],
            &["draft:draft-preview:track:text-track:segment:text-a:text"],
            "text-semantic-v1",
        )],
        changed_ranges: vec![DirtyRange {
            target_timerange: TargetTimerange::new(
                Microseconds::new(450_000),
                Microseconds::new(50_000),
            ),
            source: DirtyRangeSource::Current,
        }],
        changed_material_ids: Vec::new(),
        changed_graph_node_ids: Vec::new(),
        changed_domains: vec![DirtyDomain::ExportPrep],
        runtime_capability_fingerprint: Some("runtime-v1".to_owned()),
        output_profile_fingerprint: Some("profile-v1".to_owned()),
        full_draft: false,
        reason: "export-only dirty fact".to_owned(),
        artifact_schema_version: 2,
        generator_version: "preview-cache-generator-v2".to_owned(),
    });

    assert_eq!(export_only.invalidated_count, 0);
    assert_eq!(export_only.retained_count, 1);
}

#[test]
fn generic_preview_commands_are_not_public_command_envelope_api() {
    for command in [
        "requestPreviewDecode",
        "releasePreviewFrame",
        "requestPreviewFrame",
        "requestPreviewSegment",
        "invalidatePreviewCache",
    ] {
        let envelope = execute_command(json!({
            "command": command,
            "payload": { "kind": command },
            "requestId": format!("req-{command}")
        }))
        .expect("generic preview command should return rejection envelope");

        assert_eq!(envelope["ok"], false, "{command}: {envelope:#}");
        assert_eq!(
            envelope["error"]["kind"],
            serde_json::to_value(CommandErrorKind::UnsupportedCommand).unwrap(),
            "{command}: {envelope:#}"
        );
    }
}

#[test]
fn preview_commands_realtime_binding_preserves_fallback_and_cancellation_telemetry() {
    let mut registry = RealtimePreviewBindingRegistry::new();
    let session = registry
        .create_session(RealtimePreviewSessionBindingConfig {
            session_label: "preview-main".to_owned(),
            frame_rate_numerator: 30,
            frame_rate_denominator: 1,
            playback_rate_numerator: 1,
            playback_rate_denominator: 1,
        })
        .expect("session should be created");

    let result = registry
        .request_frame(
            &session.session_id,
            RealtimePreviewFrameBindingRequest {
                target_time_microseconds: 500_000,
                playback_generation: session.playback_generation,
                audio_sync: None,
                queue_latency_ms: 2,
                render_duration_ms: 9,
                mode: PreviewRequestMode::Seek,
                fallback_reason: Some(RealtimePreviewFallbackReason::FfmpegArtifactGenerated),
                cache_hit: true,
                cancellation_token: Some(PreviewCancellationToken::new(9)),
            },
        )
        .expect("frame request should preserve fallback diagnostics");

    assert_eq!(
        result.fallback,
        Some(RealtimePreviewFallbackReason::FfmpegArtifactGenerated)
    );
    assert_eq!(
        result.cancellation_token,
        Some(PreviewCancellationToken::new(9))
    );
    assert!(!result.diagnostics.is_empty());
    assert_eq!(result.telemetry.fallback_count, 1);
    assert_eq!(result.telemetry.cache_hit_count, 1);
    assert!(
        serde_json::to_value(&result)
            .unwrap()
            .get("gpuDevice")
            .is_none()
    );
}

struct FakePreviewExecutor {
    calls: Mutex<usize>,
}

impl FakePreviewExecutor {
    fn successful() -> Self {
        Self {
            calls: Mutex::new(0),
        }
    }

    fn calls(&self) -> usize {
        *self.calls.lock().expect("calls lock")
    }
}

impl FfmpegExecutor for FakePreviewExecutor {
    fn executor_name(&self) -> &'static str {
        "fake-preview-binding-executor"
    }

    fn can_execute(&self, _binary: &Path) -> bool {
        true
    }

    fn run_version_probe(&self, binary: &Path) -> io::Result<Output> {
        self.run(binary, &[])
    }

    fn run(&self, _binary: &Path, args: &[OsString]) -> io::Result<Output> {
        *self.calls.lock().expect("calls lock") += 1;
        let output_path = args
            .last()
            .map(PathBuf::from)
            .expect("preview args should end with output path");
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(output_path, b"preview")?;
        Ok(Output {
            status: success_status(),
            stdout: Vec::new(),
            stderr: Vec::new(),
        })
    }
}

fn preview_draft() -> Draft {
    let mut draft = Draft::new("draft-preview", "Preview");
    draft.materials = vec![
        material("video", MaterialKind::Video, "file:///media/video.mp4"),
        material("text", MaterialKind::Text, "text://title"),
    ];

    let mut video_track = Track::new("video-track", TrackKind::Video, "视频");
    video_track
        .segments
        .push(segment("video-a", "video", 0, 0, 1_000_000));

    let mut text_track = Track::new("text-track", TrackKind::Text, "文字");
    let mut text = segment("text-a", "text", 0, 500_000, 500_000);
    text.text = Some(TextSegment {
        content: "标题".to_owned(),
        source: TextSegmentSource::Text,
        style: TextStyle {
            font_size: 48,
            color: "#ffffff".to_owned(),
            alignment: TextAlignment::Center,
            stroke: None,
            shadow: None,
            background: None,
            ..TextStyle::default()
        },
        text_box: TextBox::default(),
        layout_region: TextLayoutRegion::default(),
        wrapping: TextWrapping::default(),
        bubble: None,
        effect: None,
    });
    text_track.segments.push(text);
    draft.tracks = vec![video_track, text_track];
    draft
}

fn material(material_id: &str, kind: MaterialKind, uri: &str) -> Material {
    let mut material = Material::new(material_id, kind, uri, material_id);
    material.metadata.duration = Some(Microseconds::new(2_000_000));
    match kind {
        MaterialKind::Video => {
            material.metadata.width = Some(1_920);
            material.metadata.height = Some(1_080);
            material.metadata.frame_rate = Some(draft_model::RationalFrameRate::new(30, 1));
            material.metadata.has_video = true;
            material.metadata.has_audio = true;
        }
        MaterialKind::Text => {}
        MaterialKind::Image | MaterialKind::Audio | MaterialKind::Sticker => {}
    }
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

fn cache_entry_ref(
    artifact: &str,
    profile: PreviewOutputProfile,
    start: u64,
    duration: u64,
    material_ids: &[&str],
) -> PreviewCacheEntryRef {
    PreviewCacheEntryRef {
        profile,
        target_timerange: TargetTimerange::new(
            Microseconds::new(start),
            Microseconds::new(duration),
        ),
        material_dependencies: material_ids
            .iter()
            .map(|material_id| MaterialId::new(*material_id))
            .collect(),
        artifact_path: format!("/cache/{artifact}"),
        graph_node_ids: Vec::new(),
        semantic_fingerprint: None,
        input_fingerprint: None,
        output_profile_fingerprint: None,
        runtime_capability_fingerprint: None,
        artifact_schema_version: 0,
        generator_version: String::new(),
    }
}

fn cache_entry_ref_with_metadata(
    artifact: &str,
    start: u64,
    duration: u64,
    material_ids: &[&str],
    graph_node_ids: &[&str],
    semantic_fingerprint: &str,
) -> PreviewCacheEntryRef {
    let mut entry = cache_entry_ref(
        artifact,
        PreviewOutputProfile::FramePng,
        start,
        duration,
        material_ids,
    );
    entry.graph_node_ids = graph_node_ids
        .iter()
        .map(|graph_node_id| (*graph_node_id).to_owned())
        .collect();
    entry.semantic_fingerprint = Some(semantic_fingerprint.to_owned());
    entry.input_fingerprint = Some(format!("{semantic_fingerprint}-input"));
    entry.output_profile_fingerprint = Some("profile-v1".to_owned());
    entry.runtime_capability_fingerprint = Some("runtime-v1".to_owned());
    entry.artifact_schema_version = 2;
    entry.generator_version = "preview-cache-generator-v2".to_owned();
    entry
}

#[cfg(unix)]
fn success_status() -> ExitStatus {
    ExitStatus::from_raw(0)
}
