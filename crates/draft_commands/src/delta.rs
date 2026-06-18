//! Semantic command delta builders for accepted draft commands.

use draft_model::{
    ChangedEntity, CommandDelta, CommandName, DirtyDomain, DirtyRange, DirtyRangeSource,
    InvalidationScope, MaterialId, Segment, SegmentId, TargetTimerange, TrackId,
};

const SEGMENT_DOMAINS: &[DirtyDomain] = &[
    DirtyDomain::Timing,
    DirtyDomain::Visual,
    DirtyDomain::Material,
    DirtyDomain::Preview,
    DirtyDomain::ExportPrep,
    DirtyDomain::Thumbnail,
    DirtyDomain::Proxy,
    DirtyDomain::GraphSnapshot,
    DirtyDomain::PreviewCache,
];

const SEGMENT_CONSUMERS: &[DirtyDomain] = &[
    DirtyDomain::Preview,
    DirtyDomain::ExportPrep,
    DirtyDomain::Thumbnail,
    DirtyDomain::Proxy,
    DirtyDomain::GraphSnapshot,
    DirtyDomain::PreviewCache,
];

const TEXT_DOMAINS: &[DirtyDomain] = &[
    DirtyDomain::Text,
    DirtyDomain::Visual,
    DirtyDomain::Material,
    DirtyDomain::Preview,
    DirtyDomain::ExportPrep,
    DirtyDomain::Thumbnail,
    DirtyDomain::GraphSnapshot,
    DirtyDomain::PreviewCache,
];

const TEXT_CONSUMERS: &[DirtyDomain] = &[
    DirtyDomain::Preview,
    DirtyDomain::ExportPrep,
    DirtyDomain::Thumbnail,
    DirtyDomain::GraphSnapshot,
    DirtyDomain::PreviewCache,
];

const AUDIO_SEGMENT_DOMAINS: &[DirtyDomain] = &[
    DirtyDomain::Timing,
    DirtyDomain::Audio,
    DirtyDomain::Material,
    DirtyDomain::Preview,
    DirtyDomain::ExportPrep,
    DirtyDomain::Waveform,
    DirtyDomain::GraphSnapshot,
    DirtyDomain::PreviewCache,
];

const AUDIO_PROPERTY_DOMAINS: &[DirtyDomain] = &[
    DirtyDomain::Audio,
    DirtyDomain::ExportPrep,
    DirtyDomain::Waveform,
    DirtyDomain::GraphSnapshot,
    DirtyDomain::PreviewCache,
];

const AUDIO_CONSUMERS: &[DirtyDomain] = &[
    DirtyDomain::Preview,
    DirtyDomain::ExportPrep,
    DirtyDomain::Audio,
    DirtyDomain::Waveform,
    DirtyDomain::GraphSnapshot,
    DirtyDomain::PreviewCache,
];

pub fn current_range(target_timerange: TargetTimerange) -> DirtyRange {
    DirtyRange {
        target_timerange,
        source: DirtyRangeSource::Current,
    }
}

pub fn previous_range(target_timerange: TargetTimerange) -> DirtyRange {
    DirtyRange {
        target_timerange,
        source: DirtyRangeSource::Previous,
    }
}

pub fn previous_and_current_range(target_timerange: TargetTimerange) -> DirtyRange {
    DirtyRange {
        target_timerange,
        source: DirtyRangeSource::PreviousAndCurrent,
    }
}

pub fn segment_delta(
    command: CommandName,
    track_id: &TrackId,
    segment: &Segment,
    changed_ranges: Vec<DirtyRange>,
    reason: &'static str,
) -> CommandDelta {
    CommandDelta::targeted(
        command,
        segment_entities(track_id, &segment.segment_id, &segment.material_id),
        SEGMENT_DOMAINS.to_vec(),
        changed_ranges,
        InvalidationScope::targeted(
            segment_material_ids(&segment.material_id),
            SEGMENT_CONSUMERS.to_vec(),
        ),
        reason,
    )
}

pub fn moved_segment_delta(
    source_track_id: &TrackId,
    target_track_id: &TrackId,
    segment: &Segment,
    previous_target_timerange: TargetTimerange,
    current_target_timerange: TargetTimerange,
) -> CommandDelta {
    let mut entities = vec![ChangedEntity::Track {
        track_id: source_track_id.clone(),
    }];
    if source_track_id != target_track_id {
        entities.push(ChangedEntity::Track {
            track_id: target_track_id.clone(),
        });
    }
    entities.push(ChangedEntity::Segment {
        track_id: target_track_id.clone(),
        segment_id: segment.segment_id.clone(),
    });
    entities.push(ChangedEntity::Material {
        material_id: segment.material_id.clone(),
    });

    CommandDelta::targeted(
        CommandName::MoveSegment,
        entities,
        SEGMENT_DOMAINS.to_vec(),
        vec![
            previous_range(previous_target_timerange),
            current_range(current_target_timerange),
        ],
        InvalidationScope::targeted(
            segment_material_ids(&segment.material_id),
            SEGMENT_CONSUMERS.to_vec(),
        ),
        "segment moved",
    )
}

pub fn split_segment_delta(
    track_id: &TrackId,
    original_segment: &Segment,
    right_segment_id: &SegmentId,
    original_target_timerange: TargetTimerange,
) -> CommandDelta {
    let mut entities = segment_entities(
        track_id,
        &original_segment.segment_id,
        &original_segment.material_id,
    );
    entities.insert(
        2,
        ChangedEntity::Segment {
            track_id: track_id.clone(),
            segment_id: right_segment_id.clone(),
        },
    );

    CommandDelta::targeted(
        CommandName::SplitSegment,
        entities,
        SEGMENT_DOMAINS.to_vec(),
        vec![previous_and_current_range(original_target_timerange)],
        InvalidationScope::targeted(
            segment_material_ids(&original_segment.material_id),
            SEGMENT_CONSUMERS.to_vec(),
        ),
        "segment split",
    )
}

pub fn text_segment_delta(
    command: CommandName,
    track_id: &TrackId,
    segment: &Segment,
    reason: &'static str,
) -> CommandDelta {
    text_segments_delta(command, track_id, [segment], reason)
}

pub fn text_segments_delta<'a>(
    command: CommandName,
    track_id: &TrackId,
    segments: impl IntoIterator<Item = &'a Segment>,
    reason: &'static str,
) -> CommandDelta {
    let mut entities = vec![ChangedEntity::Track {
        track_id: track_id.clone(),
    }];
    let mut ranges = Vec::new();
    let mut material_ids = Vec::new();

    for segment in segments {
        entities.push(ChangedEntity::Segment {
            track_id: track_id.clone(),
            segment_id: segment.segment_id.clone(),
        });
        entities.push(ChangedEntity::Material {
            material_id: segment.material_id.clone(),
        });
        push_material_id(&mut material_ids, &segment.material_id);
        ranges.push(current_range(segment.target_timerange.clone()));
    }

    CommandDelta::targeted(
        command,
        entities,
        TEXT_DOMAINS.to_vec(),
        ranges,
        InvalidationScope::targeted(material_ids, TEXT_CONSUMERS.to_vec()),
        reason,
    )
}

pub fn audio_segment_delta(
    command: CommandName,
    track_id: &TrackId,
    segment: &Segment,
    reason: &'static str,
) -> CommandDelta {
    CommandDelta::targeted(
        command,
        segment_entities(track_id, &segment.segment_id, &segment.material_id),
        AUDIO_SEGMENT_DOMAINS.to_vec(),
        vec![current_range(segment.target_timerange.clone())],
        InvalidationScope::targeted(
            segment_material_ids(&segment.material_id),
            AUDIO_CONSUMERS.to_vec(),
        ),
        reason,
    )
}

pub fn audio_property_delta(
    command: CommandName,
    track_id: &TrackId,
    segment: &Segment,
    reason: &'static str,
) -> CommandDelta {
    CommandDelta::targeted(
        command,
        segment_entities(track_id, &segment.segment_id, &segment.material_id),
        AUDIO_PROPERTY_DOMAINS.to_vec(),
        vec![current_range(segment.target_timerange.clone())],
        InvalidationScope::targeted(
            segment_material_ids(&segment.material_id),
            AUDIO_CONSUMERS.to_vec(),
        ),
        reason,
    )
}

pub fn track_mute_delta(track_id: &TrackId, segments: &[Segment]) -> CommandDelta {
    let mut entities = vec![ChangedEntity::Track {
        track_id: track_id.clone(),
    }];
    let mut ranges = Vec::new();
    let mut material_ids = Vec::new();

    for segment in segments {
        entities.push(ChangedEntity::Segment {
            track_id: track_id.clone(),
            segment_id: segment.segment_id.clone(),
        });
        entities.push(ChangedEntity::Material {
            material_id: segment.material_id.clone(),
        });
        push_material_id(&mut material_ids, &segment.material_id);
        ranges.push(current_range(segment.target_timerange.clone()));
    }

    CommandDelta::targeted(
        CommandName::SetTrackMute,
        entities,
        AUDIO_PROPERTY_DOMAINS.to_vec(),
        ranges,
        InvalidationScope::targeted(material_ids, AUDIO_CONSUMERS.to_vec()),
        "track mute changed",
    )
}

fn segment_entities(
    track_id: &TrackId,
    segment_id: &SegmentId,
    material_id: &MaterialId,
) -> Vec<ChangedEntity> {
    vec![
        ChangedEntity::Track {
            track_id: track_id.clone(),
        },
        ChangedEntity::Segment {
            track_id: track_id.clone(),
            segment_id: segment_id.clone(),
        },
        ChangedEntity::Material {
            material_id: material_id.clone(),
        },
    ]
}

fn segment_material_ids(material_id: &MaterialId) -> Vec<MaterialId> {
    vec![material_id.clone()]
}

fn push_material_id(material_ids: &mut Vec<MaterialId>, material_id: &MaterialId) {
    if !material_ids.iter().any(|existing| existing == material_id) {
        material_ids.push(material_id.clone());
    }
}
