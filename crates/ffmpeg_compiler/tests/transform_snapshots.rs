mod common;

use draft_model::{
    ExternalEffectReference, Keyframe, KeyframeEasing, KeyframeInterpolation, KeyframeProperty,
    KeyframeValue, MaterialKind, Microseconds, RationalFrameRate, SegmentAnchor,
    SegmentBackgroundFilling, SegmentBlendMode, SegmentCrop, SegmentFitMode, SegmentMask,
    SegmentOpacity, SegmentPosition, SegmentRotation, SegmentScale, TargetTimerange,
};
use engine_core::{EngineProfile, normalize_draft, resolve_render_range};
use ffmpeg_compiler::{CompileContext, CompilerCapabilities, compile_ffmpeg_job};
use render_graph::{
    ExportMp4Preset, OutputDimensions, RenderGraphPlan, RenderIntentSupport, RenderOutputProfile,
    build_render_graph,
};

#[test]
fn transform_snapshot_compiles_crop_scale_opacity_and_normalized_position() {
    let mut draft = common::compiler_draft();
    let overlay = &mut draft.tracks[1].segments[0];
    overlay.visual.transform.position = SegmentPosition { x: 250, y: -250 };
    overlay.visual.transform.scale = SegmentScale {
        x_millis: 500,
        y_millis: 500,
    };
    overlay.visual.transform.opacity = SegmentOpacity { value_millis: 760 };
    overlay.visual.transform.crop = SegmentCrop {
        left_millis: 100,
        right_millis: 0,
        top_millis: 0,
        bottom_millis: 0,
    };

    let job = compile_ffmpeg_job(&export_plan_from_draft(draft), &compile_context())
        .expect("transform-aware export job should compile");

    assert_eq!(
        job.filter_script,
        [
            "[2:v]trim=start=0.700000:duration=0.100000,setpts=PTS-STARTPTS,scale=1920:1080[vstage0a]",
            "[vstage0a]null[v0]",
            "[1:v]loop=loop=-1:size=1:start=0,fps=30/1,trim=duration=0.100000,setpts=PTS-STARTPTS,crop=576:360:64:0,scale=1728:1080[vstage1a]",
            "[vstage1a]scale=864:540,format=rgba,colorchannelmixer=aa=0.760[v1]",
            "color=c=black:s=1920x1080:r=30/1:d=0.100000[vbase0]",
            "[vbase0][v0]overlay=x=0:y=0:enable='between(t,0.000000,0.100000)'[vbase1]",
            "[vbase1][v1]overlay=x=768:y=405:enable='between(t,0.000000,0.100000)'[vbase2]",
            "[vbase2]subtitles=filename='/derived/draft-compiler-export-mp4-h264-balanced-text-text-a.ass':fontsdir='/fonts'[vtext0]",
            "[vtext0]format=yuv420p[vout]",
            "[0:a]atrim=start=0.600000:duration=0.100000,asetpts=PTS-STARTPTS,volume=1.000[a0]",
            "[2:a]atrim=start=0.700000:duration=0.100000,asetpts=PTS-STARTPTS,volume=1.000[a1]",
            "[a0][a1]amix=inputs=2:duration=longest:normalize=0[aout]",
        ]
        .join(";\n")
    );
}

#[test]
fn transform_snapshot_compiles_static_center_anchor_rotation_and_preserves_layer_order() {
    let mut draft = common::compiler_draft();
    let overlay = &mut draft.tracks[1].segments[0];
    overlay.visual.transform.position = SegmentPosition { x: -250, y: 125 };
    overlay.visual.transform.anchor = SegmentAnchor::center();
    overlay.visual.transform.scale = SegmentScale {
        x_millis: 500,
        y_millis: 500,
    };
    overlay.visual.transform.rotation = SegmentRotation { degrees: 90 };
    overlay.visual.transform.opacity = SegmentOpacity { value_millis: 875 };

    let job = compile_ffmpeg_job(&export_plan_from_draft(draft), &compile_context())
        .expect("static center-anchor rotation export job should compile");

    assert!(
        job.visual_diagnostics.iter().all(|diagnostic| {
            diagnostic.property != "rotation"
                || diagnostic.support != RenderIntentSupport::Unsupported
        }),
        "static rotation must not be classified as unsupported: {:?}",
        job.visual_diagnostics
    );
    assert!(
        job.filter_script.contains("rotate="),
        "static rotation should be represented in the generic export filtergraph:\n{}",
        job.filter_script
    );
    assert!(
        job.filter_script.contains("ow=rotw") && job.filter_script.contains("oh=roth"),
        "center-anchor rotation should expand the rotated layer bounds before placement:\n{}",
        job.filter_script
    );

    let base_overlay = job
        .filter_script
        .find("[vbase0][v0]overlay")
        .expect("base video should be composed first");
    let rotated_overlay = job
        .filter_script
        .find("[vbase1][v1]overlay")
        .expect("rotated overlay should be composed after the base video");
    assert!(
        base_overlay < rotated_overlay,
        "visual layers should preserve graph stack order after rotation:\n{}",
        job.filter_script
    );
}

#[test]
fn transform_snapshot_compiles_full_canvas_rotation_outside_identity_fast_path() {
    let mut draft = common::compiler_draft();
    let base_video = &mut draft.tracks[0].segments[0];
    base_video.visual.fit_mode = SegmentFitMode::Stretch;
    base_video.visual.transform.position = SegmentPosition { x: 0, y: 0 };
    base_video.visual.transform.anchor = SegmentAnchor::center();
    base_video.visual.transform.scale = SegmentScale {
        x_millis: 1_000,
        y_millis: 1_000,
    };
    base_video.visual.transform.opacity = SegmentOpacity {
        value_millis: 1_000,
    };
    base_video.visual.transform.rotation = SegmentRotation { degrees: 90 };

    let job = compile_ffmpeg_job(&export_plan_from_draft(draft), &compile_context())
        .expect("full-canvas static rotation export job should compile");

    assert!(
        job.filter_script.contains("rotate="),
        "full-canvas rotation must not be hidden by the identity fast path:\n{}",
        job.filter_script
    );
    assert!(
        job.filter_script.contains("ow=rotw") && job.filter_script.contains("oh=roth"),
        "full-canvas rotation should expand bounds before placement:\n{}",
        job.filter_script
    );
}

#[test]
fn transform_snapshot_reports_non_center_anchor_rotation_as_unsupported() {
    let mut draft = common::compiler_draft();
    let overlay = &mut draft.tracks[1].segments[0];
    overlay.visual.transform.anchor = SegmentAnchor {
        x_millis: 0,
        y_millis: 0,
    };
    overlay.visual.transform.rotation = SegmentRotation { degrees: 30 };

    let job = compile_ffmpeg_job(&export_plan_from_draft(draft), &compile_context())
        .expect("non-center-anchor rotation should compile with an explicit diagnostic");

    assert!(job.visual_diagnostics.iter().any(|diagnostic| {
        diagnostic.property == "rotationAnchor"
            && diagnostic.support == RenderIntentSupport::Unsupported
    }));
}

#[test]
fn transform_fit_mode_background_fill_and_visual_diagnostics_are_preserved() {
    let mut draft = common::compiler_draft();
    let overlay_material = draft
        .materials
        .iter_mut()
        .find(|material| material.material_id.as_str() == "overlay-material")
        .expect("overlay material fixture exists");
    overlay_material.kind = MaterialKind::Video;
    overlay_material.metadata.width = Some(720);
    overlay_material.metadata.height = Some(1_280);

    let overlay = &mut draft.tracks[1].segments[0];
    overlay.visual.fit_mode = SegmentFitMode::Fit;
    overlay.visual.background_filling = SegmentBackgroundFilling::SolidColor {
        color: "#224466".to_owned(),
    };
    overlay.visual.transform.rotation = SegmentRotation { degrees: 15 };
    overlay.visual.blend_mode = SegmentBlendMode::ExternalReference {
        reference: ExternalEffectReference::new("fixture", "screen"),
    };
    overlay.visual.mask = SegmentMask::ExternalReference {
        reference: ExternalEffectReference::new("fixture", "linear"),
    };

    let job = compile_ffmpeg_job(&export_plan_from_draft(draft), &compile_context())
        .expect("fit-mode export job should compile with diagnostics");

    assert!(job.filter_script.contains("scale=608:1080"));
    assert!(
        job.filter_script
            .contains("color=c=0x224466:s=1920x1080:r=30/1:d=0.100000[vsegbg1]")
    );
    assert!(
        job.filter_script
            .contains("[vsegbg1][vstage1a]overlay=x=656:y=0:shortest=1[vstage1b]")
    );
    assert!(job.filter_script.contains("rotate="));
    assert!(
        job.visual_diagnostics.iter().all(|diagnostic| {
            diagnostic.property != "rotation"
                || diagnostic.support != RenderIntentSupport::Unsupported
        }),
        "static rotation should compile instead of being reported unsupported: {:?}",
        job.visual_diagnostics
    );
    assert!(job.visual_diagnostics.iter().any(|diagnostic| {
        diagnostic.property == "blendMode" && diagnostic.support == RenderIntentSupport::Unsupported
    }));
    assert!(job.visual_diagnostics.iter().any(|diagnostic| {
        diagnostic.property == "mask" && diagnostic.support == RenderIntentSupport::Unsupported
    }));
    assert!(!job.filter_script.contains("blend=all_mode"));
}

#[test]
fn transform_keyframe_animation_diagnostics_are_preserved_without_ffmpeg_animation_expressions() {
    let mut draft = common::compiler_draft();
    let overlay = &mut draft.tracks[1].segments[0];
    overlay.keyframes.extend([
        int_keyframe(KeyframeProperty::VisualRotation, 600_000, 0),
        int_keyframe(KeyframeProperty::VisualRotation, 666_666, 30),
        uint_keyframe(KeyframeProperty::VisualOpacity, 600_000, 1_000),
        uint_keyframe(KeyframeProperty::VisualOpacity, 666_666, 500),
    ]);

    let job = compile_ffmpeg_job(&export_plan_from_draft(draft), &compile_context())
        .expect("animation diagnostic export job should compile");

    assert!(job.visual_diagnostics.iter().any(|diagnostic| {
        diagnostic.property == "keyframe.visualRotation"
            && diagnostic.support == RenderIntentSupport::Unsupported
    }));
    assert!(job.visual_diagnostics.iter().any(|diagnostic| {
        diagnostic.property == "keyframe.visualOpacity"
            && diagnostic.support == RenderIntentSupport::Degraded
    }));
    assert!(!job.filter_script.contains("rotate="));
    assert!(job.filter_script.contains("enable='between"));
    assert!(!job.filter_script.contains("if("));
}

fn export_plan_from_draft(draft: draft_model::Draft) -> RenderGraphPlan {
    let normalized =
        normalize_draft(&draft, &EngineProfile::mvp_default()).expect("draft should normalize");
    let range = resolve_render_range(
        &normalized,
        TargetTimerange::new(Microseconds::new(600_000), Microseconds::new(100_000)),
    )
    .expect("range state should resolve");
    let graph = build_render_graph(&normalized, &range).expect("graph should build");

    RenderGraphPlan::new(
        graph,
        RenderOutputProfile::export_mp4(
            OutputDimensions::new(1_920, 1_080),
            RationalFrameRate::new(30, 1),
            TargetTimerange::new(Microseconds::new(600_000), Microseconds::new(100_000)),
            ExportMp4Preset::h264_aac_balanced(),
        ),
    )
    .expect("export profile should validate")
}

fn compile_context() -> CompileContext {
    CompileContext::new("/derived/output.mp4", "/derived")
        .with_capabilities(CompilerCapabilities::all_available_for_tests())
}

fn int_keyframe(property: KeyframeProperty, at: u64, value: i32) -> Keyframe {
    Keyframe {
        at: Microseconds::new(at),
        property,
        value: KeyframeValue::Int { value },
        interpolation: KeyframeInterpolation::Linear,
        easing: KeyframeEasing::None,
    }
}

fn uint_keyframe(property: KeyframeProperty, at: u64, value: u32) -> Keyframe {
    Keyframe {
        at: Microseconds::new(at),
        property,
        value: KeyframeValue::Uint { value },
        interpolation: KeyframeInterpolation::Linear,
        easing: KeyframeEasing::None,
    }
}
