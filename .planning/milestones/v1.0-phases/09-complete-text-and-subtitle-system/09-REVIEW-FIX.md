---
phase: 09-complete-text-and-subtitle-system
fixed_at: 2026-06-18T05:26:16Z
review_path: .planning/phases/09-complete-text-and-subtitle-system/09-REVIEW.md
iteration: 1
findings_in_scope: 4
fixed: 4
skipped: 0
status: all_fixed
---

# Phase 09: Code Review Fix Report

**Fixed at:** 2026-06-18T05:26:16Z
**Source review:** `.planning/phases/09-complete-text-and-subtitle-system/09-REVIEW.md`
**Iteration:** 1

**Summary:**
- Findings in scope: 4
- Fixed: 4
- Skipped: 0

## Fixed Issues

### WR-01: SRT import does not split cues separated by whitespace-only blank lines

**Files modified:** `crates/draft_commands/src/text.rs`, `crates/draft_commands/tests/subtitle_commands.rs`
**Commit:** 176706c
**Applied fix:** Replaced literal double-newline SRT block splitting with line-by-line cue block collection that treats whitespace-only blank lines as separators, and added regression coverage.

### WR-02: `TextWrapping::Auto` is persisted but never resolved into wrapped lines

**Files modified:** `crates/engine_core/src/text_layout.rs`, `crates/engine_core/tests/frame_state_snapshots.rs`, `crates/ffmpeg_compiler/tests/common/mod.rs`, `crates/ffmpeg_compiler/tests/ass_snapshots.rs`
**Commit:** edc7fe4
**Applied fix:** Resolved auto wrapping deterministically in `engine_core` using resolved layout width, font size, and letter spacing; ASS sidecars now receive explicit resolved line breaks.

### WR-03: Inspector accepts text box values that Rust validation rejects

**Files modified:** `apps/desktop-electron/src/renderer/workspace/Inspector.tsx`
**Commit:** 9e95553
**Applied fix:** Aligned Inspector text box width/height controls, validation range, and Chinese error copy with Rust's `1..=1000` text box contract.

### WR-04: Generated schemas advertise weaker text constraints than Rust enforces

**Files modified:** `crates/draft_model/tests/schema_exports.rs`, `schemas/command.schema.json`, `schemas/draft.schema.json`
**Commit:** 629bf4e
**Applied fix:** Added schema export post-processing and regression checks for text box/layout millis, text colors, font size, line height, letter spacing, and stroke width constraints; regenerated draft and command schemas.

---

_Fixed: 2026-06-18T05:26:16Z_
_Fixer: the agent (gsd-code-fixer)_
_Iteration: 1_
