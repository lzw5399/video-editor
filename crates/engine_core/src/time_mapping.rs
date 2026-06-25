//! Engine-owned retime source mapping.

use draft_model::{
    AudioRetimePolicy, Microseconds, RetimeMode, SegmentRetiming, SourceTimerange, SpeedCurvePoint,
    SpeedRatio,
};
use serde::{Deserialize, Serialize};

use crate::{EngineError, EngineErrorKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentTimeMap<'a> {
    source_timerange: &'a SourceTimerange,
    retiming: &'a SegmentRetiming,
}

impl<'a> SegmentTimeMap<'a> {
    pub fn new(source_timerange: &'a SourceTimerange, retiming: &'a SegmentRetiming) -> Self {
        Self {
            source_timerange,
            retiming,
        }
    }

    pub fn source_at_target(
        &self,
        target_offset: Microseconds,
    ) -> Result<Microseconds, EngineError> {
        let source_offset = source_offset_for_mode(&self.retiming.mode, target_offset, false)?;
        let source_position = self
            .source_timerange
            .start
            .get()
            .checked_add(source_offset.get())
            .map(Microseconds::new)
            .ok_or_else(|| {
                EngineError::new(
                    EngineErrorKind::TimerangeOverflow,
                    "retimed source position overflowed u64 microseconds",
                )
            })?;
        let source_end = source_timerange_end(self.source_timerange)?;
        if source_position.get() > source_end.get() {
            return Err(EngineError::new(
                EngineErrorKind::SourceRangeExceedsMaterialDuration,
                format!(
                    "retimed source position {} exceeds segment source end {}",
                    source_position.get(),
                    source_end.get()
                ),
            ));
        }
        Ok(source_position)
    }

    pub fn source_range_for_target_duration(
        &self,
        target_duration: Microseconds,
    ) -> Result<SourceTimerange, EngineError> {
        retimed_source_range(self.source_timerange, target_duration, self.retiming)
    }
}

pub fn source_position_for_retime(
    source_timerange: &SourceTimerange,
    target_offset: Microseconds,
    retiming: &SegmentRetiming,
) -> Result<Microseconds, EngineError> {
    SegmentTimeMap::new(source_timerange, retiming).source_at_target(target_offset)
}

pub fn retimed_source_range(
    source_timerange: &SourceTimerange,
    target_duration: Microseconds,
    retiming: &SegmentRetiming,
) -> Result<SourceTimerange, EngineError> {
    let duration = source_offset_for_mode(&retiming.mode, target_duration, true)?;
    let source_end = source_timerange_end(source_timerange)?;
    let retimed_end = source_timerange
        .start
        .get()
        .checked_add(duration.get())
        .map(Microseconds::new)
        .ok_or_else(|| {
            EngineError::new(
                EngineErrorKind::TimerangeOverflow,
                "retimed source range overflowed u64 microseconds",
            )
        })?;
    if retimed_end.get() > source_end.get() {
        return Err(EngineError::new(
            EngineErrorKind::SourceRangeExceedsMaterialDuration,
            format!(
                "retimed source range ends at {} beyond segment source end {}",
                retimed_end.get(),
                source_end.get()
            ),
        ));
    }
    Ok(SourceTimerange {
        start: source_timerange.start,
        duration,
    })
}

pub fn audio_retime_diagnostic(retiming: &SegmentRetiming) -> Option<AudioRetimeDiagnostic> {
    if retiming.audio_policy != AudioRetimePolicy::PreservePitch
        || is_effectively_1x(&retiming.mode)
    {
        return None;
    }

    Some(AudioRetimeDiagnostic {
        kind: AudioRetimeDiagnosticKind::UnsupportedPitchPreservation,
        message: "preserve-pitch audio retiming is not yet supported for non-1x speed".to_owned(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioRetimeDiagnostic {
    pub kind: AudioRetimeDiagnosticKind,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AudioRetimeDiagnosticKind {
    UnsupportedPitchPreservation,
}

fn source_offset_for_mode(
    mode: &RetimeMode,
    target_offset: Microseconds,
    ceil: bool,
) -> Result<Microseconds, EngineError> {
    match mode {
        RetimeMode::Constant { speed } => checked_ratio_duration(target_offset, speed, ceil),
        RetimeMode::SpeedCurve { points } => {
            validate_speed_curve_points(points, target_offset)?;
            integrate_speed_curve(points, target_offset, ceil)
        }
    }
}

fn validate_speed_curve_points(
    points: &[SpeedCurvePoint],
    target_offset: Microseconds,
) -> Result<(), EngineError> {
    if points.is_empty() {
        return Err(EngineError::new(
            EngineErrorKind::DraftValidationFailed,
            "speed curve retime requires at least one point",
        ));
    }
    if points[0].target_time != Microseconds::ZERO {
        return Err(EngineError::new(
            EngineErrorKind::DraftValidationFailed,
            "speed curve first point must be at target time 0us",
        ));
    }

    let mut previous = None;
    for point in points {
        if point.speed.numerator == 0 || point.speed.denominator == 0 {
            return Err(EngineError::new(
                EngineErrorKind::DraftValidationFailed,
                "speed ratios must use nonzero numerator and denominator",
            ));
        }
        if let Some(previous_time) = previous {
            if point.target_time.get() <= previous_time {
                return Err(EngineError::new(
                    EngineErrorKind::DraftValidationFailed,
                    "speed curve target points must be strictly increasing",
                ));
            }
        }
        previous = Some(point.target_time.get());
    }

    if target_offset.get() < points[0].target_time.get() {
        return Err(EngineError::new(
            EngineErrorKind::TimerangeOverflow,
            "target offset precedes speed curve start",
        ));
    }

    Ok(())
}

fn integrate_speed_curve(
    points: &[SpeedCurvePoint],
    target_offset: Microseconds,
    ceil: bool,
) -> Result<Microseconds, EngineError> {
    let mut source_us = 0_u64;
    for (index, point) in points.iter().enumerate() {
        if target_offset.get() <= point.target_time.get() {
            break;
        }
        let next_target = points
            .get(index + 1)
            .map(|next| next.target_time)
            .unwrap_or(target_offset);
        let span_end = Microseconds::new(next_target.get().min(target_offset.get()));
        if span_end.get() <= point.target_time.get() {
            continue;
        }
        let span = Microseconds::new(span_end.get() - point.target_time.get());
        let mapped = checked_ratio_duration(span, &point.speed, ceil)?;
        source_us = source_us.checked_add(mapped.get()).ok_or_else(|| {
            EngineError::new(
                EngineErrorKind::TimerangeOverflow,
                "speed curve source offset overflowed u64 microseconds",
            )
        })?;
        if target_offset.get() <= next_target.get() {
            break;
        }
    }
    Ok(Microseconds::new(source_us))
}

fn checked_ratio_duration(
    duration: Microseconds,
    speed: &SpeedRatio,
    ceil: bool,
) -> Result<Microseconds, EngineError> {
    if speed.numerator == 0 || speed.denominator == 0 {
        return Err(EngineError::new(
            EngineErrorKind::DraftValidationFailed,
            "speed ratios must use nonzero numerator and denominator",
        ));
    }
    let numerator = u128::from(duration.get()) * u128::from(speed.numerator);
    let denominator = u128::from(speed.denominator);
    let mapped = if ceil {
        numerator
            .checked_add(denominator.saturating_sub(1))
            .ok_or_else(|| {
                EngineError::new(
                    EngineErrorKind::TimerangeOverflow,
                    "retime ratio conversion overflowed u128",
                )
            })?
            / denominator
    } else {
        numerator / denominator
    };
    let mapped = u64::try_from(mapped).map_err(|_| {
        EngineError::new(
            EngineErrorKind::TimerangeOverflow,
            "retime ratio conversion exceeded u64 microseconds",
        )
    })?;
    Ok(Microseconds::new(mapped))
}

fn source_timerange_end(source_timerange: &SourceTimerange) -> Result<Microseconds, EngineError> {
    source_timerange
        .start
        .get()
        .checked_add(source_timerange.duration.get())
        .map(Microseconds::new)
        .ok_or_else(|| {
            EngineError::new(
                EngineErrorKind::TimerangeOverflow,
                "sourceTimerange start + duration overflowed u64 microseconds",
            )
        })
}

fn is_effectively_1x(mode: &RetimeMode) -> bool {
    match mode {
        RetimeMode::Constant { speed } => speed.numerator == speed.denominator,
        RetimeMode::SpeedCurve { points } => points
            .iter()
            .all(|point| point.speed.numerator == point.speed.denominator),
    }
}
