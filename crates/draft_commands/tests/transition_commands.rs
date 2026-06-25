const LIB_RS: &str = include_str!("../src/lib.rs");
const TIMELINE_RS: &str = include_str!("../src/timeline.rs");

#[test]
fn phase19_transition_commands_contract_is_first_class_and_undoable() {
    assert!(
        LIB_RS.contains("pub mod transition"),
        "transition commands must live in a Rust draft_commands::transition module"
    );
    assert!(
        TIMELINE_RS.contains("SetSegmentTransitionCommandPayload")
            && TIMELINE_RS.contains("set_segment_transition"),
        "timeline dispatcher must route an explicit set_segment_transition command payload"
    );
    assert!(
        TIMELINE_RS.contains("push_undo_snapshot") && TIMELINE_RS.contains("setSegmentTransition"),
        "committed transition edits must push one undo entry labeled setSegmentTransition"
    );
}

#[test]
fn phase19_transition_commands_validate_adjacency_and_overlap_in_rust() {
    assert!(
        TIMELINE_RS.contains("validate_transition_adjacency")
            || TIMELINE_RS.contains("validate_transition_overlap"),
        "transition adjacency/overlap windows must be validated in Rust, not in renderer hit testing"
    );
    assert!(
        TIMELINE_RS.contains("TransitionAdjacency") || TIMELINE_RS.contains("TransitionWindow"),
        "transition semantics must model the adjacent segment relationship explicitly"
    );
}
