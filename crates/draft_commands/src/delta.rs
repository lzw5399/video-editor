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
