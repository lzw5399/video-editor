---
phase: 09-complete-text-and-subtitle-system
reviewed: 2026-06-18T05:31:14Z
depth: standard
files_reviewed: 16
files_reviewed_list:
  - AGENTS.md
  - .planning/phases/09-complete-text-and-subtitle-system/09-REVIEW.md
  - .planning/phases/09-complete-text-and-subtitle-system/09-REVIEW-FIX.md
  - crates/draft_commands/src/text.rs
  - crates/draft_commands/tests/subtitle_commands.rs
  - crates/engine_core/src/text_layout.rs
  - crates/engine_core/tests/frame_state_snapshots.rs
  - crates/ffmpeg_compiler/src/ass.rs
  - crates/ffmpeg_compiler/tests/ass_snapshots.rs
  - crates/ffmpeg_compiler/tests/common/mod.rs
  - apps/desktop-electron/src/renderer/workspace/Inspector.tsx
  - crates/draft_model/src/timeline.rs
  - crates/draft_model/src/validation.rs
  - crates/draft_model/tests/schema_exports.rs
  - schemas/draft.schema.json
  - schemas/command.schema.json
findings:
  critical: 0
  warning: 1
  info: 0
  total: 1
status: issues_found
---

# Phase 09: Code Review Rerun Report

**Reviewed:** 2026-06-18T05:31:14Z
**Depth:** standard
**Files Reviewed:** 16
**Status:** issues_found

## Summary

Re-reviewed the Phase 09 code-review fixes against the four original findings in `09-REVIEW.md`.

- WR-01 is resolved: SRT parsing now builds cue blocks line-by-line and treats whitespace-only blank lines as separators, with regression coverage.
- WR-02 is resolved: `TextWrapping::Auto` now resolves deterministic line breaks in `engine_core`, and ASS sidecars consume the resolved overlay content.
- WR-03 is resolved: Inspector text box controls and validation now use Rust's `1..=1000` contract.
- WR-04 is only partially resolved: text box, style, color, and per-field layout millis constraints are now exported and tested, but the generated schemas still accept layout regions that Rust rejects when `xMillis + widthMillis > 1000` or `yMillis + heightMillis > 1000`.

Targeted regression tests run:

- `cargo test -p draft_commands subtitle_srt_import_splits_cues_on_whitespace_only_blank_lines`
- `cargo test -p engine_core text_layout_resolves_auto_wrapping_into_deterministic_lines`
- `cargo test -p ffmpeg_compiler ass_text_sidecar_uses_engine_resolved_auto_wrapping`
- `cargo test -p draft_model schema_exports_generated_contract_artifacts_from_rust`

All targeted tests passed.

## Narrative Findings (AI reviewer)

## Warnings

### WR-01: [WARNING] Generated schemas still allow text layout regions that Rust rejects

**File:** `schemas/draft.schema.json:1040`

**Issue:** Rust validation rejects text layout regions whose position plus size exceed the canvas-millis coordinate space: `xMillis + widthMillis > 1000` and `yMillis + heightMillis > 1000` fail in `crates/draft_model/src/validation.rs:488` and `crates/draft_model/src/validation.rs:494`. The regenerated schemas only constrain each field independently (`xMillis`/`yMillis` `0..=1000`, `widthMillis`/`heightMillis` `1..=1000`) in `schemas/draft.schema.json:1040` and `schemas/command.schema.json:3197`, so values such as `xMillis: 900, widthMillis: 200` still pass schema validation while failing the Rust model. The schema export negative cases in `crates/draft_model/tests/schema_exports.rs:1430` do not cover either summed layout overflow case, so this drift can recur unnoticed.

**Fix:** Extend the schema post-processing for `TextLayoutRegion` to encode the summed layout bounds, or explicitly add a schema-level extension plus a validator-side check if standard JSON Schema arithmetic is not acceptable. Add regression cases for both axes.

```rust
(
    "layout x plus width must be <= 1000",
    draft_value_with_text_layout_region(900, 100, 200, 800),
),
(
    "layout y plus height must be <= 1000",
    draft_value_with_text_layout_region(100, 900, 800, 200),
),

fn draft_value_with_text_layout_region(
    x_millis: u32,
    y_millis: u32,
    width_millis: u32,
    height_millis: u32,
) -> serde_json::Value {
    let mut value = draft_value_with_text_contract();
    *value.pointer_mut("/tracks/0/segments/0/text/layoutRegion/xMillis").unwrap() = json!(x_millis);
    *value.pointer_mut("/tracks/0/segments/0/text/layoutRegion/yMillis").unwrap() = json!(y_millis);
    *value.pointer_mut("/tracks/0/segments/0/text/layoutRegion/widthMillis").unwrap() = json!(width_millis);
    *value.pointer_mut("/tracks/0/segments/0/text/layoutRegion/heightMillis").unwrap() = json!(height_millis);
    value
}
```

---

_Reviewed: 2026-06-18T05:31:14Z_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_
