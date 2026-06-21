//! Semantic audio/BGM timeline commands.

use draft_model::{
    AudioEffectSlot, AudioFade, AudioPanBalance, CommandDelta, CommandDeltaName, CommandEvent,
    CommandState, Draft, MAX_SEGMENT_VOLUME_MILLIS, MaterialId, Microseconds, Segment, SegmentId,
    SegmentVolume, SourceTimerange, TargetTimerange, TimelineCommandResponse, TimelineSelection,
    TrackId,
};

use crate::{
    TimelineCommandError, TimelineCommandErrorKind,
    delta::{audio_property_delta, audio_segment_delta, track_mute_delta},
    history::push_undo_snapshot,
    timeline::{validate_timeline_rules, validate_track_unlocked},
};

pub fn add_audio_segment(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    track_id: TrackId,
    segment_id: SegmentId,
    material_id: MaterialId,
    source_timerange: SourceTimerange,
    target_timerange: TargetTimerange,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_draft = draft.clone();
    let track_index = find_track_index(&next_draft, &track_id)?;
    validate_track_unlocked(&next_draft.tracks[track_index])?;

    next_draft.tracks[track_index].segments.push(Segment::new(
        segment_id.clone(),
        material_id,
        source_timerange,
        target_timerange,
    ));
    validate_timeline_rules(&next_draft)?;
    let delta = audio_segment_delta(
        CommandDeltaName::AddAudioSegment,
        &track_id,
        next_draft.tracks[track_index]
            .segments
            .last()
            .expect("audio segment was just appended"),
        "audio segment added",
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
        "addAudioSegment",
        "audioSegmentAdded",
        CommandDeltaName::AddAudioSegment,
        delta,
    ))
}

pub fn set_segment_volume(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    segment_id: SegmentId,
    volume: SegmentVolume,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    validate_volume(volume)?;

    let mut next_draft = draft.clone();
    let (track_index, segment_index) = find_segment_location(&next_draft, &segment_id)?;
    validate_track_unlocked(&next_draft.tracks[track_index])?;

    next_draft.tracks[track_index].segments[segment_index].volume = volume;
    next_draft.tracks[track_index].segments[segment_index]
        .audio
        .gain_millis = volume.level_millis;
    validate_timeline_rules(&next_draft)?;
    let track_id = next_draft.tracks[track_index].track_id.clone();
    let delta = audio_property_delta(
        CommandDeltaName::SetSegmentVolume,
        &track_id,
        &next_draft.tracks[track_index].segments[segment_index],
        "segment volume changed",
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
        "setSegmentVolume",
        "segmentVolumeChanged",
        CommandDeltaName::SetSegmentVolume,
        delta,
    ))
}

pub fn update_segment_audio(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    segment_id: SegmentId,
    gain_millis: Option<u32>,
    pan_balance_millis: Option<i32>,
    fade_in_duration: Option<Microseconds>,
    fade_out_duration: Option<Microseconds>,
    effect_slots: Option<Vec<AudioEffectSlot>>,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_draft = draft.clone();
    let (track_index, segment_index) = find_segment_location(&next_draft, &segment_id)?;
    validate_track_unlocked(&next_draft.tracks[track_index])?;

    let segment = &mut next_draft.tracks[track_index].segments[segment_index];
    if let Some(gain_millis) = gain_millis {
        segment.audio.gain_millis = gain_millis;
        segment.volume = SegmentVolume {
            level_millis: gain_millis,
        };
    }
    if let Some(pan_balance_millis) = pan_balance_millis {
        segment.audio.pan_balance_millis = AudioPanBalance {
            balance_millis: pan_balance_millis,
        };
    }
    if let Some(fade_in_duration) = fade_in_duration {
        segment.audio.fade_in_duration = AudioFade {
            duration: fade_in_duration,
        };
    }
    if let Some(fade_out_duration) = fade_out_duration {
        segment.audio.fade_out_duration = AudioFade {
            duration: fade_out_duration,
        };
    }
    if let Some(effect_slots) = effect_slots {
        segment.audio.effect_slots = effect_slots;
    }

    validate_timeline_rules(&next_draft)?;
    let track_id = next_draft.tracks[track_index].track_id.clone();
    let delta = audio_property_delta(
        CommandDeltaName::UpdateSegmentAudio,
        &track_id,
        &next_draft.tracks[track_index].segments[segment_index],
        "segment audio changed",
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
        "updateSegmentAudio",
        "segmentAudioUpdated",
        CommandDeltaName::UpdateSegmentAudio,
        delta,
    ))
}

pub fn set_track_mute(
    draft: &Draft,
    command_state: &CommandState,
    selection: &TimelineSelection,
    track_id: TrackId,
    muted: bool,
) -> Result<TimelineCommandResponse, TimelineCommandError> {
    let mut next_draft = draft.clone();
    let track_index = find_track_index(&next_draft, &track_id)?;
    validate_track_unlocked(&next_draft.tracks[track_index])?;

    next_draft.tracks[track_index].muted = muted;
    validate_timeline_rules(&next_draft)?;
    let delta = track_mute_delta(&track_id, &next_draft.tracks[track_index].segments);

    Ok(response(
        next_draft,
        command_state,
        draft,
        selection,
        TimelineSelection {
            segment_ids: selection.segment_ids.clone(),
            track_ids: vec![track_id],
        },
        "setTrackMute",
        "trackMuteChanged",
        CommandDeltaName::SetTrackMute,
        delta,
    ))
}

fn validate_volume(volume: SegmentVolume) -> Result<(), TimelineCommandError> {
    if volume.level_millis > MAX_SEGMENT_VOLUME_MILLIS {
        return Err(TimelineCommandError::new(
            TimelineCommandErrorKind::DraftValidationFailed {
                message: format!(
                    "segment volume {} exceeds max {}",
                    volume.level_millis, MAX_SEGMENT_VOLUME_MILLIS
                ),
            },
        ));
    }
    Ok(())
}

fn response(
    draft: Draft,
    command_state: &CommandState,
    previous_draft: &Draft,
    previous_selection: &TimelineSelection,
    selection: TimelineSelection,
    history_label: &str,
    event_kind: &str,
    _command: CommandDeltaName,
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
