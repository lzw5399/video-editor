use draft_model::{
    AudioRetimePolicy, MaterialId, Microseconds, RetimeMode, SegmentId, SourceTimerange,
    SpeedCurvePoint, SpeedRatio, TargetTimerange, TrackId,
};
use render_graph::{
    RenderAudioMixDiagnostic, RenderIntentSupport, RenderRetimeIntent, RenderRetimeSourceMapping,
};

use crate::job::format_seconds;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledAudioRetimeFilters {
    pub filters: Vec<String>,
    pub diagnostics: Vec<RenderAudioMixDiagnostic>,
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
