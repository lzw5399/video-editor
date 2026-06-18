---
phase: 10-typed-keyframe-and-animation-system
plan: 01
subsystem: draft-model
tags: [rust, schema, keyframe, animation, generated-contracts]
requires:
  - phase: 08-segment-transform-and-visual-compositing
    provides: Static segment visual transform and opacity fields used as animated base values
  - phase: 09-complete-text-and-subtitle-system
    provides: Static text style and layout fields used as animated base values
provides:
  - Typed segment keyframe schema with property, value, interpolation, and easing contracts
  - Keyframe validation for segment-relative timing, duplicate property/time pairs, value kinds, and value ranges
  - Regenerated draft and command schemas plus desktop Draft.ts contract for typed keyframes
affects: [draft-model, generated-contracts, phase-10-commands, phase-10-engine]
tech-stack:
  added: []
  patterns: [typed-animation-contracts, segment-relative-keyframes, tdd-red-green]
key-files:
  created:
    - .planning/phases/10-typed-keyframe-and-animation-system/10-01-SUMMARY.md
  modified:
    - crates/draft_model/src/timeline.rs
    - crates/draft_model/src/validation.rs
    - crates/draft_model/src/lib.rs
    - crates/draft_model/tests/draft_schema.rs
    - crates/draft_model/tests/schema_exports.rs
    - schemas/draft.schema.json
    - schemas/command.schema.json
    - apps/desktop-electron/src/generated/Draft.ts
    - fixtures/draft/positive/materials-round-trip/project.json
key-decisions:
  - "Keyframes are segment-attached typed semantic data with segment-relative integer-microsecond time offsets."
  - "Static Phase 08/09 visual, text, and volume fields remain base values that keyframes can override during later frame-time evaluation."
  - "Sticker and filter parameter animation boundaries are explicit typed enum cases but remain unsupported/deferred until later phases add full semantics."
patterns-established:
  - "Keyframe property/value compatibility is enforced in Rust validation while generated schemas expose typed enum/value contracts and color format constraints."
  - "Generated contract drift remains checked by schema export tests plus git diff after committing regenerated artifacts."
requirements-completed: [ANIM-01, ANIM-02]
duration: 16 min
completed: 2026-06-18
---

# Phase 10 Plan 01: Typed Keyframe Schema Summary

**Typed Jianying-style keyframe storage with segment-relative timing, validated animated values, easing, interpolation, and regenerated contracts.**

## Performance

- **Duration:** 16 min
- **Started:** 2026-06-18T07:00:13Z
- **Completed:** 2026-06-18T07:18:26Z
- **Tasks:** 2
- **Files modified:** 9

## Accomplishments

- Replaced placeholder string keyframes with typed `KeyframeProperty`, `KeyframeValue`, `KeyframeInterpolation`, and `KeyframeEasing` contracts.
- Added validation for keyframe `at` values within segment target duration, duplicate property/time pairs, value-kind compatibility, visual/text/audio numeric ranges, and `#RRGGBB` text colors.
- Regenerated `schemas/draft.schema.json`, `schemas/command.schema.json`, and `apps/desktop-electron/src/generated/Draft.ts` so desktop code receives Rust-owned typed keyframe contracts.
- Migrated the positive material round-trip fixture from string opacity keyframes to `visualOpacity` with a typed uint value.

## Task Commits

1. **Task 10-01-01: Add typed keyframe schema and validation** - `b65330d` (test RED), `9fc9be3` (feat GREEN)
2. **Task 10-01-02: Export keyframe schemas and generated TypeScript contracts** - `9fc9be3` (feat GREEN, generated contracts included)

## Files Created/Modified

- `crates/draft_model/src/timeline.rs` - Adds typed keyframe property/value/interpolation/easing model.
- `crates/draft_model/src/validation.rs` - Adds keyframe validation and `InvalidKeyframe` diagnostics.
- `crates/draft_model/src/lib.rs` - Re-exports the new keyframe contract types.
- `crates/draft_model/tests/draft_schema.rs` - Covers typed serialization, invalid combinations, out-of-range timing, duplicate property/time pairs, and empty keyframes.
- `crates/draft_model/tests/schema_exports.rs` - Exports keyframe contracts and checks schema rejection for invalid keyframe examples.
- `schemas/draft.schema.json` - Regenerated draft schema with typed keyframe definitions.
- `schemas/command.schema.json` - Regenerated command schema with typed keyframe definitions.
- `apps/desktop-electron/src/generated/Draft.ts` - Regenerated desktop draft contract with typed keyframe exports.
- `fixtures/draft/positive/materials-round-trip/project.json` - Migrates the sample keyframe to typed `visualOpacity`.

## Decisions Made

- Keyframe time remains `Microseconds` and is relative to the owning segment head, matching the Phase 10 research direction and Jianying-style segment-attached keyframes.
- Keyframe values use an explicit tagged union (`int`, `uint`, `color`) instead of lossy strings.
- Property/value compatibility is enforced by Rust validation because the schema cannot express all property-specific value/range rules cleanly without duplicating domain logic.

## Deviations from Plan

None - plan executed exactly as written.

---

**Total deviations:** 0 auto-fixed.
**Impact on plan:** None.

## Issues Encountered

- `gsd-tools requirements.mark-complete` did not find v2 ANIM requirement rows because they are tracked as traceability table entries rather than checkbox items. Requirement completion remains phase-level and will be finalized by Phase 10 verification/complete.

## Verification

- `cargo test -p draft_model keyframe -- --nocapture` - passed
- `cargo test -p draft_model schema -- --nocapture` - passed
- `cargo test -p draft_model schema_exports -- --nocapture` - passed
- `cargo fmt --all --check` - passed
- `git diff --exit-code schemas apps/desktop-electron/src/generated` - passed after generated artifacts were committed

## User Setup Required

None.

## Next Phase Readiness

Phase 10 Plan 02 can add Rust-owned `setSegmentKeyframe` and `removeSegmentKeyframe` commands against the typed keyframe contract without changing the persisted schema again.

## Self-Check: PASSED

- Found `.planning/phases/10-typed-keyframe-and-animation-system/10-01-SUMMARY.md`.
- Found task commits `b65330d` and `9fc9be3`.
- Confirmed contract drift is clean after committing generated artifacts.
- No tracked file deletions were introduced.

---
*Phase: 10-typed-keyframe-and-animation-system*
*Completed: 2026-06-18*
