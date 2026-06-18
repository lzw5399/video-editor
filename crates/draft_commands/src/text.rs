//! Semantic text/subtitle timeline commands.

use draft_model::{
    CommandEvent, CommandState, Draft, ImportSubtitleSrtCommandPayload, Material, MaterialId,
    MaterialKind, Microseconds, Segment, SegmentId, SourceTimerange, TargetTimerange, TextSegment,
    TextSegmentSource, TimelineCommandResponse, TimelineSelection, Track, TrackId, TrackKind,
};

use crate::{
    TimelineCommandError, TimelineCommandErrorKind,
    history::push_undo_snapshot,
    timeline::{validate_timeline_rules, validate_track_unlocked},
};

pub fn add_text_segment(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    track_id: TrackId,
    segment_id: SegmentId,
    material_id: MaterialId,
    source_timerange: SourceTimerange,
    target_timerange: TargetTimerange,
    text: TextSegment,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_draft = draft.clone();
    let track_index = find_track_index(&next_draft, &track_id)?;
    validate_track_unlocked(&next_draft.tracks[track_index])?;

    ensure_text_material(&mut next_draft, &material_id);

    let mut segment = Segment::new(
        segment_id.clone(),
        material_id,
        source_timerange,
        target_timerange,
    );
    segment.text = Some(text);
    next_draft.tracks[track_index].segments.push(segment);

    validate_timeline_rules(&next_draft)?;

    Ok(response(
        next_draft,
        command_state,
        draft,
        selection,
        TimelineSelection {
            segment_ids: vec![segment_id],
            track_ids: vec![track_id],
        },
        "addTextSegment",
        "textSegmentAdded",
    ))
}

pub fn edit_text_segment(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    segment_id: SegmentId,
    text: TextSegment,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_draft = draft.clone();
    let (track_index, segment_index) = find_segment_location(&next_draft, &segment_id)?;
    validate_track_unlocked(&next_draft.tracks[track_index])?;

    if next_draft.tracks[track_index].segments[segment_index]
        .text
        .is_none()
    {
        return Err(TimelineCommandError::new(
            TimelineCommandErrorKind::DraftValidationFailed {
                message: format!("segment {} has no text semantic data", segment_id.as_str()),
            },
        ));
    }

    next_draft.tracks[track_index].segments[segment_index].text = Some(text);
    validate_timeline_rules(&next_draft)?;
    let track_id = next_draft.tracks[track_index].track_id.clone();

    Ok(response(
        next_draft,
        command_state,
        draft,
        selection,
        TimelineSelection {
            segment_ids: vec![segment_id],
            track_ids: vec![track_id],
        },
        "editTextSegment",
        "textSegmentEdited",
    ))
}

pub fn import_subtitle_srt(
    payload: ImportSubtitleSrtCommandPayload,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let cues = parse_srt(&payload.srt_content)?;
    if payload.segment_id_prefix.trim().is_empty() {
        return invalid_srt("segment ID prefix must not be empty");
    }
    if payload.material_id_prefix.trim().is_empty() {
        return invalid_srt("material ID prefix must not be empty");
    }
    if payload.track_name.trim().is_empty() {
        return invalid_srt("track name must not be empty");
    }

    let mut next_draft = payload.draft.clone();
    let track_index = match next_draft
        .tracks
        .iter()
        .position(|track| track.track_id == payload.track_id)
    {
        Some(index) => {
            let track = &next_draft.tracks[index];
            if track.kind != TrackKind::Text {
                return Err(TimelineCommandError::new(
                    TimelineCommandErrorKind::DraftValidationFailed {
                        message: format!(
                            "SRT import target track {} is not a text track",
                            payload.track_id.as_str()
                        ),
                    },
                ));
            }
            validate_track_unlocked(track)?;
            index
        }
        None => {
            next_draft.tracks.push(Track::new(
                payload.track_id.clone(),
                TrackKind::Text,
                payload.track_name.clone(),
            ));
            next_draft.tracks.len() - 1
        }
    };

    let mut segment_ids = Vec::with_capacity(cues.len());
    for (index, cue) in cues.into_iter().enumerate() {
        let ordinal = index + 1;
        let segment_id = SegmentId::from(format!("{}-{ordinal}", payload.segment_id_prefix));
        let material_id = MaterialId::from(format!("{}-{ordinal}", payload.material_id_prefix));
        let target_start = cue
            .start
            .get()
            .checked_add(payload.time_offset.get())
            .ok_or_else(|| {
                TimelineCommandError::new(TimelineCommandErrorKind::TimerangeOverflow {
                    field: "subtitle targetTimerange.start".to_owned(),
                })
            })?;

        ensure_text_material(&mut next_draft, &material_id);

        let mut text = TextSegment {
            content: cue.content,
            source: TextSegmentSource::Subtitle,
            style: payload.style.clone(),
            text_box: payload.text_box.clone(),
            layout_region: payload.layout_region.clone(),
            wrapping: payload.wrapping,
            bubble: None,
            effect: None,
        };
        text.source = TextSegmentSource::Subtitle;

        let mut segment = Segment::new(
            segment_id.clone(),
            material_id,
            SourceTimerange::new(0, cue.duration),
            TargetTimerange::new(target_start, cue.duration),
        );
        segment.text = Some(text);
        next_draft.tracks[track_index].segments.push(segment);
        segment_ids.push(segment_id);
    }

    validate_timeline_rules(&next_draft)?;

    Ok(response(
        next_draft,
        &payload.command_state,
        &payload.draft,
        &payload.selection,
        TimelineSelection {
            segment_ids,
            track_ids: vec![payload.track_id],
        },
        "importSubtitleSrt",
        "subtitleSrtImported",
    ))
}

fn ensure_text_material(draft: &mut Draft, material_id: &MaterialId) {
    if draft
        .materials
        .iter()
        .any(|material| &material.material_id == material_id)
    {
        return;
    }

    draft.materials.push(Material::new(
        material_id.clone(),
        MaterialKind::Text,
        format!("text://{}", material_id.as_str()),
        material_id.as_str(),
    ));
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SrtCue {
    start: Microseconds,
    duration: Microseconds,
    content: String,
}

fn parse_srt(content: &str) -> Result<Vec<SrtCue>, TimelineCommandError> {
    let normalized = content.trim_start_matches('\u{feff}').replace("\r\n", "\n");
    let mut cues = Vec::new();

    for block in normalized.split("\n\n") {
        let lines = block
            .lines()
            .map(str::trim_end)
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<_>>();
        if lines.is_empty() {
            continue;
        }
        if lines.len() < 3 {
            return invalid_srt("each cue must include an index, timing line, and text");
        }
        if lines[0].trim().parse::<u32>().is_err() {
            return invalid_srt("cue index must be numeric");
        }

        let (start, end) = parse_timing_line(lines[1])?;
        if end.get() <= start.get() {
            return invalid_srt("cue end time must be after start time");
        }
        let text = lines[2..]
            .iter()
            .map(|line| line.trim())
            .collect::<Vec<_>>()
            .join("\n");
        if text.trim().is_empty() {
            return invalid_srt("cue text must not be empty");
        }
        cues.push(SrtCue {
            start,
            duration: Microseconds::new(end.get() - start.get()),
            content: text,
        });
    }

    if cues.is_empty() {
        return invalid_srt("SRT content must include at least one cue");
    }

    Ok(cues)
}

fn parse_timing_line(line: &str) -> Result<(Microseconds, Microseconds), TimelineCommandError> {
    let (start, end) = line
        .split_once("-->")
        .ok_or_else(|| invalid_srt_error("cue timing line must contain -->"))?;
    Ok((parse_srt_timestamp(start)?, parse_srt_timestamp(end)?))
}

fn parse_srt_timestamp(value: &str) -> Result<Microseconds, TimelineCommandError> {
    let value = value
        .trim()
        .split_whitespace()
        .next()
        .ok_or_else(|| invalid_srt_error("timestamp must not be empty"))?;
    let (hours, rest) = value
        .split_once(':')
        .ok_or_else(|| invalid_srt_error("timestamp must use HH:MM:SS,mmm"))?;
    let (minutes, rest) = rest
        .split_once(':')
        .ok_or_else(|| invalid_srt_error("timestamp must use HH:MM:SS,mmm"))?;
    let (seconds, millis) = rest
        .split_once(',')
        .or_else(|| rest.split_once('.'))
        .ok_or_else(|| invalid_srt_error("timestamp must include milliseconds"))?;

    let hours = parse_timestamp_part(hours, "hours")?;
    let minutes = parse_timestamp_part(minutes, "minutes")?;
    let seconds = parse_timestamp_part(seconds, "seconds")?;
    let millis = parse_timestamp_part(millis, "milliseconds")?;

    if minutes >= 60 || seconds >= 60 || millis >= 1_000 {
        return invalid_srt("timestamp minutes, seconds, and milliseconds must be in range");
    }

    let total = hours
        .checked_mul(3_600_000_000)
        .and_then(|value| value.checked_add(minutes * 60_000_000))
        .and_then(|value| value.checked_add(seconds * 1_000_000))
        .and_then(|value| value.checked_add(millis * 1_000))
        .ok_or_else(|| invalid_srt_error("timestamp overflows microseconds"))?;
    Ok(Microseconds::new(total))
}

fn parse_timestamp_part(value: &str, field: &str) -> Result<u64, TimelineCommandError> {
    value
        .parse::<u64>()
        .map_err(|_| invalid_srt_error(&format!("timestamp {field} must be numeric")))
}

fn invalid_srt<T>(message: &str) -> Result<T, TimelineCommandError> {
    Err(invalid_srt_error(message))
}

fn invalid_srt_error(message: &str) -> TimelineCommandError {
    TimelineCommandError::new(TimelineCommandErrorKind::DraftValidationFailed {
        message: format!("invalid SRT: {message}"),
    })
}

fn response(
    draft: Draft,
    command_state: &CommandState,
    previous_draft: &Draft,
    previous_selection: &TimelineSelection,
    selection: TimelineSelection,
    history_label: &str,
    event_kind: &str,
) -> TimelineCommandResponse {
    let (command_state, pruned) = push_undo_snapshot(
        command_state,
        previous_draft,
        previous_selection,
        history_label,
    );
    let mut events = vec![CommandEvent {
        kind: event_kind.to_owned(),
        message: None,
    }];
    if pruned {
        events.push(CommandEvent {
            kind: "historyLimitPruned".to_owned(),
            message: None,
        });
    }

    TimelineCommandResponse {
        draft,
        command_state,
        selection,
        events,
    }
}

fn find_track_index(draft: &Draft, track_id: &TrackId) -> Result<usize, TimelineCommandError> {
    draft
        .tracks
        .iter()
        .position(|track| &track.track_id == track_id)
        .ok_or_else(|| {
            TimelineCommandError::new(TimelineCommandErrorKind::TrackNotFound {
                track_id: track_id.clone(),
            })
        })
}

fn find_segment_location(
    draft: &Draft,
    segment_id: &SegmentId,
) -> Result<(usize, usize), TimelineCommandError> {
    draft
        .tracks
        .iter()
        .enumerate()
        .find_map(|(track_index, track)| {
            track
                .segments
                .iter()
                .position(|segment| &segment.segment_id == segment_id)
                .map(|segment_index| (track_index, segment_index))
        })
        .ok_or_else(|| {
            TimelineCommandError::new(TimelineCommandErrorKind::SegmentNotFound {
                segment_id: segment_id.clone(),
            })
        })
}
