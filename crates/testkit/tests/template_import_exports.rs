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
use draft_model::{bundled_text_font_path, Draft, Microseconds, TargetTimerange};
use engine_core::{normalize_draft, resolve_render_range, EngineProfile};
use ffmpeg_compiler::{compile_ffmpeg_job, CompileContext};
use media_runtime::{
    discover_runtime_config, run_export_job, validate_rendered_output, CancelToken, FfmpegExecutor,
    FfmpegJobState, FfmpegRuntimeJob, OutputValidationExpectation,
    RationalFrameRate as RuntimeFrameRate, RuntimeConfig,
};
use media_runtime_desktop::DesktopFfmpegExecutor;
use project_store::{open_project_bundle, save_project_bundle, StdPlatformFileSystem};
use render_graph::{
    build_render_graph, ExportMp4Preset, OutputDimensions, RenderGraphPlan, RenderOutputProfile,
};
use serde_json::Value;
use testkit::render_compare::{
    extract_rgb_frame_at, probe_phase5_render_capabilities, ComparableFrame, RenderCompareError,
    RenderCompareResult,
};

const EXPECTED_WIDTH: u32 = 1080;
const EXPECTED_HEIGHT: u32 = 1920;
const EXPECTED_FPS: u32 = 30;
const FRAME_DURATION_US: u64 = 33_334;
const EXPORT_DURATION_US: u64 = 500_000;
const GENERATED_MEDIA_SECONDS: u32 = 8;
const GENERATED_AUDIO_SECONDS: u32 = 8;

#[derive(Debug, Clone, Copy)]
struct ExportFixtureCase {
    family: &'static str,
    input_path: &'static str,
    report_path: &'static str,
    import_id: &'static str,
    sample_time_us: u64,
    expected_statuses: &'static [AdaptationStatus],
    expect_audio_stream: Option<bool>,
    evidence: ExportEvidence,
}

#[derive(Debug, Clone, Copy)]
enum ExportEvidence {
    MainVideo,
    PipLayerOrder,
    VisibleText,
    BgmAudio,
    NativeEffectBaseVideo,
    NoRenderableDraft,
}

const EXPORT_CASES: &[ExportFixtureCase] = &[
    ExportFixtureCase {
        family: "main-video",
        input_path: "positive/main-video.json",
        report_path: "main-video.report.json",
        import_id: "import-main-video-export",
        sample_time_us: 1_000_000,
        expected_statuses: &[AdaptationStatus::Supported],
        expect_audio_stream: None,
        evidence: ExportEvidence::MainVideo,
    },
    ExportFixtureCase {
        family: "pip-overlay",
        input_path: "positive/pip-overlay.json",
        report_path: "pip-overlay.report.json",
        import_id: "import-pip-overlay-export",
        sample_time_us: 1_500_000,
        expected_statuses: &[AdaptationStatus::Supported],
        expect_audio_stream: None,
        evidence: ExportEvidence::PipLayerOrder,
    },
    ExportFixtureCase {
        family: "text-sticker",
        input_path: "positive/text-sticker.json",
        report_path: "text-sticker.report.json",
        import_id: "import-text-sticker-export",
        sample_time_us: 900_000,
        expected_statuses: &[
            AdaptationStatus::Supported,
            AdaptationStatus::Approximated,
            AdaptationStatus::Dropped,
        ],
        expect_audio_stream: Some(false),
        evidence: ExportEvidence::VisibleText,
    },
    ExportFixtureCase {
        family: "bgm-audio",
        input_path: "positive/bgm-audio.json",
        report_path: "bgm-audio.report.json",
        import_id: "import-bgm-audio-export",
        sample_time_us: 500_000,
        expected_statuses: &[AdaptationStatus::Supported],
        expect_audio_stream: Some(true),
        evidence: ExportEvidence::BgmAudio,
    },
    ExportFixtureCase {
        family: "missing-resource",
        input_path: "negative/missing-resource.json",
        report_path: "missing-resource.report.json",
        import_id: "import-missing-resource-export",
        sample_time_us: 1_000_000,
        expected_statuses: &[AdaptationStatus::MissingResource, AdaptationStatus::Dropped],
        expect_audio_stream: None,
        evidence: ExportEvidence::NoRenderableDraft,
    },
    ExportFixtureCase {
        family: "native-effect",
        input_path: "negative/native-effect.json",
        report_path: "native-effect.report.json",
        import_id: "import-native-effect-export",
        sample_time_us: 500_000,
        expected_statuses: &[
            AdaptationStatus::NeedsNativeEffect,
            AdaptationStatus::Dropped,
        ],
        expect_audio_stream: None,
        evidence: ExportEvidence::NativeEffectBaseVideo,
    },
];

#[test]
fn template_import_exports_render_fixture_outputs_with_report_evidence() -> RenderCompareResult<()>
{
    let runtime = discover_runtime_config()
        .map_err(|error| RenderCompareError::Runtime(format!("{error}: {}", error.remediation)))?;
    let executor = DesktopFfmpegExecutor::with_timeout(Duration::from_secs(90));
    let capabilities = probe_phase5_render_capabilities(&executor, &runtime)?;
    let sandbox = tempfile::tempdir()?;

    for case in EXPORT_CASES {
        let imported = import_fixture(case, sandbox.path(), &executor, &runtime)?;
        assert_report_matches_snapshot(case, &imported.report)?;
        assert_report_statuses(&imported.report, case.expected_statuses)?;
        assert_project_json_is_canonical(&imported.project_json);

        if matches!(case.evidence, ExportEvidence::NoRenderableDraft) {
            assert!(
                imported.draft.tracks.is_empty(),
                "missing-resource fixture must not hide dropped material behind renderable draft output"
            );
            continue;
        }

        let export_path = sandbox
            .path()
            .join("exports")
            .join(format!("{}.mp4", case.family));
        let export_job = compile_export_job(
            &imported.draft,
            &capabilities,
            &export_path,
            case.sample_time_us,
            EXPORT_DURATION_US,
        )?;
        write_sidecars(&export_job)?;
        run_export_to_completion(
            &runtime,
            &export_path,
            &export_job,
            &format!("phase17-template-import-{}", case.family),
        )?;

        let mut expectation = OutputValidationExpectation::new()
            .with_expected_duration_microseconds(EXPORT_DURATION_US, FRAME_DURATION_US * 3)
            .with_expected_frame_rate(RuntimeFrameRate {
                numerator: EXPECTED_FPS,
                denominator: 1,
            })
            .with_expected_dimensions(EXPECTED_WIDTH, EXPECTED_HEIGHT);
        if let Some(expect_audio) = case.expect_audio_stream {
            expectation = expectation.with_audio_stream(expect_audio);
        }
        validate_rendered_output(&executor, &runtime, &export_path, &expectation)
            .map_err(|error| RenderCompareError::Runtime(error.to_string()))?;

        let frame = extract_rgb_frame_at(
            &executor,
            &runtime,
            &export_path,
            0,
            0,
            EXPECTED_WIDTH,
            EXPECTED_HEIGHT,
        )?;
        assert_export_evidence(case, &imported, &frame)?;
    }

    Ok(())
}

fn import_fixture(
    case: &ExportFixtureCase,
    sandbox: &Path,
    executor: &DesktopFfmpegExecutor,
    runtime: &RuntimeConfig,
) -> RenderCompareResult<ImportedFixture> {
    let case_root = sandbox.join(case.family);
    let bundle_path = case_root.join("imported.veproj");
    let source_root = case_root.join("source");
    fs::create_dir_all(&source_root)?;

    let value = read_fixture_value(case.input_path)?;
    seed_fixture_resources(&value, &source_root, executor, runtime)?;
    let bundle = KaipaiFormulaBundle::from_json_value(value)
        .map_err(|error| RenderCompareError::Runtime(error.to_string()))?;
    let mut options =
        KaipaiImportOptions::new(bundle_path.clone(), source_root, case.import_id.to_owned());
    options.generated_at = Some("2026-06-24T00:00:00Z".to_owned());
    options.resource_mode = ResourceLocalizationMode::CopyRenderableResources;
    options.verify_resource_sha256 = false;
    let mapped = map_kaipai_bundle_to_import_plan(&bundle, options)
        .map_err(|error| RenderCompareError::Runtime(error.to_string()))?;
    let applied = apply_import_plan_to_draft(DraftImportApplicationInput {
        plan: mapped.plan,
        source_kind: mapped.report.source_kind,
        generated_at: mapped.report.generated_at,
        report_items: mapped.report.items,
    })
    .map_err(|error| RenderCompareError::Runtime(error.to_string()))?;

    let saved = save_project_bundle(&StdPlatformFileSystem, &bundle_path, &applied.draft)
        .map_err(|error| RenderCompareError::Runtime(error.to_string()))?;
    let reopened = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .map_err(|error| RenderCompareError::Runtime(error.to_string()))?;
    if reopened.bundle.draft != applied.draft {
        return Err(RenderCompareError::Assertion(
            "saved imported draft should reopen as the same canonical draft".to_owned(),
        ));
    }
    let project_json = fs::read_to_string(saved.project_json_path)?;

    Ok(ImportedFixture {
        draft: applied.draft,
        report: applied.report,
        project_json,
    })
}

struct ImportedFixture {
    draft: Draft,
    report: draft_import::AdaptationReport,
    project_json: String,
}

fn compile_export_job(
    draft: &Draft,
    capabilities: &ffmpeg_compiler::CompilerCapabilities,
    output_path: &Path,
    target_start: u64,
    target_duration: u64,
) -> RenderCompareResult<ffmpeg_compiler::FfmpegJob> {
    let profile = EngineProfile::from_draft_canvas(draft)
        .map_err(|error| RenderCompareError::Runtime(error.to_string()))?;
    let normalized = normalize_draft(draft, &profile)
        .map_err(|error| RenderCompareError::Runtime(error.to_string()))?;
    let range = resolve_render_range(
        &normalized,
        TargetTimerange::new(
            Microseconds::new(target_start),
            Microseconds::new(target_duration),
        ),
    )
    .map_err(|error| RenderCompareError::Runtime(error.to_string()))?;
    let graph = build_render_graph(&normalized, &range)
        .map_err(|error| RenderCompareError::Runtime(error.to_string()))?;
    let plan = RenderGraphPlan::new(
        graph,
        RenderOutputProfile::export_mp4(
            OutputDimensions::new(profile.canvas_width, profile.canvas_height),
            range.frame_rate.clone(),
            TargetTimerange::new(
                Microseconds::new(target_start),
                Microseconds::new(target_duration),
            ),
            ExportMp4Preset::h264_aac_balanced(),
        ),
    )
    .map_err(|error| RenderCompareError::Runtime(error.to_string()))?;
    let artifact_dir = output_path
        .parent()
        .ok_or_else(|| RenderCompareError::Runtime("export output path has no parent".to_owned()))?
        .join("sidecars");
    let context =
        CompileContext::new(output_path, &artifact_dir).with_capabilities(capabilities.clone());
    compile_ffmpeg_job(&plan, &context)
        .map_err(|error| RenderCompareError::Runtime(error.to_string()))
}

fn run_export_to_completion(
    runtime: &RuntimeConfig,
    export_path: &Path,
    export_job: &ffmpeg_compiler::FfmpegJob,
    job_id: &str,
) -> RenderCompareResult<()> {
    let runtime_job = FfmpegRuntimeJob::new(
        job_id,
        runtime.ffmpeg.path.clone(),
        export_job.args.clone(),
        export_path,
    )
    .with_expected_duration_microseconds(EXPORT_DURATION_US)
    .with_timeout(Duration::from_secs(90));
    let export_result = run_export_job(&runtime_job, &CancelToken::new(), |_| {})
        .map_err(|error| RenderCompareError::Runtime(error.to_string()))?;
    if export_result.state != FfmpegJobState::Completed {
        return Err(RenderCompareError::Assertion(format!(
            "expected export job to complete, got {:?}",
            export_result.state
        )));
    }
    Ok(())
}

fn write_sidecars(job: &ffmpeg_compiler::FfmpegJob) -> RenderCompareResult<()> {
    for sidecar in &job.sidecars {
        let path = Path::new(&sidecar.path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, sidecar.contents.as_bytes())?;
    }
    Ok(())
}

fn assert_export_evidence(
    case: &ExportFixtureCase,
    imported: &ImportedFixture,
    frame: &ComparableFrame,
) -> RenderCompareResult<()> {
    match case.evidence {
        ExportEvidence::MainVideo => assert_min_color_pixels(
            frame,
            ColorClass::Blue,
            100_000,
            "main video should be visible",
        ),
        ExportEvidence::PipLayerOrder => {
            let layer_order = imported
                .draft
                .tracks
                .iter()
                .flat_map(|track| {
                    track
                        .segments
                        .iter()
                        .map(|segment| segment.segment_id.as_str())
                })
                .collect::<Vec<_>>();
            assert_eq!(
                layer_order,
                vec!["segment-main-video", "segment-pip-overlay"],
                "PIP fixture must preserve canonical bottom-to-top layer order"
            );
            assert_min_color_pixels(
                frame,
                ColorClass::Blue,
                100_000,
                "PIP export should keep the base layer visible",
            )?;
            assert_min_color_pixels(
                frame,
                ColorClass::Yellow,
                10_000,
                "PIP export should show the top overlay layer",
            )
        }
        ExportEvidence::VisibleText => assert_min_color_pixels(
            frame,
            ColorClass::BrightText,
            1_000,
            "text sticker export should burn visible text pixels",
        ),
        ExportEvidence::BgmAudio => {
            assert!(
                imported.draft.tracks.iter().any(|track| track
                    .segments
                    .iter()
                    .any(|segment| segment.audio.gain_millis > 0)),
                "BGM fixture must carry audible canonical audio gain"
            );
            Ok(())
        }
        ExportEvidence::NativeEffectBaseVideo => {
            assert!(
                imported
                    .report
                    .items
                    .iter()
                    .any(|item| item.status == AdaptationStatus::NeedsNativeEffect),
                "native effect fixture must report needsNativeEffect instead of hidden support"
            );
            assert!(
                imported
                    .draft
                    .tracks
                    .iter()
                    .flat_map(|track| &track.segments)
                    .all(|segment| segment.filters.is_empty()),
                "native effect parameters must not enter canonical filters"
            );
            assert_min_color_pixels(
                frame,
                ColorClass::Blue,
                100_000,
                "native-effect export should render only the supported base video",
            )
        }
        ExportEvidence::NoRenderableDraft => Ok(()),
    }
}

#[derive(Debug, Clone, Copy)]
enum ColorClass {
    Blue,
    Yellow,
    BrightText,
}

fn assert_min_color_pixels(
    frame: &ComparableFrame,
    color: ColorClass,
    min_pixels: usize,
    message: &str,
) -> RenderCompareResult<()> {
    let count = frame
        .rgb24
        .chunks_exact(3)
        .filter(|pixel| match color {
            ColorClass::Blue => pixel[2] > 120 && pixel[0] < 90 && pixel[1] < 170,
            ColorClass::Yellow => pixel[0] > 160 && pixel[1] > 140 && pixel[2] < 120,
            ColorClass::BrightText => pixel[0] > 180 && pixel[1] > 180 && pixel[2] > 180,
        })
        .count();
    if count < min_pixels {
        return Err(RenderCompareError::Assertion(format!(
            "{message}: expected at least {min_pixels} matching pixels, got {count}"
        )));
    }
    Ok(())
}

fn assert_report_matches_snapshot(
    case: &ExportFixtureCase,
    actual: &draft_import::AdaptationReport,
) -> RenderCompareResult<()> {
    let expected: Value = serde_json::from_str(&fs::read_to_string(
        fixture_root()
            .join("expected-reports")
            .join(case.report_path),
    )?)?;
    let actual = serde_json::to_value(actual)?;
    if actual != expected {
        return Err(RenderCompareError::Assertion(format!(
            "adaptation report mismatch for {}: expected {expected:#}, got {actual:#}",
            case.family
        )));
    }
    Ok(())
}

fn assert_report_statuses(
    report: &draft_import::AdaptationReport,
    expected: &[AdaptationStatus],
) -> RenderCompareResult<()> {
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
    if actual != expected {
        return Err(RenderCompareError::Assertion(format!(
            "expected report statuses {expected:?}, got {actual:?}"
        )));
    }
    Ok(())
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
    value: &Value,
    source_root: &Path,
    executor: &DesktopFfmpegExecutor,
    runtime: &RuntimeConfig,
) -> RenderCompareResult<()> {
    for resource in collect_resource_refs(value) {
        let path = source_root.join(&resource.uri);
        if path.exists() {
            continue;
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        match resource.kind.as_str() {
            "video" => generate_video_fixture(executor, runtime, &path, color_for(&resource.uri))?,
            "audio" => generate_audio_fixture(executor, runtime, &path)?,
            "image" | "sticker" => {
                generate_image_fixture(executor, runtime, &path, color_for(&resource.uri))?
            }
            "font" => {
                fs::copy(bundled_text_font_path(), &path)?;
            }
            other => {
                return Err(RenderCompareError::Runtime(format!(
                    "unsupported fixture resource kind {other} for {}",
                    resource.uri
                )));
            }
        }
    }
    Ok(())
}

#[derive(Debug)]
struct ResourceRef {
    uri: String,
    kind: String,
}

fn collect_resource_refs(value: &Value) -> Vec<ResourceRef> {
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

fn collect_resource_ref(value: &Value, refs: &mut BTreeMap<String, String>) {
    let Some(uri) = value.get("uri").and_then(Value::as_str) else {
        return;
    };
    let Some(kind) = value.get("kind").and_then(Value::as_str) else {
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
) -> RenderCompareResult<()> {
    run_ffmpeg(
        executor,
        runtime,
        &[
            "-hide_banner",
            "-y",
            "-f",
            "lavfi",
            "-i",
            &format!(
                "color=c={color}:size=320x568:rate={EXPECTED_FPS}:duration={GENERATED_MEDIA_SECONDS}"
            ),
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
    )
}

fn generate_audio_fixture(
    executor: &DesktopFfmpegExecutor,
    runtime: &RuntimeConfig,
    output_path: &Path,
) -> RenderCompareResult<()> {
    run_ffmpeg(
        executor,
        runtime,
        &[
            "-hide_banner",
            "-y",
            "-f",
            "lavfi",
            "-i",
            &format!("sine=frequency=660:sample_rate=48000:duration={GENERATED_AUDIO_SECONDS}"),
            "-c:a",
            "aac",
            "-b:a",
            "96k",
        ],
        output_path,
    )
}

fn generate_image_fixture(
    executor: &DesktopFfmpegExecutor,
    runtime: &RuntimeConfig,
    output_path: &Path,
    color: &'static str,
) -> RenderCompareResult<()> {
    run_ffmpeg(
        executor,
        runtime,
        &[
            "-hide_banner",
            "-y",
            "-f",
            "lavfi",
            "-i",
            &format!("color=c={color}:size=320x568:duration=1"),
            "-frames:v",
            "1",
        ],
        output_path,
    )
}

fn run_ffmpeg(
    executor: &DesktopFfmpegExecutor,
    runtime: &RuntimeConfig,
    args: &[&str],
    output_path: &Path,
) -> RenderCompareResult<()> {
    let mut args = args.iter().map(OsString::from).collect::<Vec<_>>();
    args.push(output_path.as_os_str().to_owned());
    let output = executor.run(&runtime.ffmpeg.path, &args).map_err(|error| {
        RenderCompareError::Runtime(format!(
            "failed to run FFmpeg fixture generation at {}: {error}",
            runtime.ffmpeg.path.display()
        ))
    })?;
    if !output.status.success() {
        return Err(RenderCompareError::Runtime(format!(
            "FFmpeg fixture generation failed: stdout=`{}` stderr=`{}`",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )));
    }
    Ok(())
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

fn assert_project_json_is_canonical(project_json: &str) {
    let value: Value = serde_json::from_str(project_json).expect("project JSON should parse");
    assert!(value.get("materials").is_some());
    let serialized = serde_json::to_string(&value).expect("project JSON should serialize");
    for forbidden in [
        "templateId",
        "recipeId",
        "formulaTaskId",
        "formulaRequestId",
        "rawFormula",
        "\"formula\"",
        "safeArea",
        "remoteRuntimeUrl",
        "remoteRenderUrl",
        "renderUrl",
        "http://",
        "https://",
        "kaipai",
        "provider",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "project.json leaked provider/runtime evidence {forbidden}: {serialized}"
        );
    }
}

fn read_fixture_value(path: &str) -> RenderCompareResult<Value> {
    Ok(serde_json::from_str(&fs::read_to_string(
        fixture_root().join(path),
    )?)?)
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures/kaipai")
}
