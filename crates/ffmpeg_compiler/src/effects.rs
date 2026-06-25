use draft_model::{
    AudioRetimePolicy, ExternalEffectReference, FilterKind, MaterialId, Microseconds, RetimeMode,
    SegmentBlendMode, SegmentId, SegmentMask, SourceTimerange, SpeedCurvePoint, SpeedRatio,
    TargetTimerange, TrackId, TransitionKind, TransitionReference,
};
use render_graph::{
    RenderAudioMixDiagnostic, RenderBlendIntent, RenderFilterIntent, RenderIntentSupport,
    RenderMaskIntent, RenderRetimeIntent, RenderRetimeSourceMapping, RenderTransitionIntent,
    RenderVisualDiagnostic,
};

use crate::job::format_seconds;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledAudioRetimeFilters {
    pub filters: Vec<String>,
    pub diagnostics: Vec<RenderAudioMixDiagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaskBlendExportTarget {
    VideoLayer,
    TextOverlay,
}

pub fn compile_dissolve_transition_filter(
    transition: &RenderTransitionIntent,
    from_label: &str,
    to_label: &str,
    output_label: &str,
    offset: Microseconds,
) -> Option<String> {
    if transition.capability.export != RenderIntentSupport::Supported {
        return None;
    }
    if !matches!(
        transition.reference,
        TransitionReference::FirstParty {
            transition: TransitionKind::Dissolve
        }
    ) {
        return None;
    }

    Some(format!(
        "[{from_label}][{to_label}]xfade=transition=fade:duration={duration}:offset={offset}[{output_label}]",
        duration = format_seconds(transition.duration),
        offset = format_seconds(offset),
    ))
}

pub fn compile_production_effect_filters(filters: &[RenderFilterIntent]) -> Vec<String> {
    let mut ordered_filters = filters.iter().collect::<Vec<_>>();
    ordered_filters.sort_by_key(|filter| filter.order_index);
    ordered_filters
        .into_iter()
        .filter(|filter| filter.enabled && filter.support == RenderIntentSupport::Supported)
        .filter_map(compile_production_effect_filter)
        .collect()
}

fn compile_production_effect_filter(filter: &RenderFilterIntent) -> Option<String> {
    match &filter.kind {
        FilterKind::GaussianBlur { radius_millis } => {
            let sigma = blur_radius_pixels(*radius_millis);
            if sigma <= 0.0 {
                return None;
            }
            Some(format!("gblur=sigma={sigma:.6}"))
        }
        FilterKind::BasicColorAdjustment {
            brightness_millis,
            contrast_millis,
            saturation_millis,
        } => {
            let brightness = decimal_from_signed_millis(*brightness_millis, -1_000, 1_000);
            let contrast = decimal_from_millis(*contrast_millis, 0, 4_000);
            let saturation = decimal_from_millis(*saturation_millis, 0, 4_000);
            if brightness == 0.0 && contrast == 1.0 && saturation == 1.0 {
                return None;
            }
            Some(format!(
                "eq=brightness={brightness:.6}:contrast={contrast:.6}:saturation={saturation:.6}"
            ))
        }
        FilterKind::OpacityAdjustment { opacity_millis } => {
            let opacity = decimal_from_millis(*opacity_millis, 0, 1_000);
            if opacity >= 1.0 {
                return None;
            }
            Some(format!("format=rgba,colorchannelmixer=aa={opacity:.6}"))
        }
        FilterKind::ExternalReference { .. } => None,
    }
}

pub fn compile_phase19_mask_alpha_filters(
    mask: &RenderMaskIntent,
    width: u32,
    height: u32,
) -> Vec<String> {
    if mask.support != RenderIntentSupport::Supported {
        return Vec::new();
    }

    let expression = match &mask.mask {
        SegmentMask::None | SegmentMask::ExternalReference { .. } => return Vec::new(),
        SegmentMask::Rectangle {
            x_millis,
            y_millis,
            width_millis,
            height_millis,
            feather_millis,
            opacity_millis,
            inverted,
        } => rectangle_mask_alpha_expression(
            MaskRect::from_millis(
                width,
                height,
                *x_millis,
                *y_millis,
                *width_millis,
                *height_millis,
            ),
            *feather_millis,
            *opacity_millis,
            *inverted,
            width,
            height,
        ),
        SegmentMask::Ellipse {
            x_millis,
            y_millis,
            width_millis,
            height_millis,
            feather_millis,
            opacity_millis,
            inverted,
        } => ellipse_mask_alpha_expression(
            MaskRect::from_millis(
                width,
                height,
                *x_millis,
                *y_millis,
                *width_millis,
                *height_millis,
            ),
            *feather_millis,
            *opacity_millis,
            *inverted,
        ),
    };

    vec![
        "format=rgba".to_owned(),
        format!("geq=r='r(X,Y)':g='g(X,Y)':b='b(X,Y)':a='{expression}'"),
    ]
}

pub fn mask_blend_export_diagnostics(
    track_id: &TrackId,
    segment_id: &SegmentId,
    material_id: &MaterialId,
    mask: &RenderMaskIntent,
    blend: &RenderBlendIntent,
    target: MaskBlendExportTarget,
) -> Vec<RenderVisualDiagnostic> {
    let mut diagnostics = Vec::new();

    match (&mask.mask, target) {
        (SegmentMask::None, _) => {}
        (
            SegmentMask::ExternalReference { reference },
            MaskBlendExportTarget::VideoLayer | MaskBlendExportTarget::TextOverlay,
        ) => diagnostics.push(visual_export_diagnostic(
            track_id,
            segment_id,
            material_id,
            "mask",
            RenderIntentSupport::Unsupported,
            format!(
                "external mask reference {} is report-only and cannot become FFmpeg export semantics",
                external_reference_id(reference)
            ),
        )),
        (_, MaskBlendExportTarget::TextOverlay) => diagnostics.push(visual_export_diagnostic(
            track_id,
            segment_id,
            material_id,
            "mask",
            RenderIntentSupport::Unsupported,
            "text overlay mask export is unsupported by the current ASS subtitle compiler path",
        )),
        (_, MaskBlendExportTarget::VideoLayer)
            if mask.support != RenderIntentSupport::Supported =>
        {
            diagnostics.push(visual_export_diagnostic(
                track_id,
                segment_id,
                material_id,
                "mask",
                mask.support,
                mask.reason.clone(),
            ));
        }
        _ => {}
    }

    match (&blend.blend_mode, target) {
        (SegmentBlendMode::Normal, _) => {}
        (
            SegmentBlendMode::ExternalReference { reference },
            MaskBlendExportTarget::VideoLayer | MaskBlendExportTarget::TextOverlay,
        ) => diagnostics.push(visual_export_diagnostic(
            track_id,
            segment_id,
            material_id,
            "blendMode",
            RenderIntentSupport::Unsupported,
            format!(
                "external blend reference {} is report-only and cannot become FFmpeg export semantics",
                external_reference_id(reference)
            ),
        )),
        (_, MaskBlendExportTarget::TextOverlay) => diagnostics.push(visual_export_diagnostic(
            track_id,
            segment_id,
            material_id,
            "blendMode",
            RenderIntentSupport::Unsupported,
            "text overlay blend export is unsupported by the current ASS subtitle compiler path",
        )),
        (SegmentBlendMode::Multiply, MaskBlendExportTarget::VideoLayer)
        | (SegmentBlendMode::Screen, MaskBlendExportTarget::VideoLayer) => {
            diagnostics.push(visual_export_diagnostic(
                track_id,
                segment_id,
                material_id,
                "blendMode",
                RenderIntentSupport::Unsupported,
                format!(
                    "{} blend export is unsupported by the compiler until alpha-correct FFmpeg blend compositing is implemented",
                    blend.blend_mode.display_name()
                ),
            ));
        }
    }

    diagnostics
}

pub fn compile_video_retime_filters(
    retime: &RenderRetimeIntent,
    target_delay: Microseconds,
) -> Vec<String> {
    match &retime.retiming.mode {
        RetimeMode::Constant { speed } => vec![constant_speed_setpts_filter(speed, target_delay)],
        RetimeMode::SpeedCurve { points } => {
            vec![speed_curve_setpts_filter(points, target_delay)]
        }
    }
}

pub fn compile_audio_retime_filters(
    track_id: &TrackId,
    segment_id: &SegmentId,
    material_id: &MaterialId,
    retime: &RenderRetimeIntent,
) -> CompiledAudioRetimeFilters {
    if retime.audio.support == RenderIntentSupport::Unsupported {
        return CompiledAudioRetimeFilters {
            filters: Vec::new(),
            diagnostics: vec![retime_audio_diagnostic(
                track_id,
                segment_id,
                material_id,
                retime.audio.support,
                retime.audio.reason.clone(),
            )],
        };
    }

    if !retime.audio.follow_speed || retime.audio.policy != AudioRetimePolicy::FollowVideoSpeed {
        return CompiledAudioRetimeFilters {
            filters: Vec::new(),
            diagnostics: Vec::new(),
        };
    }

    match &retime.retiming.mode {
        RetimeMode::Constant { speed } if is_unity_speed(speed) => CompiledAudioRetimeFilters {
            filters: Vec::new(),
            diagnostics: Vec::new(),
        },
        RetimeMode::Constant { speed } => CompiledAudioRetimeFilters {
            filters: atempo_chain(speed),
            diagnostics: Vec::new(),
        },
        RetimeMode::SpeedCurve { .. } => CompiledAudioRetimeFilters {
            filters: Vec::new(),
            diagnostics: vec![retime_audio_diagnostic(
                track_id,
                segment_id,
                material_id,
                RenderIntentSupport::Degraded,
                "speed-curve audio retime is typed in the render graph but dynamic atempo export is deferred"
                    .to_owned(),
            )],
        },
    }
}

pub fn retimed_source_timerange_for_output(
    mapping: &RenderRetimeSourceMapping,
    output: &TargetTimerange,
) -> Option<SourceTimerange> {
    let target_start = mapping.target_timerange.start.get();
    let target_end = target_start.checked_add(mapping.target_timerange.duration.get())?;
    let output_start = output.start.get();
    let output_end = output_start.checked_add(output.duration.get())?;
    let active_start = target_start.max(output_start);
    let active_end = target_end.min(output_end);
    if active_start >= active_end {
        return None;
    }
    if active_start == target_start && active_end == target_end {
        return Some(mapping.retimed_source_timerange.clone());
    }

    let active_duration = active_end.checked_sub(active_start)?;
    let target_duration = mapping.target_timerange.duration.get().max(1);
    let source_duration = mapping.retimed_source_timerange.duration.get();
    let source_offset = scale_duration(
        active_start.checked_sub(target_start)?,
        source_duration,
        target_duration,
    );
    let duration = scale_duration(active_duration, source_duration, target_duration);
    let source_start = mapping
        .retimed_source_timerange
        .start
        .get()
        .checked_add(source_offset)?;
    Some(SourceTimerange::new(
        Microseconds::new(source_start),
        Microseconds::new(duration),
    ))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MaskRect {
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
    width: u32,
    height: u32,
}

impl MaskRect {
    fn from_millis(
        output_width: u32,
        output_height: u32,
        x_millis: u32,
        y_millis: u32,
        width_millis: u32,
        height_millis: u32,
    ) -> Self {
        let output_width = output_width.max(1);
        let output_height = output_height.max(1);
        let x0 = millis_to_pixels(output_width, x_millis).min(output_width.saturating_sub(1));
        let y0 = millis_to_pixels(output_height, y_millis).min(output_height.saturating_sub(1));
        let width = millis_to_pixels(output_width, width_millis).max(1);
        let height = millis_to_pixels(output_height, height_millis).max(1);
        let x1 = x0.saturating_add(width).min(output_width);
        let y1 = y0.saturating_add(height).min(output_height);
        Self {
            x0,
            y0,
            x1,
            y1,
            width: x1.saturating_sub(x0).max(1),
            height: y1.saturating_sub(y0).max(1),
        }
    }
}

fn rectangle_mask_alpha_expression(
    rect: MaskRect,
    feather_millis: u32,
    opacity_millis: u32,
    inverted: bool,
    output_width: u32,
    output_height: u32,
) -> String {
    let inside = format!(
        "between(X,{},{})*between(Y,{},{})",
        rect.x0, rect.x1, rect.y0, rect.y1
    );
    let base = if feather_millis == 0 {
        inside
    } else {
        let feather = feather_pixels(output_width, output_height, feather_millis);
        format!(
            "if({inside},min(max(min(min(X-{x0},{x1}-X),min(Y-{y0},{y1}-Y))/{feather:.6},0),1),0)",
            x0 = rect.x0,
            x1 = rect.x1,
            y0 = rect.y0,
            y1 = rect.y1,
        )
    };
    mask_alpha_expression(base, opacity_millis, inverted)
}

fn ellipse_mask_alpha_expression(
    rect: MaskRect,
    feather_millis: u32,
    opacity_millis: u32,
    inverted: bool,
) -> String {
    let cx = f64::from(rect.x0) + f64::from(rect.width) / 2.0;
    let cy = f64::from(rect.y0) + f64::from(rect.height) / 2.0;
    let rx = (f64::from(rect.width) / 2.0).max(0.5);
    let ry = (f64::from(rect.height) / 2.0).max(0.5);
    let distance = format!(
        "sqrt(((X-{cx:.6})*(X-{cx:.6}))/({rx:.6}*{rx:.6})+((Y-{cy:.6})*(Y-{cy:.6}))/({ry:.6}*{ry:.6}))"
    );
    let base = if feather_millis == 0 {
        format!("lte({distance},1)")
    } else {
        let feather = decimal_from_millis(feather_millis, 0, 1_000).max(0.001);
        format!("if(lte({distance},1),min(max((1-{distance})/{feather:.6},0),1),0)")
    };
    mask_alpha_expression(base, opacity_millis, inverted)
}

fn mask_alpha_expression(base_alpha: String, opacity_millis: u32, inverted: bool) -> String {
    let opacity = decimal_from_millis(opacity_millis, 0, 1_000);
    let shaped_alpha = if inverted {
        format!("1-({base_alpha})")
    } else {
        base_alpha
    };
    format!("({shaped_alpha})*alpha(X,Y)*{opacity:.6}")
}

fn millis_to_pixels(size: u32, millis: u32) -> u32 {
    let value = u64::from(size) * u64::from(millis.min(1_000));
    u32::try_from((value + 500) / 1_000).unwrap_or(size)
}

fn feather_pixels(width: u32, height: u32, feather_millis: u32) -> f64 {
    let shortest = width.min(height).max(1);
    f64::from(millis_to_pixels(shortest, feather_millis).max(1))
}

fn visual_export_diagnostic(
    track_id: &TrackId,
    segment_id: &SegmentId,
    material_id: &MaterialId,
    property: &str,
    support: RenderIntentSupport,
    reason: impl Into<String>,
) -> RenderVisualDiagnostic {
    RenderVisualDiagnostic {
        track_id: track_id.clone(),
        segment_id: segment_id.clone(),
        material_id: material_id.clone(),
        property: property.to_owned(),
        support,
        reason: reason.into(),
    }
}

fn external_reference_id(reference: &ExternalEffectReference) -> String {
    format!("{}:{}", reference.provider, reference.effect_id)
}

fn constant_speed_setpts_filter(speed: &SpeedRatio, target_delay: Microseconds) -> String {
    let base = if is_unity_speed(speed) {
        "PTS-STARTPTS".to_owned()
    } else {
        format!("(PTS-STARTPTS)*{}/{}", speed.denominator, speed.numerator)
    };
    if target_delay == Microseconds::ZERO {
        format!("setpts={base}")
    } else {
        format!("setpts={base}+{}/TB", format_seconds(target_delay))
    }
}

fn speed_curve_setpts_filter(points: &[SpeedCurvePoint], target_delay: Microseconds) -> String {
    if points.is_empty() {
        return constant_speed_setpts_filter(&SpeedRatio::one(), target_delay);
    }

    let mut source_cursor = Microseconds::ZERO;
    let mut branches = Vec::new();
    for (index, point) in points.iter().enumerate() {
        let target_start = point.target_time;
        let target_end = points
            .get(index + 1)
            .map(|next| next.target_time)
            .unwrap_or_else(|| Microseconds::new(u64::MAX));
        let source_start = source_cursor;
        if target_end.get() != u64::MAX {
            let span = Microseconds::new(target_end.get().saturating_sub(target_start.get()));
            source_cursor = Microseconds::new(
                source_cursor
                    .get()
                    .saturating_add(apply_ratio_floor(span, &point.speed)),
            );
        }
        let expression = format!(
            "{}+(T-{})*{}/{}+{}",
            format_seconds(target_start),
            format_seconds(source_start),
            point.speed.denominator,
            point.speed.numerator,
            format_seconds(target_delay)
        );
        if target_end.get() == u64::MAX {
            branches.push(expression);
        } else {
            branches.push(format!(
                "if(lt(T,{}),{},",
                format_seconds(source_cursor),
                expression
            ));
        }
    }

    let mut expression = branches.pop().unwrap_or_else(|| "T".to_owned());
    while let Some(prefix) = branches.pop() {
        expression = format!("{prefix}{expression})");
    }
    format!("setpts={expression}/TB")
}

fn atempo_chain(speed: &SpeedRatio) -> Vec<String> {
    let mut value = f64::from(speed.numerator) / f64::from(speed.denominator.max(1));
    if !value.is_finite() || value <= 0.0 {
        return Vec::new();
    }

    let mut filters = Vec::new();
    while value > 2.0 {
        filters.push("atempo=2.000000".to_owned());
        value /= 2.0;
    }
    while value < 0.5 {
        filters.push("atempo=0.500000".to_owned());
        value /= 0.5;
    }
    if (value - 1.0).abs() > f64::EPSILON {
        filters.push(format!("atempo={value:.6}"));
    }
    filters
}

fn retime_audio_diagnostic(
    track_id: &TrackId,
    segment_id: &SegmentId,
    material_id: &MaterialId,
    support: RenderIntentSupport,
    reason: String,
) -> RenderAudioMixDiagnostic {
    RenderAudioMixDiagnostic {
        track_id: track_id.clone(),
        segment_id: segment_id.clone(),
        material_id: material_id.clone(),
        property: "retime.audio".to_owned(),
        support,
        reason,
    }
}

fn is_unity_speed(speed: &SpeedRatio) -> bool {
    speed.numerator == speed.denominator
}

fn scale_duration(value: u64, numerator: u64, denominator: u64) -> u64 {
    let denominator = denominator.max(1);
    ((u128::from(value) * u128::from(numerator)) / u128::from(denominator))
        .min(u128::from(u64::MAX)) as u64
}

fn apply_ratio_floor(value: Microseconds, speed: &SpeedRatio) -> u64 {
    ((u128::from(value.get()) * u128::from(speed.numerator)) / u128::from(speed.denominator.max(1)))
        .min(u128::from(u64::MAX)) as u64
}

fn blur_radius_pixels(radius_millis: u32) -> f64 {
    decimal_from_millis(radius_millis.saturating_mul(8), 0, 16_000)
}

fn decimal_from_millis(value: u32, min: u32, max: u32) -> f64 {
    f64::from(value.clamp(min, max)) / 1_000.0
}

fn decimal_from_signed_millis(value: i32, min: i32, max: i32) -> f64 {
    f64::from(value.clamp(min, max)) / 1_000.0
}
