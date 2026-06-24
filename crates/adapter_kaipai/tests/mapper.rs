use std::{collections::BTreeSet, env, fs, path::PathBuf};

use adapter_kaipai::{KaipaiFormulaBundle, KaipaiImportOptions, map_kaipai_bundle_to_import_plan};
use draft_import::{AdaptationStatus, ResourceLocalizationMode, validate_import_plan};
use draft_model::{
    CanvasAspectRatio, CanvasBackground, MaterialKind, Microseconds, SegmentAnchor, SegmentFitMode,
    TrackKind,
};
use serde_json::{Value, json};

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
fn offline_mapper_maps_main_video_to_provider_neutral_import_plan() {
    let mapped = map_fixture("positive/main-video.json", "offline-main-video");

    validate_import_plan(&mapped.plan).expect("mapped main video plan should validate");
    assert_canvas(&mapped.plan, "#101010");
    assert_eq!(mapped.plan.materials.len(), 1);

    let material = &mapped.plan.materials[0].material;
    assert_eq!(material.material_id.as_str(), "material-main-video");
    assert_eq!(material.kind, MaterialKind::Video);
    assert_localized_ref(&material.uri);
    assert_eq!(
        material.metadata.duration,
        Some(Microseconds::new(7_000_000))
    );
    assert_eq!(material.metadata.width, Some(1080));
    assert_eq!(material.metadata.height, Some(1920));
    assert!(material.metadata.has_video);
    assert!(material.metadata.has_audio);

    assert_eq!(mapped.plan.tracks.len(), 1);
    assert_eq!(mapped.plan.tracks[0].z_order, 0);
    let track = &mapped.plan.tracks[0].track;
    assert_eq!(track.track_id.as_str(), "track-main-video");
    assert_eq!(track.kind, TrackKind::Video);
    assert_eq!(track.segments.len(), 1);

    let segment = &track.segments[0];
    assert_eq!(segment.segment_id.as_str(), "segment-main-video");
    assert_eq!(segment.material_id.as_str(), "material-main-video");
    assert_eq!(segment.source_timerange.start, Microseconds::new(1_000_000));
    assert_eq!(
        segment.source_timerange.duration,
        Microseconds::new(6_000_000)
    );
    assert_eq!(segment.target_timerange.start, Microseconds::ZERO);
    assert_eq!(
        segment.target_timerange.duration,
        Microseconds::new(4_000_000)
    );
    assert_eq!(segment.visual.fit_mode, SegmentFitMode::Fill);
    assert_eq!(segment.visual.transform.position.x, 120);
    assert_eq!(segment.visual.transform.position.y, -80);
    assert_eq!(segment.visual.transform.scale.x_millis, 1100);
    assert_eq!(segment.visual.transform.scale.y_millis, 1100);
    assert_eq!(segment.visual.transform.opacity.value_millis, 920);
    assert_eq!(segment.visual.transform.crop.left_millis, 80);
    assert_eq!(segment.visual.transform.crop.bottom_millis, 120);
    assert_eq!(segment.visual.transform.anchor, SegmentAnchor::center());

    assert_statuses(&mapped.report.items, &[AdaptationStatus::Supported]);
    assert_no_provider_runtime_refs(&serde_json::to_value(&mapped.plan).unwrap());
}

#[test]
fn offline_mapper_maps_pip_sticker_text_audio_and_reports_degradations() {
    let pip = map_fixture("positive/pip-overlay.json", "offline-pip-overlay");
    validate_import_plan(&pip.plan).expect("mapped PIP plan should validate");
    let pip_material = pip
        .plan
        .materials
        .iter()
        .find(|material| material.material.material_id.as_str() == "material-pip-overlay")
        .expect("PIP material should exist");
    assert_eq!(pip_material.material.metadata.width, Some(1080));
    assert_eq!(pip_material.material.metadata.height, Some(1920));
    assert_eq!(
        pip.plan
            .tracks
            .iter()
            .map(|track| (track.track.track_id.as_str(), track.z_order))
            .collect::<Vec<_>>(),
        vec![("track-main-video", 0), ("track-pip-overlay", 20)]
    );
    let pip_segment = &pip.plan.tracks[1].track.segments[0];
    assert_eq!(pip_segment.material_id.as_str(), "material-pip-overlay");
    assert_eq!(
        pip_segment.target_timerange.start,
        Microseconds::new(1_200_000)
    );
    assert_eq!(
        pip_segment.target_timerange.duration,
        Microseconds::new(2_600_000)
    );
    assert_eq!(pip_segment.visual.fit_mode, SegmentFitMode::Fit);
    assert_eq!(pip_segment.visual.transform.position.x, 360);
    assert_eq!(pip_segment.visual.transform.position.y, 360);
    assert_eq!(pip_segment.visual.transform.scale.x_millis, 480);
    assert_eq!(pip_segment.visual.transform.scale.y_millis, 480);
    assert_eq!(pip_segment.visual.transform.rotation.degrees, 15);
    assert_eq!(pip_segment.visual.transform.opacity.value_millis, 840);
    assert_eq!(pip_segment.visual.transform.anchor, SegmentAnchor::center());
    assert_statuses(&pip.report.items, &[AdaptationStatus::Supported]);

    let text = map_fixture("positive/text-sticker.json", "offline-text-sticker");
    validate_import_plan(&text.plan).expect("mapped text sticker plan should validate");
    assert_eq!(text.plan.materials[0].material.kind, MaterialKind::Text);
    assert_eq!(text.plan.tracks[0].z_order, 30);
    let text_segment = &text.plan.tracks[0].track.segments[0];
    assert_eq!(text_segment.material_id.as_str(), "material-text-sticker");
    let text_payload = text_segment
        .text
        .as_ref()
        .expect("text sticker should map to TextSegment");
    assert_eq!(text_payload.content, "夏日片头");
    assert_eq!(
        text_payload.style.font.font_ref.as_deref(),
        Some(draft_model::BUNDLED_TEXT_FONT_REF)
    );
    assert_eq!(text_payload.style.font_size, 64);
    assert_eq!(text_payload.style.color, "#FFFFFF");
    assert_eq!(text_payload.style.stroke.as_ref().unwrap().width, 3);
    assert_eq!(text_payload.style.shadow.as_ref().unwrap().blur, 12);
    assert_eq!(text_payload.text_box.width_millis, 700);
    assert_eq!(text_payload.text_box.height_millis, 180);
    assert_eq!(text_payload.layout_region.x_millis, 150);
    assert_eq!(text_payload.layout_region.y_millis, 630);
    assert!(
        text_segment.filters.is_empty() && text_payload.effect.is_none(),
        "provider text effects belong in the report, not canonical draft semantics"
    );
    assert_statuses(
        &text.report.items,
        &[
            AdaptationStatus::Supported,
            AdaptationStatus::Approximated,
            AdaptationStatus::Dropped,
        ],
    );

    let audio = map_fixture("positive/bgm-audio.json", "offline-bgm-audio");
    validate_import_plan(&audio.plan).expect("mapped BGM plan should validate");
    assert_eq!(audio.plan.materials[0].material.kind, MaterialKind::Audio);
    assert_eq!(audio.plan.tracks[0].track.kind, TrackKind::Audio);
    let audio_segment = &audio.plan.tracks[0].track.segments[0];
    assert_eq!(audio_segment.audio.gain_millis, 720);
    assert_eq!(
        audio_segment.audio.fade_in_duration.duration,
        Microseconds::new(500_000)
    );
    assert_eq!(
        audio_segment.audio.fade_out_duration.duration,
        Microseconds::new(800_000)
    );
    assert_statuses(&audio.report.items, &[AdaptationStatus::Supported]);
}

#[test]
fn offline_mapper_reports_missing_resources_and_native_effects_without_hiding_support() {
    let missing = map_fixture("negative/missing-resource.json", "offline-missing-resource");
    validate_import_plan(&missing.plan).expect("missing-resource plan should still validate");
    assert!(missing.plan.materials.is_empty());
    assert!(missing.plan.tracks.is_empty());
    assert_statuses(
        &missing.report.items,
        &[AdaptationStatus::MissingResource, AdaptationStatus::Dropped],
    );

    let native = map_fixture("negative/native-effect.json", "offline-native-effect");
    validate_import_plan(&native.plan).expect("native-effect base plan should validate");
    assert_eq!(native.plan.materials.len(), 1);
    let segment = &native.plan.tracks[0].track.segments[0];
    assert!(
        segment.filters.is_empty(),
        "native effect parameters must not enter canonical filters"
    );
    assert_statuses(
        &native.report.items,
        &[
            AdaptationStatus::NeedsNativeEffect,
            AdaptationStatus::Dropped,
        ],
    );
    assert!(
        native
            .report
            .items
            .iter()
            .filter(|item| item.category == draft_import::AdaptationCategory::NativeEffect)
            .all(|item| item.status != AdaptationStatus::Supported),
        "native effects must never be classified as supported"
    );
    assert_no_provider_runtime_refs(&serde_json::to_value(&native.plan).unwrap());
}

#[test]
fn offline_mapper_maps_simple_transition_and_visual_keyframes_generically() {
    let mut value = read_fixture_value("positive/main-video.json");
    value["formula"]["videoClipList"][0]["transition"] = json!({
        "name": "dissolve",
        "durationMs": 300
    });
    value["formula"]["videoClipList"][0]["keyframes"] = json!([
        {"atMs": 0, "property": "positionX", "value": 120},
        {"atMs": 500, "property": "scaleX", "value": 1.25},
        {"atMs": 1000, "property": "opacity", "value": 0.5}
    ]);
    let bundle = KaipaiFormulaBundle::from_json_value(value).expect("mutated fixture should parse");
    let mapped = map_bundle(bundle, "offline-main-video-animation");

    validate_import_plan(&mapped.plan).expect("animated main video plan should validate");
    let segment = &mapped.plan.tracks[0].track.segments[0];
    let transition = segment
        .transition
        .as_ref()
        .expect("simple dissolve should map to canonical transition");
    assert_eq!(transition.name, "dissolve");
    assert_eq!(transition.duration, Microseconds::new(300_000));

    assert_eq!(segment.keyframes.len(), 3);
    assert_eq!(
        segment
            .keyframes
            .iter()
            .map(|keyframe| (&keyframe.property, &keyframe.value, keyframe.at))
            .collect::<Vec<_>>(),
        vec![
            (
                &draft_model::KeyframeProperty::VisualPositionX,
                &draft_model::KeyframeValue::Int { value: 120 },
                Microseconds::ZERO,
            ),
            (
                &draft_model::KeyframeProperty::VisualScaleX,
                &draft_model::KeyframeValue::Uint { value: 1250 },
                Microseconds::new(500_000),
            ),
            (
                &draft_model::KeyframeProperty::VisualOpacity,
                &draft_model::KeyframeValue::Uint { value: 500 },
                Microseconds::new(1_000_000),
            ),
        ]
    );
}

fn map_fixture(path: &str, import_id: &str) -> adapter_kaipai::KaipaiMappedFixture {
    let bundle = KaipaiFormulaBundle::from_json_str(&read_fixture_string(path))
        .expect("fixture should parse");
    map_bundle(bundle, import_id)
}

fn map_bundle(bundle: KaipaiFormulaBundle, import_id: &str) -> adapter_kaipai::KaipaiMappedFixture {
    let temp = temp_case_dir(import_id);
    seed_resources(&temp.source_root, &bundle);

    let mut options = KaipaiImportOptions::new(
        temp.bundle_path.clone(),
        temp.source_root.clone(),
        import_id.to_owned(),
    );
    options.generated_at = Some("2026-06-24T00:00:00Z".to_owned());
    options.resource_mode = ResourceLocalizationMode::CopyRenderableResources;
    options.verify_resource_sha256 = false;

    map_kaipai_bundle_to_import_plan(&bundle, options)
        .unwrap_or_else(|error| panic!("{import_id} should map: {error}"))
}

fn read_fixture_string(path: &str) -> String {
    fs::read_to_string(fixture_root().join(path)).expect("fixture should be readable")
}

fn read_fixture_value(path: &str) -> Value {
    serde_json::from_str(&read_fixture_string(path)).expect("fixture should parse")
}

fn assert_canvas(plan: &draft_import::DraftImportPlan, expected_background: &str) {
    assert_eq!(plan.canvas_config.width, 1080);
    assert_eq!(plan.canvas_config.height, 1920);
    assert_eq!(plan.canvas_config.frame_rate.numerator, 30);
    assert_eq!(plan.canvas_config.frame_rate.denominator, 1);
    assert_eq!(
        plan.canvas_config.aspect_ratio,
        CanvasAspectRatio::preset(draft_model::CanvasAspectRatioPreset::Ratio9x16)
    );
    assert_eq!(
        plan.canvas_config.background,
        CanvasBackground::SolidColor {
            color: expected_background.to_owned()
        }
    );
}

fn assert_localized_ref(uri: &str) {
    assert!(
        uri.starts_with("resources/template-import/"),
        "material refs should be project-relative localized resources: {uri}"
    );
    assert!(!uri.starts_with("http://") && !uri.starts_with("https://"));
}

fn assert_statuses(items: &[draft_import::AdaptationReportItem], expected: &[AdaptationStatus]) {
    let observed = items
        .iter()
        .map(|item| status_name(item.status))
        .collect::<BTreeSet<_>>();
    let expected = expected
        .iter()
        .copied()
        .map(status_name)
        .collect::<BTreeSet<_>>();
    assert_eq!(observed, expected);
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

fn assert_no_provider_runtime_refs(value: &serde_json::Value) {
    let serialized = serde_json::to_string(value).expect("plan should serialize");
    for forbidden in [
        "kaipai",
        "templateId",
        "recipeId",
        "rawFormula",
        "formula",
        "safeArea",
        "http://",
        "https://",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "canonical import plan leaked provider/runtime field {forbidden}: {serialized}"
        );
    }
}

struct TempCase {
    bundle_path: PathBuf,
    source_root: PathBuf,
}

fn temp_case_dir(name: &str) -> TempCase {
    let root = env::temp_dir().join(format!(
        "video-editor-kaipai-offline-mapper-{name}-{}",
        std::process::id()
    ));
    if root.exists() {
        fs::remove_dir_all(&root).expect("stale temp mapper directory should be removable");
    }
    let bundle_path = root.join("mapped.veproj");
    let source_root = root.join("source");
    fs::create_dir_all(&bundle_path).expect("temp bundle should create");
    fs::create_dir_all(&source_root).expect("temp source should create");
    TempCase {
        bundle_path,
        source_root,
    }
}

fn seed_resources(source_root: &PathBuf, bundle: &KaipaiFormulaBundle) {
    for resource in &bundle.resources {
        if resource.resource_id == "missing-sticker-resource" {
            continue;
        }
        let path = source_root.join(&resource.uri);
        fs::create_dir_all(path.parent().expect("resource path should have parent"))
            .expect("resource directory should create");
        fs::write(
            &path,
            format!("offline mapper fixture {}", resource.resource_id),
        )
        .expect("resource fixture should write");
    }
}
