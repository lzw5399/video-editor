---
phase: 09-complete-text-and-subtitle-system
plan: 03
subsystem: subtitle-commands
tags: [rust, srt, subtitle, text, bindings, generated-contracts]
requires:
  - phase: 09-complete-text-and-subtitle-system
    provides: Complete TextSegment schema and text render/ASS propagation from Plans 09-01 and 09-02
provides:
  - Rust-owned importSubtitleSrt command payload and timeline route
  - SRT parser that creates subtitle TextSegment batches with integer microsecond timing
  - Atomic malformed-SRT rejection and single-command undo/redo history
  - Node binding route plus generated command schema and desktop CommandEnvelope contract
affects: [draft-commands, bindings-node, desktop-contracts, phase-09-ui]
tech-stack:
  added: []
  patterns: [tdd-red-green, rust-owned-subtitle-import, generated-command-contracts]
key-files:
  created:
    - crates/draft_commands/tests/subtitle_commands.rs
    - crates/bindings_node/tests/text_commands.rs
    - .planning/phases/09-complete-text-and-subtitle-system/09-03-SUMMARY.md
  modified:
    - crates/draft_model/src/lib.rs
    - crates/draft_commands/src/text.rs
    - crates/draft_commands/src/timeline.rs
    - crates/draft_commands/tests/text_audio_commands.rs
    - crates/bindings_node/src/lib.rs
    - crates/bindings_node/tests/preview_commands.rs
    - crates/draft_model/tests/schema_exports.rs
    - schemas/command.schema.json
    - apps/desktop-electron/src/generated/CommandEnvelope.ts
key-decisions:
  - "Subtitle SRT import is a Rust timeline command named importSubtitleSrt, not a renderer parser or separate subtitle model."
  - "Imported subtitles create TextSegment values with source=subtitle while reusing the caller-provided text style, text box, layout region, and wrapping template."
  - "Segment and material IDs are deterministic from caller-provided prefixes plus cue ordinal so imports are reproducible across binding and command tests."
patterns-established:
  - "Batch timeline imports parse and validate into intermediate cues before mutating a cloned draft, then push one undo snapshot."
  - "Binding tests that are required by filtered cargo commands must include the filter term in their test names."
requirements-completed: [TEXT2-02, TEXT2-03]
duration: 12 min
completed: 2026-06-18
---

# Phase 09 Plan 03: Subtitle SRT Import Summary

**Rust-owned SRT import creates styled subtitle text segments through generated command contracts and the Node binding route.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-06-18T04:07:06Z
- **Completed:** 2026-06-18T04:19:05Z
- **Tasks:** 2
- **Files modified:** 11

## Accomplishments

- Added `ImportSubtitleSrtCommandPayload`, `CommandName::ImportSubtitleSrt`, and `CommandPayload::ImportSubtitleSrt`.
- Implemented Rust SRT parsing with `HH:MM:SS,mmm` timing, integer microsecond offsets, target text-track creation/selection, text material creation, and subtitle `TextSegment` generation.
- Ensured malformed SRT rejects before draft mutation from the caller perspective and import undo/redo behaves as one timeline command.
- Routed `importSubtitleSrt` through `bindings_node::execute_command`.
- Regenerated `schemas/command.schema.json` and `apps/desktop-electron/src/generated/CommandEnvelope.ts`.

## Task Commits

1. **Task 09-03-01: Implement Rust SRT import semantics** - `07777ba` (test RED), `3467259` (feat GREEN)
2. **Task 09-03-02: Expose subtitle import through bindings and generated contracts** - `59a4061` (test RED), `14f6d53` (feat GREEN)

## Files Created/Modified

- `crates/draft_model/src/lib.rs` - Adds `importSubtitleSrt` command name, payload variant, and payload schema.
- `crates/draft_commands/src/text.rs` - Parses SRT and imports subtitle text segments atomically with one undo snapshot.
- `crates/draft_commands/src/timeline.rs` - Routes `ImportSubtitleSrt` through timeline command execution.
- `crates/draft_commands/tests/subtitle_commands.rs` - Covers track creation, existing-track targeting, timing offset, style/layout template, malformed rejection, route execution, and undo/redo.
- `crates/draft_commands/tests/text_audio_commands.rs` - Updates stale text fixture literals for the Phase 09 complete text schema.
- `crates/bindings_node/src/lib.rs` - Allows and routes `importSubtitleSrt` through `execute_command`.
- `crates/bindings_node/tests/text_commands.rs` - Covers binding success and malformed-SRT error envelopes under the required filter.
- `crates/bindings_node/tests/preview_commands.rs` - Updates stale text fixture literals for the Phase 09 complete text schema.
- `crates/draft_model/tests/schema_exports.rs` - Includes `ImportSubtitleSrtCommandPayload` in schema and TypeScript generation.
- `schemas/command.schema.json` - Regenerated command schema with `importSubtitleSrt`.
- `apps/desktop-electron/src/generated/CommandEnvelope.ts` - Regenerated desktop command contract with `ImportSubtitleSrtCommandPayload`.

## Decisions Made

- `importSubtitleSrt` accepts raw SRT content at the Rust command boundary; the renderer may read a file, but parsing and segment creation stay in Rust.
- The command creates a missing `TrackKind::Text` target track using caller-provided track id/name, or appends to an existing unlocked text track.
- Each subtitle cue gets `sourceTimerange.start = 0`, `sourceTimerange.duration = cue duration`, and `targetTimerange.start = cue start + timeOffset`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Updated stale draft_commands text fixture literals**
- **Found during:** Task 09-03-01 RED verification
- **Issue:** `crates/draft_commands/tests/text_audio_commands.rs` still used pre-09-01 `TextSegment` and `TextStyle` struct literals, preventing `cargo test -p draft_commands subtitle -- --nocapture` from compiling all integration tests.
- **Fix:** Added defaulted Phase 09 text fields to the fixture without changing its behavior.
- **Files modified:** `crates/draft_commands/tests/text_audio_commands.rs`
- **Verification:** `cargo test -p draft_commands subtitle -- --nocapture`
- **Committed in:** `3467259`

**2. [Rule 3 - Blocking] Updated stale bindings preview text fixture literals**
- **Found during:** Task 09-03-02 binding verification
- **Issue:** `crates/bindings_node/tests/preview_commands.rs` still used pre-09-01 `TextSegment` and `TextStyle` struct literals, preventing the required binding test command from compiling.
- **Fix:** Added defaulted Phase 09 text fields to the preview fixture without changing preview behavior.
- **Files modified:** `crates/bindings_node/tests/preview_commands.rs`
- **Verification:** `cargo test -p bindings_node text_commands -- --nocapture`
- **Committed in:** `14f6d53`

**3. [Rule 2 - Missing Critical] Ensured required filtered binding tests execute**
- **Found during:** Task 09-03-02 binding verification
- **Issue:** The new binding tests initially lived in `text_commands.rs` but their function names did not include `text_commands`, so the required filtered command compiled the file but ran zero tests.
- **Fix:** Renamed the two tests to include `text_commands`, making the plan gate execute success and malformed-failure coverage.
- **Files modified:** `crates/bindings_node/tests/text_commands.rs`
- **Verification:** `cargo test -p bindings_node text_commands -- --nocapture` ran 2 tests.
- **Committed in:** `14f6d53`

---

**Total deviations:** 3 auto-fixed (2 blocking, 1 missing critical verification coverage)
**Impact on plan:** All fixes were required to make the planned gates meaningful and passing. No feature scope was added beyond subtitle import and binding exposure.

## Known Stubs

None.

## Threat Flags

None.

## Issues Encountered

- `gsd-tools` was not on PATH in this shell; the available CLI was invoked via `node /Users/zhiwen/.codex/get-shit-done/bin/gsd-tools.cjs` for state follow-up.
- `cargo fmt` reformatted two unrelated draft-model files during verification; those formatting-only changes were restored before task commits.

## Verification

- `cargo test -p draft_commands subtitle -- --nocapture` - passed, 4 subtitle tests executed.
- `cargo test -p bindings_node text_commands -- --nocapture` - passed, 2 binding tests executed.
- `cargo test -p draft_model schema_exports -- --nocapture` - passed after regenerating contracts.
- `git diff --exit-code schemas apps/desktop-electron/src/generated` - passed after generated artifacts were committed.

## User Setup Required

None.

## Next Phase Readiness

Phase 09 UI work can build an import-subtitle control against the generated `importSubtitleSrt` envelope while keeping SRT parsing, timing, text segment creation, and undo/redo in Rust.

## Self-Check: PASSED

- Found `.planning/phases/09-complete-text-and-subtitle-system/09-03-SUMMARY.md`.
- Found key implementation files `crates/draft_commands/src/text.rs`, `crates/bindings_node/tests/text_commands.rs`, `schemas/command.schema.json`, and `apps/desktop-electron/src/generated/CommandEnvelope.ts`.
- Found task commits `07777ba`, `3467259`, `59a4061`, and `14f6d53`.
- No tracked file deletions were introduced.

---
*Phase: 09-complete-text-and-subtitle-system*
*Completed: 2026-06-18*
