use std::collections::BTreeMap;

use draft_model::{
    Microseconds, SegmentId, Track, TrackId, TrackKind, TrackTransition, TransitionKind,
    TransitionReference,
};

#[test]
fn phase19_transition_relationship_model_is_adjacent_and_typed() {
    let mut parameters = BTreeMap::new();
    parameters.insert("curve".to_owned(), "linear".to_owned());

    let relationship = TrackTransition {
        from_segment_id: SegmentId::from("left-segment"),
        to_segment_id: SegmentId::from("right-segment"),
        reference: TransitionReference::FirstParty {
            transition: TransitionKind::Dissolve,
        },
        duration: Microseconds::new(500_000),
        parameters: parameters.clone(),
    };

    assert_eq!(relationship.from_segment_id, SegmentId::from("left-segment"));
    assert_eq!(relationship.to_segment_id, SegmentId::from("right-segment"));
    assert!(matches!(
        relationship.reference,
        TransitionReference::FirstParty {
            transition: TransitionKind::Dissolve
        }
    ));
    assert_eq!(relationship.duration, Microseconds::new(500_000));
    assert_eq!(relationship.parameters, parameters);
}

#[test]
fn phase19_track_owns_transition_relationships_not_segment_local_deltas() {
    let mut track = Track::new(TrackId::from("video-1"), TrackKind::Video, "main video");

    track.transitions.push(TrackTransition::dissolve(
        SegmentId::from("left-segment"),
        SegmentId::from("right-segment"),
        Microseconds::new(300_000),
    ));

    assert_eq!(track.transitions.len(), 1);
    assert_eq!(
        track.transitions[0].capability_id(),
        TransitionKind::Dissolve.capability_id()
    );
}

#[test]
fn phase19_transition_external_references_are_report_only_not_first_party_kinds() {
    let relationship = TrackTransition::external_reference(
        SegmentId::from("left-segment"),
        SegmentId::from("right-segment"),
        "jianying",
        "private-transition-id",
        Microseconds::new(400_000),
    );

    let external = relationship
        .external()
        .expect("provider transition must remain an external reference");
    assert_eq!(external.provider, "jianying");
    assert_eq!(external.effect_id, "private-transition-id");
    assert_eq!(
        relationship.capability_id(),
        "external:jianying:private-transition-id"
    );
    assert!(!matches!(
        relationship.reference,
        TransitionReference::FirstParty { .. }
    ));
}
