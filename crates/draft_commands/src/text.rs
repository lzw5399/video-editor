//! Semantic text/subtitle timeline commands.

use draft_model::{
    CommandDelta, CommandEvent, CommandName, CommandState, Draft, ImportSubtitleSrtCommandPayload,
    ImportSubtitleSrtIntentCommandPayload, Material, MaterialId, MaterialKind, Microseconds,
    Segment, SegmentId, SourceTimerange, TargetTimerange, TextBox, TextLayoutRegion, TextSegment,
    TextSegmentSource, TextStyle, TextWrapping, TimelineCommandResponse, TimelineSelection, Track,
    TrackId, TrackKind,
};

use crate::{
    TimelineCommandError, TimelineCommandErrorKind,
    delta::{text_segment_delta, text_segments_delta},
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

    ensure_text_material(
        &mut next_draft,
        &material_id,
        text_material_display_name(&text.content),
    );

    let mut segment = Segment::new(
        segment_id.clone(),
        material_id,
        source_timerange,
        target_timerange,
    );
    segment.text = Some(text);
    next_draft.tracks[track_index].segments.push(segment);

    validate_timeline_rules(&next_draft)?;
    let delta = text_segment_delta(
        CommandName::AddTextSegment,
        &track_id,
        next_draft.tracks[track_index]
            .segments
            .last()
            .expect("text segment was just appended"),
        "text segment added",
    );

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
        CommandName::AddTextSegment,
        delta,
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
    let delta = text_segment_delta(
        CommandName::EditTextSegment,
        &track_id,
        &next_draft.tracks[track_index].segments[segment_index],
        "text segment edited",
    );

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
        CommandName::EditTextSegment,
        delta,
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
    let mut added_segments = Vec::with_capacity(cues.len());
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

        ensure_text_material(
            &mut next_draft,
            &material_id,
            text_material_display_name(&cue.content),
        );

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
        added_segments.push(segment.clone());
        next_draft.tracks[track_index].segments.push(segment);
        segment_ids.push(segment_id);
    }

    validate_timeline_rules(&next_draft)?;
    let delta = text_segments_delta(
        CommandName::ImportSubtitleSrt,
        &payload.track_id,
        added_segments.iter(),
        "subtitle SRT imported",
    );

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
        CommandName::ImportSubtitleSrt,
        delta,
    ))
}

pub fn import_subtitle_srt_intent(
    payload: ImportSubtitleSrtIntentCommandPayload,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let cues = parse_srt(&payload.srt_content)?;
    let mut next_draft = payload.draft.clone();
    let (track_id, track_index) =
        resolve_subtitle_intent_track(&mut next_draft, &payload.selection)?;

    let mut segment_ids = Vec::with_capacity(cues.len());
    let mut added_segments = Vec::with_capacity(cues.len());
    for cue in cues {
        let segment_id = next_segment_id(&next_draft, "subtitle-segment");
        let material_id = next_material_id(&next_draft, "subtitle-material");
        let target_start = cue
            .start
            .get()
            .checked_add(payload.time_offset.get())
            .ok_or_else(|| {
                TimelineCommandError::new(TimelineCommandErrorKind::TimerangeOverflow {
                    field: "subtitle targetTimerange.start".to_owned(),
                })
            })?;

        let segment = subtitle_segment_from_cue(
            cue,
            segment_id.clone(),
            material_id.clone(),
            target_start,
            payload.style.clone(),
            payload.text_box.clone(),
            payload.layout_region.clone(),
            payload.wrapping,
        );
        ensure_text_material(
            &mut next_draft,
            &material_id,
            text_material_display_name(
                segment
                    .text
                    .as_ref()
                    .expect("subtitle segment has text")
                    .content
                    .as_str(),
            ),
        );

        added_segments.push(segment.clone());
        next_draft.tracks[track_index].segments.push(segment);
        segment_ids.push(segment_id);
    }

    validate_timeline_rules(&next_draft)?;
    let delta = text_segments_delta(
        CommandName::ImportSubtitleSrtIntent,
        &track_id,
        added_segments.iter(),
        "subtitle SRT imported from intent",
    );

    Ok(response(
        next_draft,
        &payload.command_state,
        &payload.draft,
        &payload.selection,
        TimelineSelection {
            segment_ids,
            track_ids: vec![track_id],
        },
        "importSubtitleSrtIntent",
        "subtitleSrtImported",
        CommandName::ImportSubtitleSrtIntent,
        delta,
    ))
}

fn resolve_subtitle_intent_track(
    draft: &mut Draft,
    selection: &TimelineSelection,
) -> Result<(TrackId, usize), TimelineCommandError> {
    if let Some(index) = selection
        .track_ids
        .iter()
        .filter_map(|track_id| {
            draft
                .tracks
                .iter()
                .position(|track| &track.track_id == track_id)
        })
        .find(|index| is_subtitle_track(&draft.tracks[*index]))
    {
        validate_track_unlocked(&draft.tracks[index])?;
        return Ok((draft.tracks[index].track_id.clone(), index));
    }

    if let Some(index) = draft
        .tracks
        .iter()
        .position(|track| is_subtitle_track(track) && !track.locked)
    {
        return Ok((draft.tracks[index].track_id.clone(), index));
    }

    let track_id = next_track_id(draft, "track-subtitle");
    draft
        .tracks
        .push(Track::new(track_id.clone(), TrackKind::Text, "字幕"));
    Ok((track_id, draft.tracks.len() - 1))
}

fn is_subtitle_track(track: &Track) -> bool {
    track.kind == TrackKind::Text
        && (track.track_id.as_str().starts_with("track-subtitle")
            || track.name.contains("字幕")
            || track.name.to_ascii_lowercase().contains("subtitle"))
}

fn subtitle_segment_from_cue(
    cue: SrtCue,
    segment_id: SegmentId,
    material_id: MaterialId,
    target_start: u64,
    style: TextStyle,
    text_box: TextBox,
    layout_region: TextLayoutRegion,
    wrapping: TextWrapping,
) -> Segment {
    let mut segment = Segment::new(
        segment_id,
        material_id,
        SourceTimerange::new(0, cue.duration),
        TargetTimerange::new(target_start, cue.duration),
    );
    segment.text = Some(TextSegment {
        content: cue.content,
        source: TextSegmentSource::Subtitle,
        style,
        text_box,
        layout_region,
        wrapping,
        bubble: None,
        effect: None,
    });
    segment
}

fn ensure_text_material(draft: &mut Draft, material_id: &MaterialId, display_name: String) {
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
        display_name,
    ));
}

fn text_material_display_name(content: &str) -> String {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        "默认文字".to_owned()
    } else {
        trimmed.chars().take(32).collect()
    }
}

fn next_segment_id(draft: &Draft, prefix: &str) -> SegmentId {
    let mut ordinal = draft
        .tracks
        .iter()
        .map(|track| track.segments.len())
        .sum::<usize>()
        .saturating_add(1);
    loop {
        let candidate = SegmentId::from(format!("{prefix}-{ordinal}"));
        if !draft.tracks.iter().any(|track| {
            track
                .segments
                .iter()
                .any(|segment| segment.segment_id == candidate)
        }) {
            return candidate;
        }
        ordinal = ordinal.saturating_add(1);
    }
}

fn next_material_id(draft: &Draft, prefix: &str) -> MaterialId {
    let mut ordinal = draft.materials.len().saturating_add(1);
    loop {
        let candidate = MaterialId::from(format!("{prefix}-{ordinal}"));
        if !draft
            .materials
            .iter()
            .any(|material| material.material_id == candidate)
        {
            return candidate;
        }
        ordinal = ordinal.saturating_add(1);
    }
}

fn next_track_id(draft: &Draft, prefix: &str) -> TrackId {
    let mut ordinal = draft.tracks.len().saturating_add(1);
    loop {
        let candidate = TrackId::from(format!("{prefix}-{ordinal}"));
        if !draft.tracks.iter().any(|track| track.track_id == candidate) {
            return candidate;
        }
        ordinal = ordinal.saturating_add(1);
    }
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
    let mut blocks = Vec::new();
    let mut current = Vec::new();

    for line in normalized.lines().map(str::trim_end) {
        if line.trim().is_empty() {
            if !current.is_empty() {
                blocks.push(std::mem::take(&mut current));
            }
        } else {
            current.push(line);
        }
    }
    if !current.is_empty() {
        blocks.push(current);
    }

    for lines in blocks {
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
    _command: CommandName,
    delta: CommandDelta,
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
        delta,
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
