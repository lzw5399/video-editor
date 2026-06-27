use std::path::{Path, PathBuf};
use std::process::Command;

use draft_model::{MaterialKind, Microseconds, TrackKind, validate_draft};
use project_store::{StdPlatformFileSystem, open_project_bundle, project_json_path};
use serde_json::Value;
use testkit::large_timeline::{
    PHASE20_BLOCKING_SEGMENTS_PER_TRACK, PHASE20_DIAGNOSTIC_SEGMENTS_PER_TRACK,
    PHASE20_PRODUCT_SEGMENTS_PER_TRACK, PHASE20_SEGMENT_DURATION_US, Phase20ProductMediaUris,
    assert_no_track_overlaps, build_phase20_product_timeline, phase20_product_timeline_config,
};

const FORBIDDEN_DERIVED_KEYS: &[&str] = &[
    "renderGraph",
    "renderGraphs",
    "ffmpegScript",
    "ffmpegScripts",
    "previewCache",
    "previewCaches",
    "previewFrame",
    "previewFrames",
    "thumbnail",
    "thumbnails",
    "waveform",
    "waveforms",
    "proxyFile",
    "proxyFiles",
    "export",
    "exports",
    "exportJob",
    "exportJobs",
    "runtime",
    "runtimeHandle",
    "runtimeHandles",
    "absoluteTempOutputPath",
];

#[test]
fn phase20_product_fixture_config_matches_locked_scale() {
    let config = phase20_product_timeline_config();

    assert_eq!(PHASE20_PRODUCT_SEGMENTS_PER_TRACK, 180);
    assert_eq!(PHASE20_BLOCKING_SEGMENTS_PER_TRACK, 1_000);
    assert_eq!(PHASE20_DIAGNOSTIC_SEGMENTS_PER_TRACK, 3_000);
    assert_eq!(PHASE20_SEGMENT_DURATION_US, 1_000_000);
    assert_eq!(config.segments_per_track, 180);
    assert_eq!(config.track_count(), 3);
    assert_eq!(config.total_segment_count(), 540);
    assert_eq!(
        config.segment_duration,
        Microseconds::new(PHASE20_SEGMENT_DURATION_US)
    );
    assert_eq!(
        config.target_stride,
        Microseconds::new(PHASE20_SEGMENT_DURATION_US)
    );
}

#[test]
fn phase20_materializer_writes_reopenable_canonical_bundle() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("phase20-long.veproj");
    let media = create_media_paths(temp_dir.path());

    let output = Command::new(phase20_materializer_bin())
        .args([
            "--bundle",
            path_str(&bundle_path),
            "--video",
            path_str(&media.video_path),
            "--audio",
            path_str(&media.audio_path),
        ])
        .output()
        .expect("phase20 materializer should run");

    assert!(
        output.status.success(),
        "materializer failed: status={:?}\nstdout={}\nstderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let summary: Value =
        serde_json::from_slice(&output.stdout).expect("materializer summary should be JSON");
    assert_eq!(summary["bundlePath"], path_str(&bundle_path));
    assert_eq!(
        summary["projectJsonPath"],
        path_str(&project_json_path(&bundle_path))
    );
    assert_eq!(summary["tracks"], 3);
    assert_eq!(summary["segmentsPerTrack"], 180);
    assert_eq!(summary["totalSegments"], 540);
    assert_eq!(summary["durationUs"], 180_000_000);

    let expected = build_phase20_product_timeline(Phase20ProductMediaUris::new(
        path_str(&media.video_path),
        path_str(&media.audio_path),
    ))
    .expect("expected phase 20 fixture should build");
    let reopened = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("materialized project should reopen");
    assert_eq!(reopened.bundle.draft, expected.draft);
    assert!(reopened.warnings.is_empty());
}

#[test]
fn phase20_materializer_rejects_missing_required_arguments() {
    for args in [
        vec!["--video", "video.mp4", "--audio", "audio.wav"],
        vec!["--bundle", "bundle.veproj", "--audio", "audio.wav"],
        vec!["--bundle", "bundle.veproj", "--video", "video.mp4"],
    ] {
        let output = Command::new(phase20_materializer_bin())
            .args(args)
            .output()
            .expect("phase20 materializer should run");
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(
            !output.status.success(),
            "missing required arguments should fail"
        );
        assert!(
            stderr.contains(
                "Usage: phase20_long_fixture --bundle <path> --video <path> --audio <path>"
            ),
            "usage text should be product-readable, stderr={stderr}"
        );
    }
}

#[test]
fn phase20_materializer_project_json_excludes_derived_artifacts() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("phase20-derived-check.veproj");
    let media = create_media_paths(temp_dir.path());

    let output = Command::new(phase20_materializer_bin())
        .args([
            "--bundle",
            path_str(&bundle_path),
            "--video",
            path_str(&media.video_path),
            "--audio",
            path_str(&media.audio_path),
        ])
        .output()
        .expect("phase20 materializer should run");
    assert!(
        output.status.success(),
        "materializer failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let project_json: Value = serde_json::from_str(
        &std::fs::read_to_string(project_json_path(&bundle_path))
            .expect("project.json should be readable"),
    )
    .expect("project.json should parse");
    let violations = collect_forbidden_project_keys(&project_json);
    assert!(
        violations.is_empty(),
        "project.json must not include derived/runtime/export/cache fields: {violations:?}"
    );
}

#[test]
fn phase20_derived_artifact_scan_rejects_nested_project_json_fields() {
    let project_json = serde_json::json!({
        "materials": [
            {
                "materialId": "video-1",
                "metadata": {
                    "previewCache": {
                        "path": "/tmp/derived-preview.png"
                    }
                }
            }
        ],
        "tracks": [
            {
                "segments": [
                    {
                        "segmentId": "segment-1",
                        "exportJob": "derived-job-id"
                    }
                ]
            }
        ]
    });

    let violations = collect_forbidden_project_keys(&project_json);
    assert_eq!(
        violations,
        vec![
            "$.materials[0].metadata.previewCache".to_string(),
            "$.tracks[0].segments[0].exportJob".to_string(),
        ]
    );
}

struct Phase20MediaPaths {
    video_path: PathBuf,
    audio_path: PathBuf,
}

fn create_media_paths(root: &Path) -> Phase20MediaPaths {
    let video_path = root.join("p0-long-av-tone-testsrc.mp4");
    let audio_path = root.join("p0-long-tone.wav");
    std::fs::write(&video_path, b"phase20 video placeholder")
        .expect("video placeholder should be written");
    std::fs::write(&audio_path, b"phase20 audio placeholder")
        .expect("audio placeholder should be written");
    Phase20MediaPaths {
        video_path,
        audio_path,
    }
}

fn phase20_materializer_bin() -> &'static str {
    env!("CARGO_BIN_EXE_phase20_long_fixture")
}

fn path_str(path: &Path) -> &str {
    path.to_str().expect("test path should be UTF-8")
}

fn collect_forbidden_project_keys(value: &Value) -> Vec<String> {
    let mut violations = Vec::new();
    collect_forbidden_project_keys_at(value, "$", &mut violations);
    violations
}

fn collect_forbidden_project_keys_at(value: &Value, path: &str, violations: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (key, child) in map {
                let child_path = format!("{path}.{key}");
                if FORBIDDEN_DERIVED_KEYS.contains(&key.as_str()) {
                    violations.push(child_path.clone());
                }
                collect_forbidden_project_keys_at(child, &child_path, violations);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                collect_forbidden_project_keys_at(child, &format!("{path}[{index}]"), violations);
            }
        }
        _ => {}
    }
}

#[test]
fn phase20_product_fixture_uses_real_video_and_audio_uris() {
    let media = Phase20ProductMediaUris::new(
        "/repo/apps/desktop-electron/tests/fixtures/media/p0-long-av-tone-testsrc.mp4",
        "/repo/apps/desktop-electron/tests/fixtures/media/p0-long-tone.wav",
    );
    let fixture = build_phase20_product_timeline(media.clone())
        .expect("phase 20 product fixture should build");

    let video_materials = fixture
        .draft
        .materials
        .iter()
        .filter(|material| material.kind == MaterialKind::Video)
        .collect::<Vec<_>>();
    let audio_materials = fixture
        .draft
        .materials
        .iter()
        .filter(|material| material.kind == MaterialKind::Audio)
        .collect::<Vec<_>>();

    assert_eq!(video_materials.len(), PHASE20_PRODUCT_SEGMENTS_PER_TRACK);
    assert_eq!(audio_materials.len(), PHASE20_PRODUCT_SEGMENTS_PER_TRACK);
    assert!(
        video_materials
            .iter()
            .all(|material| material.uri == media.video_uri),
        "video materials should use the supplied product media URI"
    );
    assert!(
        audio_materials
            .iter()
            .all(|material| material.uri == media.audio_uri),
        "audio materials should use the supplied product media URI"
    );
    assert!(
        fixture
            .draft
            .materials
            .iter()
            .filter(|material| material.kind != MaterialKind::Text)
            .all(|material| {
                !material.uri.starts_with("video://phase13/")
                    && !material.uri.starts_with("audio://phase13/")
            }),
        "product media materials must not keep synthetic Phase 13 media URIs"
    );
}

#[test]
fn phase20_product_fixture_is_valid_and_overlap_free() {
    let fixture = build_phase20_product_timeline(Phase20ProductMediaUris::new(
        "/repo/apps/desktop-electron/tests/fixtures/media/p0-long-av-tone-testsrc.mp4",
        "/repo/apps/desktop-electron/tests/fixtures/media/p0-long-tone.wav",
    ))
    .expect("phase 20 product fixture should build");

    validate_draft(&fixture.draft).expect("phase 20 product draft should validate");
    assert_no_track_overlaps(&fixture.draft).expect("phase 20 product tracks should not overlap");
    assert_eq!(fixture.draft.tracks.len(), 3);
    assert_eq!(fixture.draft.materials.len(), 540);
    assert_eq!(
        fixture
            .draft
            .tracks
            .iter()
            .map(|track| track.kind)
            .collect::<Vec<_>>(),
        vec![TrackKind::Video, TrackKind::Audio, TrackKind::Text]
    );
}
