use std::{
    collections::BTreeMap,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use adapter_kaipai::{map_kaipai_bundle_to_import_plan, KaipaiFormulaBundle, KaipaiImportOptions};
use draft_import::{
    apply_import_plan_to_draft, AdaptationStatus, DraftImportApplicationInput,
    ResourceLocalizationMode,
};
use draft_model::{bundled_text_font_path, Draft, Microseconds};
use media_runtime::{discover_runtime_config, FfmpegExecutor, RuntimeConfig};
use media_runtime_desktop::DesktopFfmpegExecutor;
use realtime_preview_runtime::{
    prepare_realtime_preview_graph, RealtimePreviewCapabilityClassifier, RealtimePreviewGraphInput,
    RealtimePreviewGraphSupport, RealtimePreviewSupport,
};
use render_graph::OutputDimensions;

const PREVIEW_WIDTH: u32 = 540;
const PREVIEW_HEIGHT: u32 = 960;
const GENERATED_MEDIA_SECONDS: u32 = 8;

#[derive(Debug, Clone, Copy)]
struct PreviewFixtureCase {
    family: &'static str,
    input_path: &'static str,
    import_id: &'static str,
    target_time_us: u64,
    expected_statuses: &'static [AdaptationStatus],
}

const PREVIEW_CASES: &[PreviewFixtureCase] = &[
    PreviewFixtureCase {
        family: "main-video",
        input_path: "positive/main-video.json",
        import_id: "import-main-video-preview",
        target_time_us: 1_000_000,
        expected_statuses: &[AdaptationStatus::Supported],
    },
    PreviewFixtureCase {
        family: "pip-overlay",
        input_path: "positive/pip-overlay.json",
        import_id: "import-pip-overlay-preview",
        target_time_us: 1_500_000,
        expected_statuses: &[AdaptationStatus::Supported],
    },
    PreviewFixtureCase {
        family: "text-sticker",
        input_path: "positive/text-sticker.json",
        import_id: "import-text-sticker-preview",
        target_time_us: 900_000,
        expected_statuses: &[
            AdaptationStatus::Supported,
            AdaptationStatus::Approximated,
            AdaptationStatus::Dropped,
        ],
    },
    PreviewFixtureCase {
        family: "bgm-audio",
        input_path: "positive/bgm-audio.json",
        import_id: "import-bgm-audio-preview",
        target_time_us: 500_000,
        expected_statuses: &[AdaptationStatus::Supported],
    },
];

#[test]
fn template_import_preview_uses_realtime_render_graph_without_fallback_evidence() {
    let runtime = discover_runtime_config().expect(
        "ffmpeg and ffprobe must be available in the bundled runtime directory for fixture media",
    );
    let executor = DesktopFfmpegExecutor::with_timeout(Duration::from_secs(90));
    let sandbox = tempfile::tempdir().expect("preview sandbox should create");

    for case in PREVIEW_CASES {
        let imported = import_fixture(case, sandbox.path(), &executor, &runtime);
        assert_report_statuses(&imported.report, case.expected_statuses);

        let prepared = prepare_realtime_preview_graph(RealtimePreviewGraphInput {
            draft: imported.draft,
            target_time: Microseconds::new(case.target_time_us),
            preview_dimensions: OutputDimensions::new(PREVIEW_WIDTH, PREVIEW_HEIGHT),
        })
        .unwrap_or_else(|error| panic!("{} preview graph should prepare: {error}", case.family));
        let report =
            RealtimePreviewCapabilityClassifier::supported_for_tests().classify(&prepared.graph);

        assert_eq!(
            report.support,
            RealtimePreviewGraphSupport::Supported,
            "{} imported fixture should be supported by realtime render-graph preview: {report:#?}",
            case.family
        );
        assert_no_realtime_fallback_evidence(case.family, &report);
        assert!(
            !prepared.graph.video_layers.is_empty()
                || !prepared.graph.text_overlays.is_empty()
                || !prepared.graph.audio_mixes.is_empty(),
            "{} imported fixture should produce render-graph preview work",
            case.family
        );
    }
}

#[test]
fn template_import_preview_rejects_unavailable_runtime_or_surface_as_success() {
    let runtime = discover_runtime_config().expect(
        "ffmpeg and ffprobe must be available in the bundled runtime directory for fixture media",
    );
    let executor = DesktopFfmpegExecutor::with_timeout(Duration::from_secs(90));
    let sandbox = tempfile::tempdir().expect("preview fallback sandbox should create");
    let case = &PREVIEW_CASES[0];
    let imported = import_fixture(case, sandbox.path(), &executor, &runtime);
    let prepared = prepare_realtime_preview_graph(RealtimePreviewGraphInput {
        draft: imported.draft,
        target_time: Microseconds::new(case.target_time_us),
        preview_dimensions: OutputDimensions::new(PREVIEW_WIDTH, PREVIEW_HEIGHT),
    })
    .expect("main-video preview graph should prepare");

    let runtime_missing = RealtimePreviewCapabilityClassifier::supported_for_tests()
        .with_runtime_backend_available(false)
        .classify(&prepared.graph);
    assert_eq!(
        runtime_missing.support,
        RealtimePreviewGraphSupport::Unsupported,
        "missing realtime backend must fail closed instead of counting fallback success"
    );
    assert!(
        runtime_missing
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.fallback_used),
        "missing runtime report must contain fallback diagnostics"
    );

    let surface_missing = RealtimePreviewCapabilityClassifier::supported_for_tests()
        .with_surface_available(false)
        .classify(&prepared.graph);
    assert_eq!(
        surface_missing.support,
        RealtimePreviewGraphSupport::Degraded,
        "missing native surface must be degraded and cannot satisfy product preview success"
    );
    assert!(
        surface_missing
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.fallback_used),
        "missing surface report must contain fallback diagnostics"
    );
}

fn assert_no_realtime_fallback_evidence(
    family: &str,
    report: &realtime_preview_runtime::RealtimePreviewCapabilityReport,
) {
    for diagnostic in &report.diagnostics {
        assert!(
            !diagnostic.fallback_used,
            "{family} preview used fallback diagnostic: {diagnostic:#?}"
        );
        assert!(
            !matches!(
                diagnostic.support,
                RealtimePreviewSupport::Unsupported { .. }
                    | RealtimePreviewSupport::Degraded { .. }
            ),
            "{family} preview diagnostic is not supported: {diagnostic:#?}"
        );
        let serialized = serde_json::to_string(diagnostic).expect("diagnostic should serialize");
        for forbidden in [
            "PreviewArtifact",
            "FfmpegArtifact",
            "MediaIoNativeCpuFrame",
            "MediaIoFfmpegCpuFrame",
            "MediaIoPreviewArtifact",
            "Android",
            "android",
            "mock",
            "oracle",
        ] {
            assert!(
                !serialized.contains(forbidden),
                "{family} preview diagnostic leaked forbidden fallback evidence {forbidden}: {serialized}"
            );
        }
    }
}

fn import_fixture(
    case: &PreviewFixtureCase,
    sandbox: &Path,
    executor: &DesktopFfmpegExecutor,
    runtime: &RuntimeConfig,
) -> ImportedFixture {
    let case_root = sandbox.join(case.family);
    let bundle_path = case_root.join("imported.veproj");
    let source_root = case_root.join("source");
    fs::create_dir_all(&source_root).expect("source root should create");
    fs::create_dir_all(&bundle_path).expect("bundle path should create before localization");

    let value = read_fixture_value(case.input_path);
    seed_fixture_resources(&value, &source_root, executor, runtime);
    let bundle = KaipaiFormulaBundle::from_json_value(value).expect("fixture should parse");
    let mut options = KaipaiImportOptions::new(bundle_path, source_root, case.import_id.to_owned());
    options.generated_at = Some("2026-06-24T00:00:00Z".to_owned());
    options.resource_mode = ResourceLocalizationMode::CopyRenderableResources;
    options.verify_resource_sha256 = false;
    let mapped = map_kaipai_bundle_to_import_plan(&bundle, options)
        .unwrap_or_else(|error| panic!("{} should map: {error}", case.family));
    let applied = apply_import_plan_to_draft(DraftImportApplicationInput {
        plan: mapped.plan,
        source_kind: mapped.report.source_kind,
        generated_at: mapped.report.generated_at,
        report_items: mapped.report.items,
    })
    .unwrap_or_else(|error| panic!("{} should apply to draft: {error}", case.family));

    ImportedFixture {
        draft: applied.draft,
        report: applied.report,
    }
}

struct ImportedFixture {
    draft: Draft,
    report: draft_import::AdaptationReport,
}

fn assert_report_statuses(report: &draft_import::AdaptationReport, expected: &[AdaptationStatus]) {
    let mut actual = report
        .items
        .iter()
        .map(|item| item.status)
        .map(status_label)
        .collect::<Vec<_>>();
    actual.sort_unstable();
    actual.dedup();
    let mut expected = expected
        .iter()
        .copied()
        .map(status_label)
        .collect::<Vec<_>>();
    expected.sort_unstable();
    expected.dedup();
    assert_eq!(actual, expected);
}

fn status_label(status: AdaptationStatus) -> &'static str {
    match status {
        AdaptationStatus::Supported => "supported",
        AdaptationStatus::Approximated => "approximated",
        AdaptationStatus::Dropped => "dropped",
        AdaptationStatus::MissingResource => "missingResource",
        AdaptationStatus::NeedsNativeEffect => "needsNativeEffect",
    }
}

fn seed_fixture_resources(
    value: &serde_json::Value,
    source_root: &Path,
    executor: &DesktopFfmpegExecutor,
    runtime: &RuntimeConfig,
) {
    for resource in collect_resource_refs(value) {
        let path = source_root.join(&resource.uri);
        if path.exists() {
            continue;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("resource parent should create");
        }
        match resource.kind.as_str() {
            "video" => generate_video_fixture(executor, runtime, &path, color_for(&resource.uri)),
            "audio" => generate_audio_fixture(executor, runtime, &path),
            "image" | "sticker" => {
                generate_image_fixture(executor, runtime, &path, color_for(&resource.uri))
            }
            "font" => {
                fs::copy(bundled_text_font_path(), &path).expect("bundled font should copy");
            }
            other => panic!(
                "unsupported fixture resource kind {other} for {}",
                resource.uri
            ),
        }
    }
}

#[derive(Debug)]
struct ResourceRef {
    uri: String,
    kind: String,
}

fn collect_resource_refs(value: &serde_json::Value) -> Vec<ResourceRef> {
    let mut refs = BTreeMap::<String, String>::new();
    if let Some(source) = value.get("sourceMedia") {
        collect_resource_ref(source, &mut refs);
    }
    for resource in value["directMaterials"].as_array().into_iter().flatten() {
        collect_resource_ref(resource, &mut refs);
    }
    for resource in value["resources"].as_array().into_iter().flatten() {
        collect_resource_ref(resource, &mut refs);
    }
    refs.into_iter()
        .map(|(uri, kind)| ResourceRef { uri, kind })
        .collect()
}

fn collect_resource_ref(value: &serde_json::Value, refs: &mut BTreeMap<String, String>) {
    let Some(uri) = value.get("uri").and_then(serde_json::Value::as_str) else {
        return;
    };
    let Some(kind) = value.get("kind").and_then(serde_json::Value::as_str) else {
        return;
    };
    refs.entry(uri.to_owned())
        .or_insert_with(|| kind.to_owned());
}

fn generate_video_fixture(
    executor: &DesktopFfmpegExecutor,
    runtime: &RuntimeConfig,
    output_path: &Path,
    color: &'static str,
) {
    run_ffmpeg(
        executor,
        runtime,
        &[
            "-hide_banner",
            "-y",
            "-f",
            "lavfi",
            "-i",
            &format!("color=c={color}:size=1080x1920:rate=30:duration={GENERATED_MEDIA_SECONDS}"),
            "-f",
            "lavfi",
            "-i",
            &format!("sine=frequency=440:sample_rate=48000:duration={GENERATED_MEDIA_SECONDS}"),
            "-shortest",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-b:a",
            "96k",
        ],
        output_path,
    );
}

fn generate_audio_fixture(
    executor: &DesktopFfmpegExecutor,
    runtime: &RuntimeConfig,
    output_path: &Path,
) {
    run_ffmpeg(
        executor,
        runtime,
        &[
            "-hide_banner",
            "-y",
            "-f",
            "lavfi",
            "-i",
            &format!("sine=frequency=660:sample_rate=48000:duration={GENERATED_MEDIA_SECONDS}"),
            "-c:a",
            "aac",
            "-b:a",
            "96k",
        ],
        output_path,
    );
}

fn generate_image_fixture(
    executor: &DesktopFfmpegExecutor,
    runtime: &RuntimeConfig,
    output_path: &Path,
    color: &'static str,
) {
    run_ffmpeg(
        executor,
        runtime,
        &[
            "-hide_banner",
            "-y",
            "-f",
            "lavfi",
            "-i",
            &format!("color=c={color}:size=1080x1920:duration=1"),
            "-frames:v",
            "1",
        ],
        output_path,
    );
}

fn run_ffmpeg(
    executor: &DesktopFfmpegExecutor,
    runtime: &RuntimeConfig,
    args: &[&str],
    output_path: &Path,
) {
    let mut args = args.iter().map(OsString::from).collect::<Vec<_>>();
    args.push(output_path.as_os_str().to_owned());
    let output = executor
        .run(&runtime.ffmpeg.path, &args)
        .unwrap_or_else(|error| {
            panic!(
                "failed to run FFmpeg fixture generation at {}: {error}",
                runtime.ffmpeg.path.display()
            )
        });
    assert!(
        output.status.success(),
        "FFmpeg fixture generation failed: stdout=`{}` stderr=`{}`",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn color_for(uri: &str) -> &'static str {
    if uri.contains("pip") {
        "0xffd400"
    } else if uri.contains("background") {
        "0x2ea043"
    } else {
        "0x1f6feb"
    }
}

fn read_fixture_value(path: &str) -> serde_json::Value {
    serde_json::from_str(
        &fs::read_to_string(fixture_root().join(path)).expect("fixture should read"),
    )
    .expect("fixture should parse")
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures/kaipai")
}
