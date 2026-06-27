---
phase: 09-complete-text-and-subtitle-system
plan: 01
subsystem: draft-model
tags: [rust, schema, text, subtitle, generated-contracts]
requires:
  - phase: 08-segment-transform-and-visual-compositing
    provides: Segment.visual ownership and transform/compositing foundation for text segment placement
provides:
  - Complete defaulted TextSegment schema for text and subtitle sources
  - Text font, box, layout region, wrapping, line height, letter spacing, bubble, and effect refs
  - Text validation for required content, colors, ranges, font refs, and unsupported external refs
  - Regenerated draft and command schemas plus desktop Draft.ts contract
affects: [draft-model, generated-contracts, phase-09-text-engine, phase-09-ui]
tech-stack:
  added: []
  patterns: [defaulted-schema-evolution, unsupported-external-ref, tdd-red-green]
key-files:
  created:
    - .planning/phases/09-complete-text-and-subtitle-system/09-01-SUMMARY.md
  modified:
    - crates/draft_model/src/timeline.rs
    - crates/draft_model/src/validation.rs
    - crates/draft_model/src/lib.rs
    - crates/draft_model/tests/draft_schema.rs
    - crates/draft_model/tests/schema_exports.rs
    - schemas/draft.schema.json
    - schemas/command.schema.json
    - apps/desktop-electron/src/generated/Draft.ts
key-decisions:
  - "Text and subtitle now share a defaulted TextSegment schema with TextSegmentSource distinguishing text from subtitle usage."
  - "Proprietary text bubble and 花字 references are represented as unsupported external refs with camelCase externalRef contract fields."
patterns-established:
  - "New persisted text fields use defaulted serde fields so existing MVP text segments continue to deserialize."
  - "Generated contract tests must explicitly export new nested Draft.ts semantic types, not rely only on transitive references."
requirements-completed: [TEXT2-01, TEXT2-03]
duration: 12 min
completed: 2026-06-18
---

# Phase 09 Plan 01: Text And Subtitle Schema Summary

**Defaulted Jianying-style text/subtitle semantics with validation and regenerated Rust-owned schema contracts.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-06-18T03:31:27Z
- **Completed:** 2026-06-18T03:42:41Z
- **Tasks:** 2
- **Files modified:** 8

## Accomplishments

- Added `TextSegmentSource`, `TextFont`, `TextBox`, `TextLayoutRegion`, `TextWrapping`, `TextBubbleRef`, and `TextEffectRef` to the Rust draft model.
- Extended `TextStyle` with defaulted font, line height, and letter spacing while preserving old text segment deserialization.
- Added validation for text content, font family/ref, `#RRGGBB` colors, font size, line-height and letter-spacing ranges, text box/layout bounds, and unsupported bubble/effect refs.
- Regenerated `schemas/draft.schema.json`, `schemas/command.schema.json`, and `apps/desktop-electron/src/generated/Draft.ts`.

## Task Commits

1. **Task 09-01-01: Add complete text schema defaults and validation** - `959d781` (test RED), `f40da29` (feat GREEN)
2. **Task 09-01-02: Regenerate schema and command contracts** - `a2ef421` (test RED), `a044f15` (feat GREEN)

## Files Created/Modified

- `crates/draft_model/src/timeline.rs` - Adds complete text/subtitle schema types, defaults, and camelCase external refs.
- `crates/draft_model/src/validation.rs` - Adds text-specific validation and `InvalidTextSegment` diagnostics.
- `crates/draft_model/src/lib.rs` - Re-exports new text model types and validation constants.
- `crates/draft_model/tests/draft_schema.rs` - Covers backward-compatible defaults, valid complete subtitle semantics, and invalid text cases.
- `crates/draft_model/tests/schema_exports.rs` - Requires generated contracts for all Phase 09 text types.
- `schemas/draft.schema.json` - Regenerated draft schema with Phase 09 text definitions.
- `schemas/command.schema.json` - Regenerated command schema with Phase 09 text definitions in text command payloads.
- `apps/desktop-electron/src/generated/Draft.ts` - Regenerated desktop draft contract with Phase 09 text types.

## Decisions Made

- Text/subtitle classification is a `TextSegmentSource` enum on `Segment.text`, not a separate subtitle object or render path.
- Default text layout uses integer millis units: 800x200 text box and a 100/100/800/800 safe layout region.
- Unsupported text bubble and text effect refs are stored as explicit `unsupported` variants with optional `externalRef` values.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Contract Bug] Fixed unsupported text ref casing**
- **Found during:** Task 09-01-02 (Regenerate schema and command contracts)
- **Issue:** The initial generated TypeScript contract emitted enum payload fields as `external_ref`, which would violate the repo's camelCase JSON contract convention.
- **Fix:** Added explicit serde renames so `TextBubbleRef` and `TextEffectRef` serialize and generate as `externalRef`.
- **Files modified:** `crates/draft_model/src/timeline.rs`, `schemas/draft.schema.json`, `schemas/command.schema.json`, `apps/desktop-electron/src/generated/Draft.ts`
- **Verification:** `cargo test -p draft_model text -- --nocapture`; `cargo test -p draft_model schema_exports -- --nocapture`; `git diff --exit-code schemas apps/desktop-electron/src/generated`
- **Committed in:** `a044f15`

---

**Total deviations:** 1 auto-fixed (1 bug)
**Impact on plan:** The fix was necessary to keep generated contracts consistent with established camelCase IPC/schema conventions. No scope expansion.

## Known Stubs

None.

## Issues Encountered

- `gsd-tools` was not linked on PATH in this shell; the executable was available at `/Users/zhiwen/.codex/get-shit-done/bin/gsd-tools.cjs` for state follow-up commands.

## Verification

- `cargo test -p draft_model text -- --nocapture` - passed
- `cargo test -p draft_model schema_exports -- --nocapture` - passed
- `git diff --exit-code schemas apps/desktop-electron/src/generated` - passed after generated artifacts were committed
- `cargo check --workspace` - passed as an additional compile sanity check

## User Setup Required

None.

## Next Phase Readiness

Phase 09 Plan 02 can now propagate these static text/subtitle semantics into engine frame state, render graph intent, and compiler diagnostics without changing the persisted draft contract again.

## Self-Check: PASSED

- Found `.planning/phases/09-complete-text-and-subtitle-system/09-01-SUMMARY.md`.
- Found task commits `959d781`, `f40da29`, `a2ef421`, and `a044f15`.
- No tracked file deletions were introduced.

---
*Phase: 09-complete-text-and-subtitle-system*
*Completed: 2026-06-18*
