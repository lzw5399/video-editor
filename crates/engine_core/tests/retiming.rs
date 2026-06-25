const FRAME_STATE_RS: &str = include_str!("../src/frame_state.rs");

#[test]
fn phase19_retiming_engine_core_uses_segment_time_map_for_retimed_source_positions() {
    assert!(
        FRAME_STATE_RS.contains("SegmentTimeMap"),
        "engine_core must introduce a SegmentTimeMap rather than keeping linear source_position_at semantics"
    );
    assert!(
        FRAME_STATE_RS.contains("source_at_target") || FRAME_STATE_RS.contains("source_at("),
        "retimed source lookup must be evaluated by engine_core from integer/rational timeline inputs"
    );
}

#[test]
fn phase19_retiming_frame_state_carries_retime_transition_and_effect_facts() {
    assert!(
        FRAME_STATE_RS.contains("retime")
            && FRAME_STATE_RS.contains("transition")
            && FRAME_STATE_RS.contains("effect"),
        "resolved frame state must carry retime, transition, and effect facts into render graph/audio/export consumers"
    );
}
