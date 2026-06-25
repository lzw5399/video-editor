const LIB_RS: &str = include_str!("../src/lib.rs");
const TIMELINE_RS: &str = include_str!("../src/timeline.rs");

#[test]
fn phase19_retiming_commands_contract_is_undoable_and_rust_owned() {
    assert!(
        LIB_RS.contains("pub mod retiming"),
        "retiming commands must live in a Rust draft_commands::retiming module"
    );
    assert!(
        TIMELINE_RS.contains("UpdateSegmentRetimingCommandPayload")
            && TIMELINE_RS.contains("update_segment_retiming"),
        "timeline dispatcher must route an explicit update_segment_retiming command payload"
    );
    assert!(
        TIMELINE_RS.contains("push_undo_snapshot") && TIMELINE_RS.contains("updateSegmentRetiming"),
        "committed retiming edits must push one undo entry labeled updateSegmentRetiming"
    );
}

#[test]
fn phase19_retiming_commands_split_trim_move_preserve_source_mapping_contracts() {
    assert!(
        TIMELINE_RS.contains("retime_source_mapping_after_split")
            || TIMELINE_RS.contains("preserve_retime_source_mapping"),
        "split/trim/move commands must preserve deterministic retimed source mapping instead of renderer duration math"
    );
    assert!(
        TIMELINE_RS.contains("DirtyDomain::Timing") || TIMELINE_RS.contains("DirtyDomain::Retime"),
        "retiming commands must emit timing dirty facts for downstream preview/export/audio invalidation"
    );
}
