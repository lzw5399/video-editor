//! Rust-owned snapping and MainTrackMagnet helpers.

use draft_model::{
    CommandEvent, Draft, Microseconds, SegmentId, SnappingSettings, TargetTimerange, TrackId,
    TrackKind, TrimSegmentDirection,
};

use crate::{TimelineCommandError, TimelineCommandErrorKind, timeline::checked_target_end};

pub const DEFAULT_SNAP_THRESHOLD_US: u64 = 100_000;

pub fn apply_snapping(
    draft: &Draft,
    target_track_id: &TrackId,
    moving_segment_id: &SegmentId,
    desired_start: Microseconds,
    duration: Microseconds,
    settings: &SnappingSettings,
) -> Result<(Microseconds, Option<CommandEvent>), TimelineCommandError> {
    if !settings.enabled {
        return Ok((desired_start, None));
    }

    let threshold = snap_threshold(settings);
    let desired_end = desired_start
        .get()
        .checked_add(duration.get())
        .ok_or_else(|| {
            TimelineCommandError::new(TimelineCommandErrorKind::TimerangeOverflow {
                field: "targetTimerange".to_owned(),
            })
        })?;

    let mut best: Option<(u64, Microseconds)> = None;
    for candidate in main_track_snap_candidates(draft, target_track_id, Some(moving_segment_id))? {
        consider_snap(
            &mut best,
            desired_start.get(),
            candidate.get(),
            threshold,
            || Some(candidate),
        );
        consider_snap(&mut best, desired_end, candidate.get(), threshold, || {
            candidate
                .get()
                .checked_sub(duration.get())
                .map(Microseconds::new)
        });
    }

    let Some((_, snapped_start)) = best else {
        return Ok((desired_start, None));
    };
    if snapped_start == desired_start {
        return Ok((desired_start, None));
    }

    Ok((
        snapped_start,
        Some(event(
            "snapped",
            format!(
                "targetStart {} -> {}",
                desired_start.get(),
                snapped_start.get()
            ),
        )),
    ))
}

pub fn snap_trim_boundary(
    draft: &Draft,
    track_id: &TrackId,
    segment_id: &SegmentId,
    direction: TrimSegmentDirection,
    desired: TargetTimerange,
    settings: &SnappingSettings,
) -> Result<(TargetTimerange, Option<CommandEvent>), TimelineCommandError> {
    if !settings.enabled {
        return Ok((desired, None));
    }

    let threshold = snap_threshold(settings);
    let desired_start = desired.start.get();
    let desired_end = checked_target_end(&desired)?.get();
    let mut best: Option<(u64, Microseconds)> = None;

    for candidate in main_track_snap_candidates(draft, track_id, Some(segment_id))? {
        match direction {
            TrimSegmentDirection::Left => {
                consider_snap(&mut best, desired_start, candidate.get(), threshold, || {
                    Some(candidate)
                });
            }
            TrimSegmentDirection::Right => {
                consider_snap(&mut best, desired_end, candidate.get(), threshold, || {
                    Some(candidate)
                });
            }
        }
    }

    let Some((_, snapped_boundary)) = best else {
        return Ok((desired, None));
    };

    let snapped = match direction {
        TrimSegmentDirection::Left => {
            if snapped_boundary.get() >= desired_end {
                return Ok((desired, None));
            }
            TargetTimerange {
                start: snapped_boundary,
                duration: Microseconds::new(desired_end - snapped_boundary.get()),
            }
        }
        TrimSegmentDirection::Right => {
            if snapped_boundary.get() <= desired_start {
                return Ok((desired, None));
            }
            TargetTimerange {
                start: desired.start,
                duration: Microseconds::new(snapped_boundary.get() - desired_start),
            }
        }
    };

    if snapped == desired {
        return Ok((desired, None));
    }

    Ok((
        snapped,
        Some(event(
            "snapped",
            format!("trimBoundary -> {}", snapped_boundary.get()),
        )),
    ))
}

pub fn apply_main_track_magnet(
    draft: &mut Draft,
    track_id: &TrackId,
) -> Result<Option<CommandEvent>, TimelineCommandError> {
    let Some(main_track_id) = draft
        .tracks
        .iter()
        .find(|track| track.kind == TrackKind::Video)
        .map(|track| track.track_id.clone())
    else {
        return Ok(None);
    };
    if &main_track_id != track_id {
        return Ok(None);
    }

    let Some(track) = draft
        .tracks
        .iter_mut()
        .find(|track| &track.track_id == track_id)
    else {
        return Err(TimelineCommandError::new(
            TimelineCommandErrorKind::TrackNotFound {
                track_id: track_id.clone(),
            },
        ));
    };

    if !track
        .segments
        .iter()
        .any(|segment| segment.main_track_magnet.enabled)
    {
        return Ok(None);
    }

    track.segments.sort_by(|left, right| {
        left.target_timerange
            .start
            .cmp(&right.target_timerange.start)
            .then_with(|| left.segment_id.cmp(&right.segment_id))
    });

    let mut cursor = 0_u64;
    let mut changed = false;
    for segment in &mut track.segments {
        if segment.target_timerange.start.get() != cursor {
            segment.target_timerange.start = Microseconds::new(cursor);
            changed = true;
        }
        cursor = cursor
            .checked_add(segment.target_timerange.duration.get())
            .ok_or_else(|| {
                TimelineCommandError::new(TimelineCommandErrorKind::TimerangeOverflow {
                    field: "targetTimerange".to_owned(),
                })
            })?;
    }

    Ok(changed.then(|| event("mainTrackMagnetApplied", "main track gaps closed")))
}

pub fn main_track_snap_candidates(
    draft: &Draft,
    track_id: &TrackId,
    exclude_segment_id: Option<&SegmentId>,
) -> Result<Vec<Microseconds>, TimelineCommandError> {
    let Some(track) = draft
        .tracks
        .iter()
        .find(|track| &track.track_id == track_id)
    else {
        return Err(TimelineCommandError::new(
            TimelineCommandErrorKind::TrackNotFound {
                track_id: track_id.clone(),
            },
        ));
    };

    let mut candidates = Vec::new();
    for segment in &track.segments {
        if exclude_segment_id == Some(&segment.segment_id) {
            continue;
        }
        candidates.push(segment.target_timerange.start);
        candidates.push(checked_target_end(&segment.target_timerange)?);
    }
    candidates.sort();
    candidates.dedup();
    Ok(candidates)
}

fn snap_threshold(settings: &SnappingSettings) -> u64 {
    if settings.threshold.get() == 0 {
        DEFAULT_SNAP_THRESHOLD_US
    } else {
        settings.threshold.get()
    }
}

fn consider_snap(
    best: &mut Option<(u64, Microseconds)>,
    desired: u64,
    candidate: u64,
    threshold: u64,
    snapped_start: impl FnOnce() -> Option<Microseconds>,
) {
    let distance = desired.abs_diff(candidate);
    if distance > threshold {
        return;
    }
    if best
        .as_ref()
        .is_some_and(|(best_distance, _)| *best_distance <= distance)
    {
        return;
    }
    if let Some(snapped_start) = snapped_start() {
        *best = Some((distance, snapped_start));
    }
}

fn event(kind: &str, message: impl Into<String>) -> CommandEvent {
    CommandEvent {
        kind: kind.to_owned(),
        message: Some(message.into()),
    }
}
