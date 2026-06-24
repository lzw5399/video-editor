use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Path, PathBuf},
};

use adapter_kaipai::KaipaiFormulaBundle;
use draft_import::{
    AdaptationCategory, AdaptationReport, AdaptationReportItem, AdaptationSeverity,
    AdaptationStatus, AdaptationTargetKind, AdaptationTargetRef, ExternalProvenanceRef,
};
use serde_json::{Value, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum FixturePolarity {
    Positive,
    Negative,
}

#[derive(Debug, Clone, Copy)]
struct ExpectedPlanOutcome {
    canvas: Option<ExpectedCanvas>,
    materials: &'static [&'static str],
    tracks: &'static [ExpectedTrack],
    segments: &'static [ExpectedSegment],
}

#[derive(Debug, Clone, Copy)]
struct ExpectedCanvas {
    width: u64,
    height: u64,
    frame_rate: u64,
}

#[derive(Debug, Clone, Copy)]
struct ExpectedTrack {
    id: &'static str,
    kind: &'static str,
    z_order: u64,
}

#[derive(Debug, Clone, Copy)]
struct ExpectedSegment {
    id: &'static str,
    material_id: &'static str,
    source_duration_ms: u64,
    target_duration_ms: u64,
}

#[derive(Debug, Clone, Copy)]
struct FixtureCase {
    family: &'static str,
    polarity: FixturePolarity,
    input_path: &'static str,
    report_path: &'static str,
    expected_statuses: &'static [AdaptationStatus],
    expected_plan: ExpectedPlanOutcome,
}

const MAIN_VIDEO_PLAN: ExpectedPlanOutcome = ExpectedPlanOutcome {
    canvas: Some(ExpectedCanvas {
        width: 1080,
        height: 1920,
        frame_rate: 30,
    }),
    materials: &["material-main-video"],
    tracks: &[ExpectedTrack {
        id: "track-main-video",
        kind: "video",
        z_order: 0,
    }],
    segments: &[ExpectedSegment {
        id: "segment-main-video",
        material_id: "material-main-video",
        source_duration_ms: 6000,
        target_duration_ms: 4000,
    }],
};

const PIP_OVERLAY_PLAN: ExpectedPlanOutcome = ExpectedPlanOutcome {
    canvas: Some(ExpectedCanvas {
        width: 1080,
        height: 1920,
        frame_rate: 30,
    }),
    materials: &["material-main-video", "material-pip-overlay"],
    tracks: &[
        ExpectedTrack {
            id: "track-main-video",
            kind: "video",
            z_order: 0,
        },
        ExpectedTrack {
            id: "track-pip-overlay",
            kind: "video",
            z_order: 20,
        },
    ],
    segments: &[
        ExpectedSegment {
            id: "segment-main-video",
            material_id: "material-main-video",
            source_duration_ms: 5000,
            target_duration_ms: 5000,
        },
        ExpectedSegment {
            id: "segment-pip-overlay",
            material_id: "material-pip-overlay",
            source_duration_ms: 2600,
            target_duration_ms: 2600,
        },
    ],
};

const TEXT_STICKER_PLAN: ExpectedPlanOutcome = ExpectedPlanOutcome {
    canvas: Some(ExpectedCanvas {
        width: 1080,
        height: 1920,
        frame_rate: 30,
    }),
    materials: &["material-text-sticker"],
    tracks: &[ExpectedTrack {
        id: "track-text-sticker",
        kind: "text",
        z_order: 30,
    }],
    segments: &[ExpectedSegment {
        id: "segment-text-sticker",
        material_id: "material-text-sticker",
        source_duration_ms: 3200,
        target_duration_ms: 3200,
    }],
};

const BGM_AUDIO_PLAN: ExpectedPlanOutcome = ExpectedPlanOutcome {
    canvas: Some(ExpectedCanvas {
        width: 1080,
        height: 1920,
        frame_rate: 30,
    }),
    materials: &["material-bgm-audio"],
    tracks: &[ExpectedTrack {
        id: "track-bgm-audio",
        kind: "audio",
        z_order: 100,
    }],
    segments: &[ExpectedSegment {
        id: "segment-bgm-audio",
        material_id: "material-bgm-audio",
        source_duration_ms: 6200,
        target_duration_ms: 6200,
    }],
};

const MISSING_RESOURCE_PLAN: ExpectedPlanOutcome = ExpectedPlanOutcome {
    canvas: Some(ExpectedCanvas {
        width: 1080,
        height: 1920,
        frame_rate: 30,
    }),
    materials: &[],
    tracks: &[],
    segments: &[],
};

const NATIVE_EFFECT_PLAN: ExpectedPlanOutcome = ExpectedPlanOutcome {
    canvas: Some(ExpectedCanvas {
        width: 1080,
        height: 1920,
        frame_rate: 30,
    }),
    materials: &["material-main-video"],
    tracks: &[ExpectedTrack {
        id: "track-main-video",
        kind: "video",
        z_order: 0,
    }],
    segments: &[ExpectedSegment {
        id: "segment-main-video",
        material_id: "material-main-video",
        source_duration_ms: 4000,
        target_duration_ms: 4000,
    }],
};

const FIXTURE_CASES: &[FixtureCase] = &[
    FixtureCase {
        family: "main-video",
        polarity: FixturePolarity::Positive,
        input_path: "positive/main-video.json",
        report_path: "main-video.report.json",
        expected_statuses: &[AdaptationStatus::Supported],
        expected_plan: MAIN_VIDEO_PLAN,
    },
    FixtureCase {
        family: "pip-overlay",
        polarity: FixturePolarity::Positive,
        input_path: "positive/pip-overlay.json",
        report_path: "pip-overlay.report.json",
        expected_statuses: &[AdaptationStatus::Supported],
        expected_plan: PIP_OVERLAY_PLAN,
    },
    FixtureCase {
        family: "text-sticker",
        polarity: FixturePolarity::Positive,
        input_path: "positive/text-sticker.json",
        report_path: "text-sticker.report.json",
        expected_statuses: &[AdaptationStatus::Supported, AdaptationStatus::Approximated],
        expected_plan: TEXT_STICKER_PLAN,
    },
    FixtureCase {
        family: "bgm-audio",
        polarity: FixturePolarity::Positive,
        input_path: "positive/bgm-audio.json",
        report_path: "bgm-audio.report.json",
        expected_statuses: &[AdaptationStatus::Supported],
        expected_plan: BGM_AUDIO_PLAN,
    },
    FixtureCase {
        family: "missing-resource",
        polarity: FixturePolarity::Negative,
        input_path: "negative/missing-resource.json",
        report_path: "missing-resource.report.json",
        expected_statuses: &[AdaptationStatus::MissingResource, AdaptationStatus::Dropped],
        expected_plan: MISSING_RESOURCE_PLAN,
    },
    FixtureCase {
        family: "native-effect",
        polarity: FixturePolarity::Negative,
        input_path: "negative/native-effect.json",
        report_path: "native-effect.report.json",
        expected_statuses: &[
            AdaptationStatus::NeedsNativeEffect,
            AdaptationStatus::Dropped,
        ],
        expected_plan: NATIVE_EFFECT_PLAN,
    },
];

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("adapter_kaipai should live under crates/")
        .to_path_buf()
}

fn fixture_root() -> PathBuf {
    project_root().join("fixtures/kaipai")
}

#[test]
fn mapper_fixture_snapshots_catalog_covers_required_families_once() {
    let actual_families = FIXTURE_CASES
        .iter()
        .map(|case| case.family)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        actual_families,
        BTreeSet::from([
            "main-video",
            "pip-overlay",
            "text-sticker",
            "bgm-audio",
            "missing-resource",
            "native-effect",
        ]),
        "fixture catalog should cover every required family exactly once"
    );

    let mut by_polarity = BTreeMap::new();
    for case in FIXTURE_CASES {
        by_polarity
            .entry(case.polarity)
            .or_insert_with(Vec::new)
            .push(case.family);
    }
    assert_eq!(
        by_polarity[&FixturePolarity::Positive],
        ["main-video", "pip-overlay", "text-sticker", "bgm-audio"]
    );
    assert_eq!(
        by_polarity[&FixturePolarity::Negative],
        ["missing-resource", "native-effect"]
    );

    let report_paths = expected_report_paths(&fixture_root().join("expected-reports"));
    let expected_report_paths = FIXTURE_CASES
        .iter()
        .map(|case| case.report_path.to_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        report_paths, expected_report_paths,
        "expected report snapshots must be explicitly cataloged"
    );
}

#[test]
fn mapper_fixture_snapshots_inputs_parse_and_define_import_plan_outcomes() {
    for case in FIXTURE_CASES {
        let value = read_json(&fixture_root().join(case.input_path));
        scan_sanitized_fixture(&value, case.input_path, "$root");
        let bundle = KaipaiFormulaBundle::from_json_value(value.clone())
            .unwrap_or_else(|error| panic!("{} should parse: {error}", case.input_path));

        assert_eq!(bundle.provenance.provider, "kaipai", "{}", case.input_path);
        assert!(!bundle.provenance_refs().is_empty(), "{}", case.input_path);
        assert_expected_canvas(case, &value);
        assert_expected_formula_fields(case, &value);
        assert_expected_plan_outcome(case);
    }
}

#[test]
fn mapper_fixture_snapshots_expected_reports_match_rust_contract() {
    let report_dir = fixture_root().join("expected-reports");
    let mut observed_statuses = BTreeSet::new();

    for case in FIXTURE_CASES {
        let report_path = report_dir.join(case.report_path);
        let actual_json = fs::read_to_string(&report_path)
            .unwrap_or_else(|error| panic!("{} should be readable: {error}", case.report_path));
        let actual_report: AdaptationReport = serde_json::from_str(&actual_json)
            .unwrap_or_else(|error| panic!("{} should deserialize: {error}", case.report_path));
        let expected_report = expected_report(case);
        let expected_json = serde_json::to_string_pretty(&expected_report)
            .expect("expected report serializes")
            + "\n";

        assert_eq!(
            actual_json, expected_json,
            "report snapshot drifted: {}",
            case.report_path
        );
        assert_eq!(
            actual_report.summary, expected_report.summary,
            "{} summary should match expected items",
            case.report_path
        );

        let statuses = actual_report
            .items
            .iter()
            .map(|item| status_name(item.status))
            .collect::<BTreeSet<_>>();
        observed_statuses.extend(statuses.iter().copied());
        for expected_status in case.expected_statuses {
            assert!(
                statuses.contains(status_name(*expected_status)),
                "{} missing expected status {:?}",
                case.report_path,
                expected_status
            );
        }

        for item in actual_report
            .items
            .iter()
            .filter(|item| item.category == AdaptationCategory::NativeEffect)
        {
            assert_ne!(
                item.status,
                AdaptationStatus::Supported,
                "{} must not mark native effects as supported",
                case.report_path
            );
        }
    }

    assert_eq!(
        observed_statuses,
        BTreeSet::from([
            "supported",
            "approximated",
            "dropped",
            "missingResource",
            "needsNativeEffect",
        ]),
        "report snapshots should cover the full Phase 17 status taxonomy"
    );
}

fn status_name(status: AdaptationStatus) -> &'static str {
    match status {
        AdaptationStatus::Supported => "supported",
        AdaptationStatus::Approximated => "approximated",
        AdaptationStatus::Dropped => "dropped",
        AdaptationStatus::MissingResource => "missingResource",
        AdaptationStatus::NeedsNativeEffect => "needsNativeEffect",
    }
}

#[test]
fn mapper_fixture_snapshots_reject_in_memory_secret_and_remote_shapes() {
    let base = read_json(&fixture_root().join("positive/main-video.json"));
    for (case_name, payload, expected_path) in [
        (
            "account id",
            patch(&base, |value| {
                value["formula"]["accountId"] = json!("redacted-account");
            }),
            "formula.accountId",
        ),
        (
            "cookie",
            patch(&base, |value| {
                value["formula"]["Cookie"] = json!("redacted-cookie");
            }),
            "formula.Cookie",
        ),
        (
            "signed URL",
            patch(&base, |value| {
                value["formula"]["assetPath"] =
                    json!("resources/source/main.mp4?signature=redacted&expires=1");
            }),
            "formula.assetPath",
        ),
        (
            "remote runtime URL",
            patch(&base, |value| {
                value["formula"]["remoteRuntimeUrl"] =
                    json!("https://provider.invalid/runtime/render.mp4");
            }),
            "formula.remoteRuntimeUrl",
        ),
    ] {
        let error = KaipaiFormulaBundle::from_json_value(payload)
            .expect_err("unsafe in-memory fixture mutation should fail validation");
        assert!(
            error.to_string().contains(expected_path),
            "{case_name}: unexpected error: {error}"
        );
    }
}

fn expected_report_paths(report_dir: &Path) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    for entry in fs::read_dir(report_dir).expect("expected report directory should be readable") {
        let entry = entry.expect("expected report directory entry should be readable");
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) == Some("json") {
            paths.insert(
                path.file_name()
                    .expect("report path should have file name")
                    .to_string_lossy()
                    .into_owned(),
            );
        }
    }
    paths
}

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(
        &fs::read(path).unwrap_or_else(|error| {
            panic!("fixture should be readable: {}: {error}", path.display())
        }),
    )
    .unwrap_or_else(|error| panic!("fixture should parse as JSON: {}: {error}", path.display()))
}

fn scan_sanitized_fixture(value: &Value, fixture_path: &str, json_path: &str) {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                assert!(
                    !is_credential_like_key(key),
                    "{fixture_path} contains credential-like key at {json_path}.{key}"
                );
                assert!(
                    !is_remote_runtime_key(key),
                    "{fixture_path} contains remote runtime URL key at {json_path}.{key}"
                );
                scan_sanitized_fixture(child, fixture_path, &format!("{json_path}.{key}"));
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                scan_sanitized_fixture(child, fixture_path, &format!("{json_path}[{index}]"));
            }
        }
        Value::String(text) => {
            assert!(
                !looks_like_remote_url(text),
                "{fixture_path} contains remote URL at {json_path}"
            );
            assert!(
                !looks_like_signed_url(text),
                "{fixture_path} contains signed URL at {json_path}"
            );
        }
        _ => {}
    }
}

fn is_credential_like_key(key: &str) -> bool {
    let normalized = normalized_key(key);
    [
        "apikey",
        "authorization",
        "authorizationheader",
        "bearertoken",
        "cookie",
        "password",
        "privatekey",
        "refreshtoken",
        "secret",
        "secretkey",
        "session",
        "sessionjson",
        "token",
        "accesstoken",
        "accountid",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn is_remote_runtime_key(key: &str) -> bool {
    let normalized = normalized_key(key);
    [
        "remoteruntimeurl",
        "remoterenderurl",
        "renderurl",
        "signedurl",
        "cdnurl",
        "downloadurl",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn normalized_key(key: &str) -> String {
    key.chars()
        .filter(|character| *character != '_' && *character != '-')
        .flat_map(char::to_lowercase)
        .collect()
}

fn looks_like_remote_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

fn looks_like_signed_url(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("x-amz-signature")
        || lower.contains("x-oss-signature")
        || lower.contains("signature=")
        || lower.contains("expires=")
}

fn assert_expected_canvas(case: &FixtureCase, value: &Value) {
    let Some(canvas) = case.expected_plan.canvas else {
        return;
    };
    assert_eq!(
        value.pointer("/formula/videoCanvasConfig/width"),
        Some(&json!(canvas.width)),
        "{} should define expected canvas width",
        case.input_path
    );
    assert_eq!(
        value.pointer("/formula/videoCanvasConfig/height"),
        Some(&json!(canvas.height)),
        "{} should define expected canvas height",
        case.input_path
    );
    assert_eq!(
        value.pointer("/formula/videoCanvasConfig/frameRate"),
        Some(&json!(canvas.frame_rate)),
        "{} should define expected frame rate",
        case.input_path
    );
}

fn assert_expected_formula_fields(case: &FixtureCase, value: &Value) {
    match case.family {
        "main-video" => {
            assert_eq!(
                value.pointer("/formula/videoClipList/0/durationMsWithSpeed"),
                Some(&json!(4000))
            );
            assert_eq!(
                value.pointer("/formula/videoClipList/0/crop/left"),
                Some(&json!(80))
            );
            assert_eq!(
                value.pointer("/formula/videoClipList/0/fitMode"),
                Some(&json!("fill"))
            );
        }
        "pip-overlay" => {
            assert_eq!(value.pointer("/formula/pipList/0/level"), Some(&json!(20)));
            assert_eq!(value.pointer("/formula/pipList/0/rotate"), Some(&json!(15)));
            assert_eq!(
                value.pointer("/formula/pipList/0/bounds/widthMillis"),
                Some(&json!(360))
            );
        }
        "text-sticker" => {
            assert_eq!(
                value.pointer("/formula/stickerList/0/textEditInfoList/0/text"),
                Some(&json!("夏日片头"))
            );
            assert_eq!(
                value.pointer("/formula/stickerList/0/textEditInfoList/0/fontPath"),
                Some(&json!("resources/fonts/redacted-noto.otf"))
            );
            assert_eq!(
                value.pointer("/formula/stickerList/0/textEditInfoList/0/showShadow"),
                Some(&json!(true))
            );
        }
        "bgm-audio" => {
            assert_eq!(
                value.pointer("/formula/bgm/volumeMillis"),
                Some(&json!(720))
            );
            assert_eq!(value.pointer("/formula/bgm/fadeInMs"), Some(&json!(500)));
            assert_eq!(value.pointer("/formula/bgm/fadeOutMs"), Some(&json!(800)));
        }
        "missing-resource" => {
            assert_eq!(
                value.pointer("/formula/stickerList/0/resourceId"),
                Some(&json!("missing-sticker-resource"))
            );
        }
        "native-effect" => {
            assert_eq!(
                value.pointer("/formula/nativeEffectList/0/effectType"),
                Some(&json!("beautyRetouch"))
            );
        }
        other => panic!("unexpected fixture family {other}"),
    }
}

fn assert_expected_plan_outcome(case: &FixtureCase) {
    let outcome = case.expected_plan;
    assert!(
        outcome
            .tracks
            .windows(2)
            .all(|pair| pair[0].z_order < pair[1].z_order),
        "{} expected tracks should have deterministic z-order",
        case.family
    );
    for segment in outcome.segments {
        assert!(
            outcome
                .materials
                .iter()
                .any(|material_id| *material_id == segment.material_id),
            "{} expected segment {} references uncataloged material {}",
            case.family,
            segment.id,
            segment.material_id
        );
        assert!(
            segment.source_duration_ms > 0 && segment.target_duration_ms > 0,
            "{} expected segment {} should use positive integer durations",
            case.family,
            segment.id
        );
    }
    assert!(
        outcome
            .tracks
            .iter()
            .all(|track| ["video", "audio", "text", "sticker"].contains(&track.kind)),
        "{} expected tracks should use canonical draft track kinds",
        case.family
    );
    assert!(
        outcome
            .tracks
            .iter()
            .all(|track| !track.id.contains("kaipai") && !track.id.contains("template")),
        "{} expected canonical track IDs should stay provider-neutral",
        case.family
    );
}

fn expected_report(case: &FixtureCase) -> AdaptationReport {
    let items = match case.family {
        "main-video" => vec![
            item(
                AdaptationStatus::Supported,
                AdaptationSeverity::Info,
                AdaptationCategory::Canvas,
                AdaptationTargetKind::Canvas,
                "canvas-main",
                "Canvas, frame rate, and background color map to DraftCanvasConfig.",
                None,
                case.family,
                "formula.videoCanvasConfig",
            ),
            item(
                AdaptationStatus::Supported,
                AdaptationSeverity::Info,
                AdaptationCategory::Segment,
                AdaptationTargetKind::Segment,
                "segment-main-video",
                "Main video source and speed-adjusted target timeranges map to canonical integer microseconds.",
                Some(
                    "durationMsWithSpeed=4000 maps to a 4s target range while preserving the 6s source range.",
                ),
                case.family,
                "formula.videoClipList[0]",
            ),
        ],
        "pip-overlay" => vec![
            item(
                AdaptationStatus::Supported,
                AdaptationSeverity::Info,
                AdaptationCategory::Track,
                AdaptationTargetKind::Track,
                "track-pip-overlay",
                "PIP level maps to provider-neutral overlay z-order.",
                Some(
                    "Kaipai level is catalog evidence only; canonical Draft track ordering carries layer order.",
                ),
                case.family,
                "formula.pipList[0].level",
            ),
            item(
                AdaptationStatus::Supported,
                AdaptationSeverity::Info,
                AdaptationCategory::Segment,
                AdaptationTargetKind::Segment,
                "segment-pip-overlay",
                "PIP bounds, fit, opacity, position, scale, and static center-anchor rotation map to SegmentVisual.",
                Some(
                    "Static center-anchor rotation is supported generically by Plan 17-07 export parity.",
                ),
                case.family,
                "formula.pipList[0]",
            ),
        ],
        "text-sticker" => vec![
            item(
                AdaptationStatus::Supported,
                AdaptationSeverity::Info,
                AdaptationCategory::Text,
                AdaptationTargetKind::Text,
                "segment-text-sticker",
                "Text sticker content, color, stroke, shadow, layout, and wrapping map to TextSegment.",
                None,
                case.family,
                "formula.stickerList[0].textEditInfoList[0]",
            ),
            item(
                AdaptationStatus::Approximated,
                AdaptationSeverity::Warning,
                AdaptationCategory::Font,
                AdaptationTargetKind::Font,
                "font://bundled/noto-sans-cjk-sc-regular",
                "Requested provider font is approximated with bundled Noto Sans CJK SC fallback.",
                Some(
                    "Font closure keeps a local fontRef and records localization fallback in the report.",
                ),
                case.family,
                "formula.stickerList[0].textEditInfoList[0].fontPath",
            ),
            item(
                AdaptationStatus::Dropped,
                AdaptationSeverity::Warning,
                AdaptationCategory::Text,
                AdaptationTargetKind::Effect,
                "text-effect-glow",
                "Provider text glow effect is dropped until a local text effect exists.",
                Some(
                    "Unsupported text effects must not be smuggled into generic filter parameters.",
                ),
                case.family,
                "formula.stickerList[0].textEditInfoList[0].textEffect",
            ),
        ],
        "bgm-audio" => vec![item(
            AdaptationStatus::Supported,
            AdaptationSeverity::Info,
            AdaptationCategory::Audio,
            AdaptationTargetKind::Audio,
            "segment-bgm-audio",
            "BGM material, gain, fade-in, and fade-out map to canonical SegmentAudio.",
            None,
            case.family,
            "formula.bgm",
        )],
        "missing-resource" => vec![
            item(
                AdaptationStatus::MissingResource,
                AdaptationSeverity::Error,
                AdaptationCategory::Resource,
                AdaptationTargetKind::Resource,
                "missing-sticker-resource",
                "Referenced sticker resource is absent from the sanitized offline bundle.",
                Some("Mapper must report the missing resource and skip the dependent segment."),
                case.family,
                "formula.stickerList[0].resourceId",
            ),
            item(
                AdaptationStatus::Dropped,
                AdaptationSeverity::Warning,
                AdaptationCategory::Sticker,
                AdaptationTargetKind::Sticker,
                "segment-missing-sticker",
                "Sticker segment is dropped because its material cannot be localized.",
                None,
                case.family,
                "formula.stickerList[0]",
            ),
        ],
        "native-effect" => vec![
            item(
                AdaptationStatus::NeedsNativeEffect,
                AdaptationSeverity::Warning,
                AdaptationCategory::NativeEffect,
                AdaptationTargetKind::Effect,
                "native-effect-beauty-retouch",
                "Provider-native beauty effect requires a local implementation before it can be represented.",
                Some("Native effects are never classified as supported by fixture expectations."),
                case.family,
                "formula.nativeEffectList[0]",
            ),
            item(
                AdaptationStatus::Dropped,
                AdaptationSeverity::Warning,
                AdaptationCategory::Segment,
                AdaptationTargetKind::Filter,
                "filter-native-effect-beauty-retouch",
                "Native effect is omitted from the canonical draft filter stack.",
                Some(
                    "Report evidence preserves the external reference without writing native parameters to Draft.",
                ),
                case.family,
                "formula.nativeEffectList[0].nativeEffectName",
            ),
        ],
        other => panic!("unexpected report family {other}"),
    };
    AdaptationReport::new("kaipaiOfflineBundle", "2026-06-24T00:00:00Z", items)
}

#[allow(clippy::too_many_arguments)]
fn item(
    status: AdaptationStatus,
    severity: AdaptationSeverity,
    category: AdaptationCategory,
    target_kind: AdaptationTargetKind,
    target_id: &str,
    message: &str,
    details: Option<&str>,
    external_id: &str,
    external_path: &str,
) -> AdaptationReportItem {
    AdaptationReportItem {
        status,
        severity,
        category,
        target: Some(AdaptationTargetRef {
            kind: target_kind,
            id: Some(target_id.to_owned()),
        }),
        message: message.to_owned(),
        details: details.map(str::to_owned),
        provenance: vec![ExternalProvenanceRef {
            source_kind: "kaipaiOfflineBundle".to_owned(),
            external_id: Some(format!("redacted-{external_id}")),
            external_path: Some(external_path.to_owned()),
            note: Some("adapter evidence only; not canonical render semantics".to_owned()),
        }],
    }
}

fn patch(base: &Value, update: impl FnOnce(&mut Value)) -> Value {
    let mut value = base.clone();
    update(&mut value);
    value
}
