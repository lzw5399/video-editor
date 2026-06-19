//! Semantic command delta builders for accepted draft commands.

use draft_model::{
    ChangedEntity, CommandDelta, CommandName, DirtyDomain, DirtyRange, DirtyRangeSource, Draft,
    InvalidationScope, KeyframeProperty, MaterialId, Microseconds, Segment, SegmentId,
    TargetTimerange, TrackId,
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

const VISUAL_DOMAINS: &[DirtyDomain] = &[
    DirtyDomain::Visual,
    DirtyDomain::Preview,
    DirtyDomain::ExportPrep,
    DirtyDomain::Thumbnail,
    DirtyDomain::Proxy,
    DirtyDomain::GraphSnapshot,
    DirtyDomain::PreviewCache,
];

const VISUAL_CONSUMERS: &[DirtyDomain] = &[
    DirtyDomain::Preview,
    DirtyDomain::ExportPrep,
    DirtyDomain::Thumbnail,
    DirtyDomain::Proxy,
    DirtyDomain::GraphSnapshot,
    DirtyDomain::PreviewCache,
];

const CANVAS_DOMAINS: &[DirtyDomain] = &[
    DirtyDomain::Canvas,
    DirtyDomain::OutputProfile,
    DirtyDomain::Preview,
    DirtyDomain::ExportPrep,
    DirtyDomain::Thumbnail,
    DirtyDomain::Proxy,
    DirtyDomain::GraphSnapshot,
    DirtyDomain::PreviewCache,
];

const CANVAS_CONSUMERS: &[DirtyDomain] = &[
    DirtyDomain::Preview,
    DirtyDomain::ExportPrep,
    DirtyDomain::Thumbnail,
    DirtyDomain::Proxy,
    DirtyDomain::GraphSnapshot,
    DirtyDomain::PreviewCache,
];

const HISTORY_DOMAINS: &[DirtyDomain] = &[
    DirtyDomain::Timing,
    DirtyDomain::Visual,
    DirtyDomain::Text,
    DirtyDomain::Audio,
    DirtyDomain::Material,
    DirtyDomain::GraphSnapshot,
];

const HISTORY_CONSUMERS: &[DirtyDomain] = &[
    DirtyDomain::Preview,
    DirtyDomain::ExportPrep,
    DirtyDomain::Audio,
    DirtyDomain::Thumbnail,
    DirtyDomain::Waveform,
    DirtyDomain::Proxy,
    DirtyDomain::GraphSnapshot,
    DirtyDomain::PreviewCache,
];

const MATERIAL_DOMAINS: &[DirtyDomain] = &[
    DirtyDomain::Material,
    DirtyDomain::Preview,
    DirtyDomain::ExportPrep,
    DirtyDomain::Audio,
    DirtyDomain::Thumbnail,
    DirtyDomain::Waveform,
    DirtyDomain::Proxy,
    DirtyDomain::GraphSnapshot,
    DirtyDomain::PreviewCache,
];

const MATERIAL_CONSUMERS: &[DirtyDomain] = &[
    DirtyDomain::Preview,
    DirtyDomain::ExportPrep,
    DirtyDomain::Audio,
    DirtyDomain::Thumbnail,
    DirtyDomain::Waveform,
    DirtyDomain::Proxy,
    DirtyDomain::GraphSnapshot,
    DirtyDomain::PreviewCache,
];

const TRACK_DOMAINS: &[DirtyDomain] = &[DirtyDomain::Track, DirtyDomain::GraphSnapshot];

const TRACK_CONSUMERS: &[DirtyDomain] = &[
    DirtyDomain::Preview,
    DirtyDomain::ExportPrep,
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

pub fn segment_with_canvas_delta(
    track_id: &TrackId,
    segment: &Segment,
    draft: &Draft,
    reason: &'static str,
) -> CommandDelta {
    let mut changed_entities = vec![
        ChangedEntity::Draft {
            draft_id: draft.draft_id.clone(),
        },
        ChangedEntity::Canvas {
            draft_id: draft.draft_id.clone(),
        },
    ];
    changed_entities.extend(segment_entities(
        track_id,
        &segment.segment_id,
        &segment.material_id,
    ));

    let mut changed_domains = CANVAS_DOMAINS.to_vec();
    push_domain(&mut changed_domains, DirtyDomain::Timing);
    push_domain(&mut changed_domains, DirtyDomain::Visual);
    push_domain(&mut changed_domains, DirtyDomain::Material);

    let mut consumer_domains = CANVAS_CONSUMERS.to_vec();
    for domain in SEGMENT_CONSUMERS {
        push_domain(&mut consumer_domains, *domain);
    }

    CommandDelta {
        command: CommandName::AddSegment,
        changed_entities,
        changed_domains,
        changed_ranges: vec![DirtyRange {
            target_timerange: draft_duration_range(draft),
            source: DirtyRangeSource::FullDraft,
        }],
        invalidation: InvalidationScope {
            full_draft: true,
            material_ids: segment_material_ids(&segment.material_id),
            graph_node_ids: Vec::new(),
            consumer_domains,
        },
        reason: reason.to_owned(),
    }
}

pub fn track_delta(command: CommandName, track_id: &TrackId, reason: &'static str) -> CommandDelta {
    CommandDelta::targeted(
        command,
        vec![ChangedEntity::Track {
            track_id: track_id.clone(),
        }],
        TRACK_DOMAINS.to_vec(),
        Vec::new(),
        InvalidationScope::targeted(Vec::new(), TRACK_CONSUMERS.to_vec()),
        reason,
    )
}

pub fn track_visibility_delta(track_id: &TrackId, segments: &[Segment]) -> CommandDelta {
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

    let mut domains = TRACK_DOMAINS.to_vec();
    push_domain(&mut domains, DirtyDomain::Visual);
    push_domain(&mut domains, DirtyDomain::Preview);
    push_domain(&mut domains, DirtyDomain::ExportPrep);
    push_domain(&mut domains, DirtyDomain::Thumbnail);
    push_domain(&mut domains, DirtyDomain::PreviewCache);

    let mut consumers = TRACK_CONSUMERS.to_vec();
    push_domain(&mut consumers, DirtyDomain::Thumbnail);

    CommandDelta::targeted(
        CommandName::SetTrackVisibility,
        entities,
        domains,
        ranges,
        InvalidationScope::targeted(material_ids, consumers),
        "track visibility changed",
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

pub fn visual_segment_delta(
    command: CommandName,
    track_id: &TrackId,
    segment: &Segment,
    reason: &'static str,
) -> CommandDelta {
    CommandDelta::targeted(
        command,
        segment_entities(track_id, &segment.segment_id, &segment.material_id),
        VISUAL_DOMAINS.to_vec(),
        vec![current_range(segment.target_timerange.clone())],
        InvalidationScope::targeted(
            segment_material_ids(&segment.material_id),
            VISUAL_CONSUMERS.to_vec(),
        ),
        reason,
    )
}

pub fn keyframe_delta(
    command: CommandName,
    track_id: &TrackId,
    segment: &Segment,
    property: KeyframeProperty,
    at: Microseconds,
    reason: &'static str,
) -> CommandDelta {
    let mut entities = segment_entities(track_id, &segment.segment_id, &segment.material_id);
    entities.push(ChangedEntity::Keyframe {
        track_id: track_id.clone(),
        segment_id: segment.segment_id.clone(),
        property: property.clone(),
        at,
    });

    let changed_domains = keyframe_domains(property);
    let consumer_domains = consumer_domains_for_semantic_domains(&changed_domains);
    CommandDelta::targeted(
        command,
        entities,
        changed_domains,
        vec![current_range(segment.target_timerange.clone())],
        InvalidationScope::targeted(segment_material_ids(&segment.material_id), consumer_domains),
        reason,
    )
}

pub fn canvas_delta(draft: &Draft) -> CommandDelta {
    let range = DirtyRange {
        target_timerange: draft_duration_range(draft),
        source: DirtyRangeSource::FullDraft,
    };
    CommandDelta {
        command: CommandName::UpdateDraftCanvasConfig,
        changed_entities: vec![
            ChangedEntity::Draft {
                draft_id: draft.draft_id.clone(),
            },
            ChangedEntity::Canvas {
                draft_id: draft.draft_id.clone(),
            },
        ],
        changed_domains: CANVAS_DOMAINS.to_vec(),
        changed_ranges: vec![range],
        invalidation: InvalidationScope {
            full_draft: true,
            material_ids: Vec::new(),
            graph_node_ids: Vec::new(),
            consumer_domains: CANVAS_CONSUMERS.to_vec(),
        },
        reason: "draft canvas config changed".to_owned(),
    }
}

pub fn material_dependency_delta(
    command: CommandName,
    draft: &Draft,
    changed_material_ids: &[MaterialId],
    reason: &'static str,
) -> CommandDelta {
    let mut entities = Vec::new();
    let mut ranges = Vec::new();
    let mut material_ids = Vec::new();

    for material_id in changed_material_ids {
        push_entity(
            &mut entities,
            ChangedEntity::Material {
                material_id: material_id.clone(),
            },
        );
        push_material_id(&mut material_ids, material_id);
    }

    for (track_id, segment) in draft_segments(draft) {
        if changed_material_ids
            .iter()
            .any(|material_id| material_id == &segment.material_id)
        {
            push_segment_entities(&mut entities, &mut material_ids, track_id, segment);
            ranges.push(DirtyRange {
                target_timerange: segment.target_timerange.clone(),
                source: DirtyRangeSource::MaterialWide,
            });
        }
    }

    if ranges.is_empty() {
        ranges.push(DirtyRange {
            target_timerange: draft_duration_range(draft),
            source: DirtyRangeSource::MaterialWide,
        });
    }

    CommandDelta::targeted(
        command,
        entities,
        MATERIAL_DOMAINS.to_vec(),
        ranges,
        InvalidationScope::targeted(material_ids, MATERIAL_CONSUMERS.to_vec()),
        reason,
    )
}

pub fn restored_draft_delta(
    command: CommandName,
    previous_draft: &Draft,
    restored_draft: &Draft,
    reason: &'static str,
) -> CommandDelta {
    if previous_draft == restored_draft {
        return CommandDelta::none(command, reason);
    }

    if previous_draft.canvas_config != restored_draft.canvas_config {
        return CommandDelta::full_draft(
            command,
            vec![
                ChangedEntity::Draft {
                    draft_id: restored_draft.draft_id.clone(),
                },
                ChangedEntity::Canvas {
                    draft_id: restored_draft.draft_id.clone(),
                },
            ],
            CANVAS_DOMAINS.to_vec(),
            CANVAS_CONSUMERS.to_vec(),
            reason,
        );
    }

    let mut entities = Vec::new();
    let mut ranges = Vec::new();
    let mut material_ids = Vec::new();

    for (track_id, previous_segment) in draft_segments(previous_draft) {
        match find_segment(restored_draft, &previous_segment.segment_id) {
            Some((restored_track_id, restored_segment)) => {
                if previous_segment != restored_segment || track_id != restored_track_id {
                    push_segment_entities(
                        &mut entities,
                        &mut material_ids,
                        track_id,
                        previous_segment,
                    );
                    if track_id != restored_track_id {
                        push_segment_entities(
                            &mut entities,
                            &mut material_ids,
                            restored_track_id,
                            restored_segment,
                        );
                    }
                    ranges.push(previous_range(previous_segment.target_timerange.clone()));
                    ranges.push(current_range(restored_segment.target_timerange.clone()));
                }
            }
            None => {
                push_segment_entities(&mut entities, &mut material_ids, track_id, previous_segment);
                ranges.push(previous_range(previous_segment.target_timerange.clone()));
            }
        }
    }

    for (track_id, restored_segment) in draft_segments(restored_draft) {
        if find_segment(previous_draft, &restored_segment.segment_id).is_none() {
            push_segment_entities(&mut entities, &mut material_ids, track_id, restored_segment);
            ranges.push(current_range(restored_segment.target_timerange.clone()));
        }
    }

    if ranges.is_empty() {
        CommandDelta::full_draft(
            command,
            vec![ChangedEntity::Draft {
                draft_id: restored_draft.draft_id.clone(),
            }],
            HISTORY_DOMAINS.to_vec(),
            HISTORY_CONSUMERS.to_vec(),
            reason,
        )
    } else {
        CommandDelta::targeted(
            command,
            entities,
            HISTORY_DOMAINS.to_vec(),
            ranges,
            InvalidationScope::targeted(material_ids, HISTORY_CONSUMERS.to_vec()),
            reason,
        )
    }
}

pub fn consumer_domains_for_semantic_domains(domains: &[DirtyDomain]) -> Vec<DirtyDomain> {
    let mut consumers = Vec::new();
    for domain in domains {
        match domain {
            DirtyDomain::Track => push_all(&mut consumers, TRACK_CONSUMERS),
            DirtyDomain::Timing => push_all(
                &mut consumers,
                &[
                    DirtyDomain::Preview,
                    DirtyDomain::ExportPrep,
                    DirtyDomain::Audio,
                    DirtyDomain::Thumbnail,
                    DirtyDomain::Proxy,
                    DirtyDomain::GraphSnapshot,
                    DirtyDomain::PreviewCache,
                ],
            ),
            DirtyDomain::Visual => push_all(&mut consumers, VISUAL_CONSUMERS),
            DirtyDomain::Text => push_all(&mut consumers, TEXT_CONSUMERS),
            DirtyDomain::Audio => push_all(
                &mut consumers,
                &[
                    DirtyDomain::Preview,
                    DirtyDomain::ExportPrep,
                    DirtyDomain::Audio,
                    DirtyDomain::Waveform,
                    DirtyDomain::GraphSnapshot,
                    DirtyDomain::PreviewCache,
                ],
            ),
            DirtyDomain::Material | DirtyDomain::RuntimeCapabilities => push_all(
                &mut consumers,
                &[
                    DirtyDomain::Preview,
                    DirtyDomain::ExportPrep,
                    DirtyDomain::Audio,
                    DirtyDomain::Thumbnail,
                    DirtyDomain::Waveform,
                    DirtyDomain::Proxy,
                    DirtyDomain::GraphSnapshot,
                    DirtyDomain::PreviewCache,
                ],
            ),
            DirtyDomain::Canvas | DirtyDomain::OutputProfile => push_all(
                &mut consumers,
                &[
                    DirtyDomain::Preview,
                    DirtyDomain::ExportPrep,
                    DirtyDomain::Thumbnail,
                    DirtyDomain::Proxy,
                    DirtyDomain::GraphSnapshot,
                    DirtyDomain::PreviewCache,
                ],
            ),
            DirtyDomain::Effect | DirtyDomain::Filter | DirtyDomain::Transition => {
                push_all(&mut consumers, VISUAL_CONSUMERS);
            }
            DirtyDomain::Preview
            | DirtyDomain::ExportPrep
            | DirtyDomain::Thumbnail
            | DirtyDomain::Waveform
            | DirtyDomain::Proxy
            | DirtyDomain::GraphSnapshot
            | DirtyDomain::PreviewCache => push_domain(&mut consumers, *domain),
        }
    }
    sort_consumer_domains(&mut consumers);
    consumers
}

fn keyframe_domains(property: KeyframeProperty) -> Vec<DirtyDomain> {
    match property {
        KeyframeProperty::Volume => AUDIO_PROPERTY_DOMAINS.to_vec(),
        KeyframeProperty::TextFontSize
        | KeyframeProperty::TextColor
        | KeyframeProperty::TextLineHeight
        | KeyframeProperty::TextLetterSpacing
        | KeyframeProperty::TextLayoutX
        | KeyframeProperty::TextLayoutY
        | KeyframeProperty::TextLayoutWidth
        | KeyframeProperty::TextLayoutHeight => TEXT_DOMAINS.to_vec(),
        KeyframeProperty::VisualPositionX
        | KeyframeProperty::VisualPositionY
        | KeyframeProperty::VisualScaleX
        | KeyframeProperty::VisualScaleY
        | KeyframeProperty::VisualRotation
        | KeyframeProperty::VisualOpacity
        | KeyframeProperty::StickerPositionX
        | KeyframeProperty::StickerPositionY
        | KeyframeProperty::StickerScaleX
        | KeyframeProperty::StickerScaleY => VISUAL_DOMAINS.to_vec(),
        KeyframeProperty::FilterParameterUnsupported => {
            let mut domains = VISUAL_DOMAINS.to_vec();
            push_domain(&mut domains, DirtyDomain::Filter);
            domains
        }
    }
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

fn push_segment_entities(
    entities: &mut Vec<ChangedEntity>,
    material_ids: &mut Vec<MaterialId>,
    track_id: &TrackId,
    segment: &Segment,
) {
    push_entity(
        entities,
        ChangedEntity::Track {
            track_id: track_id.clone(),
        },
    );
    push_entity(
        entities,
        ChangedEntity::Segment {
            track_id: track_id.clone(),
            segment_id: segment.segment_id.clone(),
        },
    );
    push_entity(
        entities,
        ChangedEntity::Material {
            material_id: segment.material_id.clone(),
        },
    );
    push_material_id(material_ids, &segment.material_id);
}

fn push_entity(entities: &mut Vec<ChangedEntity>, entity: ChangedEntity) {
    if !entities.contains(&entity) {
        entities.push(entity);
    }
}

fn draft_segments(draft: &Draft) -> impl Iterator<Item = (&TrackId, &Segment)> {
    draft.tracks.iter().flat_map(|track| {
        track
            .segments
            .iter()
            .map(move |segment| (&track.track_id, segment))
    })
}

fn find_segment<'a>(
    draft: &'a Draft,
    segment_id: &SegmentId,
) -> Option<(&'a TrackId, &'a Segment)> {
    draft.tracks.iter().find_map(|track| {
        track
            .segments
            .iter()
            .find(|segment| &segment.segment_id == segment_id)
            .map(|segment| (&track.track_id, segment))
    })
}

fn push_all(domains: &mut Vec<DirtyDomain>, additions: &[DirtyDomain]) {
    for domain in additions {
        push_domain(domains, *domain);
    }
}

fn push_domain(domains: &mut Vec<DirtyDomain>, domain: DirtyDomain) {
    if !domains.contains(&domain) {
        domains.push(domain);
    }
}

fn sort_consumer_domains(domains: &mut [DirtyDomain]) {
    domains.sort_by_key(|domain| match domain {
        DirtyDomain::Preview => 0,
        DirtyDomain::ExportPrep => 1,
        DirtyDomain::Audio => 2,
        DirtyDomain::Thumbnail => 3,
        DirtyDomain::Waveform => 4,
        DirtyDomain::Proxy => 5,
        DirtyDomain::GraphSnapshot => 6,
        DirtyDomain::PreviewCache => 7,
        _ => 8,
    });
}

fn draft_duration_range(draft: &Draft) -> TargetTimerange {
    let duration = draft
        .tracks
        .iter()
        .flat_map(|track| track.segments.iter())
        .filter_map(|segment| {
            segment
                .target_timerange
                .start
                .get()
                .checked_add(segment.target_timerange.duration.get())
        })
        .max()
        .unwrap_or(0);
    TargetTimerange::new(Microseconds::ZERO, Microseconds::new(duration))
}
