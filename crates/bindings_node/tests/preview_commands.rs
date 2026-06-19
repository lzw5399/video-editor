use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{ExitStatus, Output};
use std::sync::Mutex;

use bindings_node::execute_command;
use bindings_node::preview_export_service::{
    invalidate_preview_cache_command, request_preview_frame_with_executor,
    request_preview_segment_with_executor,
};
use bindings_node::realtime_preview_service::{
    RealtimePreviewBindingRegistry, RealtimePreviewFrameBindingRequest,
    RealtimePreviewSessionBindingConfig,
};
use draft_model::{
    CommandErrorKind, DecodedPreviewFrameResponse, DirtyDomain, DirtyRange, DirtyRangeSource,
    Draft, InvalidatePreviewCacheCommandPayload, Material, MaterialId, MaterialKind, Microseconds,
    PreviewCacheEntryRef, PreviewFrameStorageKind, PreviewOutputProfile,
    RequestPreviewFrameCommandPayload, RequestPreviewSegmentCommandPayload,
    RuntimeSelectedDecodePath, Segment, SourceTimerange, TargetTimerange, TextAlignment, TextBox,
    TextLayoutRegion, TextSegment, TextSegmentSource, TextStyle, TextWrapping, Track, TrackKind,
};
use media_runtime::FfmpegExecutor;
use preview_service::PreviewServiceConfig;
use realtime_preview_runtime::{
    PreviewCancellationToken, PreviewRequestMode, RealtimePreviewFallbackReason,
};
use serde_json::{Value, json};

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
        RequestPreviewFrameCommandPayload {
            draft: draft.clone(),
            cache_root: Some(temp.path().display().to_string()),
            bundle_path: None,
            target_time: Microseconds::new(500_000),
        },
    )
    .expect("frame preview should route through preview_service");

    let segment = request_preview_segment_with_executor(
        &executor,
        &config,
        RequestPreviewSegmentCommandPayload {
            draft,
            cache_root: Some(temp.path().display().to_string()),
            bundle_path: None,
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
        RequestPreviewFrameCommandPayload {
            draft,
            cache_root: None,
            bundle_path: Some(bundle_path.display().to_string()),
            target_time: Microseconds::new(500_000),
        },
    )
    .expect("project-local preview should not require renderer cache root");

    assert!(frame.path.contains("draft.veproj/derived/blobs/preview"));
    assert!(!frame.path.contains("fallback-cache"));
    assert_eq!(executor.calls(), 1);

    let envelope = execute_command(json!({
        "command": "requestPreviewFrame",
        "payload": {
            "kind": "requestPreviewFrame",
            "draft": preview_draft(),
            "bundlePath": bundle_path,
            "targetTime": 500000
        },
        "requestId": "req-preview-project-root"
    }))
    .expect("preview command should return envelope");

    assert_ne!(envelope["error"]["kind"], "invalidPayload", "{envelope:#}");
}

#[test]
fn preview_commands_invalidate_cache_without_mutating_draft() {
    let payload = InvalidatePreviewCacheCommandPayload {
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

    let envelope = execute_command(json!({
        "command": "invalidatePreviewCache",
        "payload": {
            "kind": "invalidatePreviewCache",
            "entries": [
                {
                    "profile": "framePng",
                    "targetTimerange": { "start": 0, "duration": 200000 },
                    "materialDependencies": ["video"],
                    "artifactPath": "/cache/video.png"
                },
                {
                    "profile": "segmentMp4",
                    "targetTimerange": { "start": 800000, "duration": 200000 },
                    "materialDependencies": ["audio"],
                    "artifactPath": "/cache/audio.mp4"
                }
            ],
            "changedRanges": [],
            "changedMaterialIds": ["audio"],
            "reason": "material changed"
        },
        "requestId": "req-invalidate-preview"
    }))
    .expect("invalidate preview command should return envelope");

    assert_eq!(envelope["ok"], true, "{envelope:#}");
    assert_eq!(envelope["data"]["invalidatedCount"], 1);
    assert_eq!(envelope["data"]["retainedCount"], 1);
    assert_eq!(envelope["error"], Value::Null);
}

#[test]
fn preview_commands_transport_v2_dirty_facts_without_renderer_owned_overrides() {
    let envelope = execute_command(json!({
        "command": "invalidatePreviewCache",
        "payload": {
            "kind": "invalidatePreviewCache",
            "entries": [
                {
                    "profile": "framePng",
                    "targetTimerange": { "start": 0, "duration": 200000 },
                    "materialDependencies": ["video"],
                    "artifactPath": "/cache/video.png",
                    "graphNodeIds": ["draft:draft-preview:track:video-track:segment:video-a:video"],
                    "semanticFingerprint": "video-semantic-v1",
                    "inputFingerprint": "video-input-v1",
                    "outputProfileFingerprint": "profile-v1",
                    "runtimeCapabilityFingerprint": "runtime-v1",
                    "artifactSchemaVersion": 2,
                    "generatorVersion": "preview-cache-generator-v2"
                },
                {
                    "profile": "framePng",
                    "targetTimerange": { "start": 400000, "duration": 100000 },
                    "materialDependencies": ["text"],
                    "artifactPath": "/cache/text.png",
                    "graphNodeIds": ["draft:draft-preview:track:text-track:segment:text-a:text"],
                    "semanticFingerprint": "text-semantic-v1",
                    "inputFingerprint": "text-input-v1",
                    "outputProfileFingerprint": "profile-v1",
                    "runtimeCapabilityFingerprint": "runtime-v1",
                    "artifactSchemaVersion": 2,
                    "generatorVersion": "preview-cache-generator-v2"
                }
            ],
            "changedRanges": [
                { "targetTimerange": { "start": 450000, "duration": 50000 }, "source": "current" }
            ],
            "changedMaterialIds": [],
            "changedGraphNodeIds": ["draft:draft-preview:track:text-track:segment:text-a:text"],
            "changedDomains": ["text", "previewCache"],
            "runtimeCapabilityFingerprint": "runtime-v1",
            "outputProfileFingerprint": "profile-v1",
            "fullDraft": false,
            "reason": "accepted text edit",
            "artifactSchemaVersion": 2,
            "generatorVersion": "preview-cache-generator-v2"
        },
        "requestId": "req-invalidate-preview-v2"
    }))
    .expect("invalidate preview command should return envelope");

    assert_eq!(envelope["ok"], true, "{envelope:#}");
    assert_eq!(envelope["data"]["invalidatedCount"], 1);
    assert_eq!(envelope["data"]["retainedCount"], 1);
    assert_eq!(
        envelope["data"]["changedGraphNodeIds"],
        json!(["draft:draft-preview:track:text-track:segment:text-a:text"])
    );
    assert_eq!(envelope["data"]["dirtyRanges"][0]["source"], "current");
    assert_eq!(
        envelope["data"]["runtimeCapabilityFingerprint"],
        "runtime-v1"
    );
    assert_eq!(
        envelope["data"]["generatorVersion"],
        "preview-cache-generator-v2"
    );

    let export_only = execute_command(json!({
        "command": "invalidatePreviewCache",
        "payload": {
            "kind": "invalidatePreviewCache",
            "entries": [
                {
                    "profile": "framePng",
                    "targetTimerange": { "start": 400000, "duration": 100000 },
                    "materialDependencies": ["text"],
                    "artifactPath": "/cache/text.png",
                    "graphNodeIds": ["draft:draft-preview:track:text-track:segment:text-a:text"],
                    "semanticFingerprint": "text-semantic-v1",
                    "inputFingerprint": "text-input-v1",
                    "outputProfileFingerprint": "profile-v1",
                    "runtimeCapabilityFingerprint": "runtime-v1",
                    "artifactSchemaVersion": 2,
                    "generatorVersion": "preview-cache-generator-v2"
                }
            ],
            "changedRanges": [
                { "targetTimerange": { "start": 450000, "duration": 50000 }, "source": "current" }
            ],
            "changedMaterialIds": [],
            "changedGraphNodeIds": [],
            "changedDomains": ["exportPrep"],
            "runtimeCapabilityFingerprint": "runtime-v1",
            "outputProfileFingerprint": "profile-v1",
            "fullDraft": false,
            "reason": "export-only dirty fact",
            "artifactSchemaVersion": 2,
            "generatorVersion": "preview-cache-generator-v2"
        },
        "requestId": "req-invalidate-preview-export-only"
    }))
    .expect("invalidate preview command should return envelope");

    assert_eq!(export_only["ok"], true, "{export_only:#}");
    assert_eq!(export_only["data"]["invalidatedCount"], 0);
    assert_eq!(export_only["data"]["retainedCount"], 1);
}

#[test]
fn preview_commands_reject_mismatched_preview_command_payload_pair() {
    let envelope = execute_command(json!({
        "command": "requestPreviewFrame",
        "payload": {
            "kind": "requestPreviewSegment",
            "draft": preview_draft(),
            "cacheRoot": "/cache",
            "targetTimerange": { "start": 0, "duration": 100000 }
        },
        "requestId": "req-preview-mismatch"
    }))
    .expect("mismatched preview command should return envelope");

    assert_eq!(envelope["ok"], false);
    assert_eq!(
        envelope["error"]["kind"],
        serde_json::to_value(CommandErrorKind::InvalidPayload).unwrap()
    );
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

#[test]
fn preview_decode_command_returns_handle_metadata_without_full_frame_payloads() {
    let envelope = execute_command(json!({
        "command": "requestPreviewDecode",
        "payload": {
            "kind": "requestPreviewDecode",
            "sessionId": "preview-session-1",
            "draft": preview_draft(),
            "materialId": "video",
            "sourceTime": 250000,
            "playbackGeneration": 7,
            "preferredStorage": "texture",
            "previewDevice": {
                "backend": "d3d11Texture2D",
                "adapterId": "adapter-1",
                "deviceId": "device-1"
            }
        },
        "requestId": "req-preview-decode"
    }))
    .expect("preview decode command should return envelope");

    assert_eq!(envelope["ok"], true, "{envelope:#}");
    let data: DecodedPreviewFrameResponse =
        serde_json::from_value(envelope["data"].clone()).expect("decode response contract");
    assert_eq!(data.storage_kind, PreviewFrameStorageKind::Texture);
    assert_eq!(
        data.selected_path,
        RuntimeSelectedDecodePath::NativeHardwareTexture
    );
    assert!(
        data.frame.frame_handle_id.starts_with("preview-frame-"),
        "preview frame handle IDs must be opaque binding-owned IDs"
    );
    assert_eq!(data.frame.owner_session, "preview-session-1");
    assert_eq!(data.frame.generation, 7);
    assert_eq!(
        data.texture.as_ref().expect("texture metadata").generation,
        7
    );
    assert!(data.texture_compatible);

    let serialized = serde_json::to_string(&envelope).expect("response serializes");
    for forbidden in [
        "nativePointer",
        "rawHandle",
        "ArrayBuffer",
        "Uint8Array",
        "bytes",
        "pixels",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "preview decode response must not expose {forbidden}"
        );
    }
}

#[test]
fn preview_decode_release_rejects_unknown_wrong_session_and_stale_generation_handles() {
    let envelope = execute_command(json!({
        "command": "requestPreviewDecode",
        "payload": {
            "kind": "requestPreviewDecode",
            "sessionId": "preview-session-release",
            "draft": preview_draft(),
            "materialId": "video",
            "sourceTime": 0,
            "playbackGeneration": 3,
            "preferredStorage": "cpu"
        },
        "requestId": "req-preview-decode-release"
    }))
    .expect("preview decode command should return envelope");
    assert_eq!(envelope["ok"], true, "{envelope:#}");
    let data: DecodedPreviewFrameResponse =
        serde_json::from_value(envelope["data"].clone()).expect("decode response contract");

    let wrong_session = execute_command(json!({
        "command": "releasePreviewFrame",
        "payload": {
            "kind": "releasePreviewFrame",
            "sessionId": "other-session",
            "frameHandleId": data.frame.frame_handle_id,
            "playbackGeneration": 3
        },
        "requestId": "req-preview-release-wrong-session"
    }))
    .expect("release command should return envelope");
    assert_eq!(wrong_session["ok"], false);
    assert_eq!(wrong_session["error"]["kind"], "previewServiceFailed");

    let stale = execute_command(json!({
        "command": "releasePreviewFrame",
        "payload": {
            "kind": "releasePreviewFrame",
            "sessionId": "preview-session-release",
            "frameHandleId": data.frame.frame_handle_id,
            "playbackGeneration": 99
        },
        "requestId": "req-preview-release-stale"
    }))
    .expect("release command should return envelope");
    assert_eq!(stale["ok"], false);
    assert_eq!(stale["error"]["kind"], "previewServiceFailed");

    let released = execute_command(json!({
        "command": "releasePreviewFrame",
        "payload": {
            "kind": "releasePreviewFrame",
            "sessionId": "preview-session-release",
            "frameHandleId": data.frame.frame_handle_id,
            "playbackGeneration": 3
        },
        "requestId": "req-preview-release-valid"
    }))
    .expect("release command should return envelope");
    assert_eq!(released["ok"], true, "{released:#}");
    assert_eq!(released["data"]["released"], true);

    let unknown = execute_command(json!({
        "command": "releasePreviewFrame",
        "payload": {
            "kind": "releasePreviewFrame",
            "sessionId": "preview-session-release",
            "frameHandleId": "preview-frame-missing",
            "playbackGeneration": 3
        },
        "requestId": "req-preview-release-unknown"
    }))
    .expect("release command should return envelope");
    assert_eq!(unknown["ok"], false);
    assert_eq!(unknown["error"]["kind"], "previewServiceFailed");
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

#[cfg(unix)]
fn success_status() -> ExitStatus {
    ExitStatus::from_raw(0)
}
