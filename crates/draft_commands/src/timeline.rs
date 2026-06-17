//! Timeline command validation helpers.

use draft_model::{
    Draft, Material, MaterialId, MaterialKind, Microseconds, SourceTimerange, TargetTimerange,
    Track, TrackId, TrackKind, validate_draft,
};

use crate::{TimelineCommandError, TimelineCommandErrorKind};

pub fn checked_source_end(
    timerange: &SourceTimerange,
) -> Result<Microseconds, TimelineCommandError> {
    checked_timerange_end(
        "sourceTimerange",
        "sourceTimerange.duration",
        timerange.start,
        timerange.duration,
    )
}

pub fn checked_target_end(
    timerange: &TargetTimerange,
) -> Result<Microseconds, TimelineCommandError> {
    checked_timerange_end(
        "targetTimerange",
        "targetTimerange.duration",
        timerange.start,
        timerange.duration,
    )
}

pub fn target_ranges_overlap(
    first: &TargetTimerange,
    second: &TargetTimerange,
) -> Result<bool, TimelineCommandError> {
    let first_end = checked_target_end(first)?;
    let second_end = checked_target_end(second)?;
    Ok(first.start.get() < second_end.get() && second.start.get() < first_end.get())
}

pub fn validate_timeline_rules(draft: &Draft) -> Result<(), TimelineCommandError> {
    validate_timeranges(draft)?;
    validate_track_material_rules(draft)?;
    validate_segment_material_bounds(draft)?;
    validate_track_overlaps(draft)?;
    validate_draft(draft)?;
    Ok(())
}

pub fn validate_segment_material_bounds(draft: &Draft) -> Result<(), TimelineCommandError> {
    for track in &draft.tracks {
        for segment in &track.segments {
            let material = find_material(draft, &segment.material_id)?;
            if let Some(material_duration) = material.metadata.duration {
                let source_end = checked_source_end(&segment.source_timerange)?;
                if source_end.get() > material_duration.get() {
                    return Err(TimelineCommandError::new(
                        TimelineCommandErrorKind::SourceRangeExceedsMaterialDuration {
                            segment_id: segment.segment_id.clone(),
                            material_id: material.material_id.clone(),
                            source_end,
                            material_duration,
                        },
                    ));
                }
            }
        }
    }

    Ok(())
}

pub fn validate_track_material_compatibility(
    track: &Track,
    material: &Material,
) -> Result<(), TimelineCommandError> {
    if track_accepts_material(track.kind, material.kind) {
        return Ok(());
    }

    Err(TimelineCommandError::new(
        TimelineCommandErrorKind::IncompatibleTrackMaterialKind {
            track_id: track.track_id.clone(),
            track_kind: track.kind,
            material_id: material.material_id.clone(),
            material_kind: material.kind,
        },
    ))
}

pub fn validate_track_unlocked(track: &Track) -> Result<(), TimelineCommandError> {
    if track.locked {
        return Err(TimelineCommandError::new(
            TimelineCommandErrorKind::LockedTrack {
                track_id: track.track_id.clone(),
            },
        ));
    }

    Ok(())
}

pub fn visual_track_stack_order(draft: &Draft) -> Vec<TrackId> {
    draft
        .tracks
        .iter()
        .filter(|track| is_visual_track(track.kind))
        .map(|track| track.track_id.clone())
        .collect()
}

pub fn audio_track_mix_order(draft: &Draft) -> Vec<TrackId> {
    draft
        .tracks
        .iter()
        .filter(|track| track.kind == TrackKind::Audio)
        .map(|track| track.track_id.clone())
        .collect()
}

pub fn main_video_track_id(draft: &Draft) -> Option<TrackId> {
    draft
        .tracks
        .iter()
        .find(|track| track.kind == TrackKind::Video)
        .map(|track| track.track_id.clone())
}

fn checked_timerange_end(
    field: &str,
    duration_field: &str,
    start: Microseconds,
    duration: Microseconds,
) -> Result<Microseconds, TimelineCommandError> {
    if duration.get() == 0 {
        return Err(TimelineCommandError::new(
            TimelineCommandErrorKind::ZeroDuration {
                field: duration_field.to_owned(),
            },
        ));
    }

    start
        .get()
        .checked_add(duration.get())
        .map(Microseconds::new)
        .ok_or_else(|| {
            TimelineCommandError::new(TimelineCommandErrorKind::TimerangeOverflow {
                field: field.to_owned(),
            })
        })
}

fn validate_timeranges(draft: &Draft) -> Result<(), TimelineCommandError> {
    for track in &draft.tracks {
        for segment in &track.segments {
            checked_source_end(&segment.source_timerange)?;
            checked_target_end(&segment.target_timerange)?;
        }
    }

    Ok(())
}

fn validate_track_material_rules(draft: &Draft) -> Result<(), TimelineCommandError> {
    for track in &draft.tracks {
        for segment in &track.segments {
            let material = find_material(draft, &segment.material_id)?;
            validate_track_material_compatibility(track, material)?;
        }
    }

    Ok(())
}

fn validate_track_overlaps(draft: &Draft) -> Result<(), TimelineCommandError> {
    for track in &draft.tracks {
        for first_index in 0..track.segments.len() {
            for second_index in (first_index + 1)..track.segments.len() {
                let first = &track.segments[first_index];
                let second = &track.segments[second_index];
                if target_ranges_overlap(&first.target_timerange, &second.target_timerange)? {
                    return Err(TimelineCommandError::new(
                        TimelineCommandErrorKind::OverlappingSegment {
                            track_id: track.track_id.clone(),
                            first_segment_id: first.segment_id.clone(),
                            second_segment_id: second.segment_id.clone(),
                        },
                    ));
                }
            }
        }
    }

    Ok(())
}

fn find_material<'a>(
    draft: &'a Draft,
    material_id: &MaterialId,
) -> Result<&'a Material, TimelineCommandError> {
    draft
        .materials
        .iter()
        .find(|material| &material.material_id == material_id)
        .ok_or_else(|| {
            TimelineCommandError::new(TimelineCommandErrorKind::MaterialNotFound {
                material_id: material_id.clone(),
            })
        })
}

fn track_accepts_material(track_kind: TrackKind, material_kind: MaterialKind) -> bool {
    match track_kind {
        TrackKind::Video => matches!(material_kind, MaterialKind::Video | MaterialKind::Image),
        TrackKind::Audio => material_kind == MaterialKind::Audio,
        TrackKind::Text => material_kind == MaterialKind::Text,
        TrackKind::Sticker => material_kind == MaterialKind::Sticker,
        TrackKind::Filter => matches!(material_kind, MaterialKind::Video | MaterialKind::Image),
    }
}

fn is_visual_track(kind: TrackKind) -> bool {
    matches!(
        kind,
        TrackKind::Video | TrackKind::Text | TrackKind::Sticker | TrackKind::Filter
    )
}
