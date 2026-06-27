---
phase: 19-production-effects-retiming-and-transition-semantics
plan: "02"
subsystem: effects-rendering
tags: [rust, draft-model, render-graph, realtime-preview, ffmpeg-compiler, schema-contracts]

requires:
  - phase: 19-production-effects-retiming-and-transition-semantics/19-01
    provides: Phase 19 RED gates, guard scripts, and schema-push detection for production effects semantics.
provides:
  - Rust-owned typed contracts for first-party filters, transitions, retiming, masks, blends, and external effect references.
  - Phase 19 capability registry with explicit preview/export support states and report-only external reference classification.
  - Render graph projection helpers that carry typed capability support facts into realtime preview and compiler diagnostics.
  - Regenerated JSON schema and desktop TypeScript contracts derived from Rust.
affects: [phase-19, draft-model, render-graph, realtime-preview-runtime, ffmpeg-compiler, desktop-contracts]

tech-stack:
  added: []
  patterns:
    - Typed capability registry in draft_model with renderer-neutral projection in render_graph.
    - Registry-derived support diagnostics consumed by preview/export layers instead of string inference.

key-files:
  created:
    - crates/draft_model/src/effects.rs
    - crates/render_graph/src/effects.rs
  modified:
    - crates/draft_model/src/timeline.rs
    - crates/draft_model/src/lib.rs
    - crates/draft_model/src/validation.rs
    - crates/draft_model/tests/production_effects_contracts.rs
    - crates/draft_model/tests/schema_exports.rs
    - crates/render_graph/src/graph.rs
    - crates/render_graph/src/fingerprint.rs
    - crates/render_graph/src/incremental.rs
    - crates/realtime_preview_runtime/src/capabilities.rs
    - crates/ffmpeg_compiler/src/job.rs
    - schemas/draft.schema.json
    - schemas/command.schema.json
    - apps/desktop-electron/src/generated/Draft.ts

key-decisions:
  - "Rust draft_model owns Phase 19 effect, transition, retime, mask, blend, and external-reference semantics."
  - "External provider IDs are report-only compatibility references and cannot become first-party supported capability kinds."
  - "Render graph carries typed preview/export support facts so realtime preview and compiler layers consume registry decisions instead of inferring support from strings."
  - "Desktop TypeScript contracts remain generated from Rust schema exports; renderer code does not hand-author semantic unions."

patterns-established:
  - "First-party production effects use tagged typed enums, rational speed ratios, and integer microsecond values instead of string maps or floating persisted seconds."
  - "Capability projection maps draft_model::EffectCapabilityRegistry states into RenderIntentSupport and diagnostic reason strings."
  - "Unsupported and external effect paths fail closed through explicit diagnostics for preview/export."

requirements-completed: [PRODFX-02, PRODFX-03, PRODFX-04]

duration: 26 min
completed: 2026-06-25
status: complete
---

# Phase 19 Plan 02: Typed Production Effects Capability Registry Summary

**Rust-owned production effect contracts now define first-party support states and project those typed capabilities into render graph, realtime preview, compiler diagnostics, schema, and generated desktop TypeScript.**

## Performance

- **Duration:** 26 min
- **Started:** 2026-06-25T07:20:03Z
- **Completed:** 2026-06-25T07:45:45Z
- **Tasks:** 3
- **Files modified:** 22 implementation and contract files

## Accomplishments

- Added `draft_model::effects` with typed filters, transitions, segment retiming, masks, blends, capability support states, and report-only external references.
- Replaced legacy string-only production effect fields with typed timeline contracts and validation for invalid persisted effect/retime data.
- Exported every public Phase 19 contract through the Rust schema export path and regenerated checked-in JSON schema and desktop TypeScript.
- Added render graph capability projection helpers and carried typed preview/export decisions into graph intents, fingerprints, dirty domains, realtime preview diagnostics, and FFmpeg compiler diagnostics.

## Task Commits

Each task was committed atomically:

1. **Task 1: Replace legacy string forms with typed capability contracts** - `e82d7a4` (`feat`)
2. **Task 2: Export schema and TypeScript capability contracts** - `5579578` (`feat`)
3. **Task 3: Project capability states into graph preview and compiler layers** - `5899d2e` (`feat`)

## Files Created/Modified

- `crates/draft_model/src/effects.rs` - Phase 19 capability registry, typed first-party effect contracts, support states, and external-reference types.
- `crates/draft_model/src/timeline.rs` - Timeline segment fields now use typed filters, transitions, retiming, masks, and blend modes.
- `crates/draft_model/src/validation.rs` - Validation rejects invalid typed retime ratios and external reference data.
- `crates/draft_model/tests/production_effects_contracts.rs` - Green tests for typed support states and external-reference classification.
- `crates/draft_model/tests/schema_exports.rs` - Schema export coverage for the Phase 19 public contract surface.
- `schemas/draft.schema.json`, `schemas/command.schema.json`, `apps/desktop-electron/src/generated/Draft.ts` - Regenerated contracts derived from Rust.
- `crates/render_graph/src/effects.rs` - Renderer-neutral projection helpers from draft capability states to render graph support facts.
- `crates/render_graph/src/graph.rs` - Render intents now carry typed filter, transition, retime, mask, and blend capability decisions.
- `crates/render_graph/src/fingerprint.rs`, `crates/render_graph/src/incremental.rs` - Fingerprints and dirty domains include production effect semantics.
- `crates/realtime_preview_runtime/src/capabilities.rs` - Preview diagnostics consume graph capability facts for Phase 19 categories.
- `crates/ffmpeg_compiler/src/job.rs` - Compiler job diagnostics report non-supported typed filter and transition export support.
- `crates/engine_core/src/normalize.rs`, `crates/artifact_store/src/resource_index.rs` - Downstream normalization and resource indexing consume typed effect references.

## Decisions Made

- Rust remains the semantic authority for Phase 19 effect capability support; UI and generated TypeScript expose shapes only.
- External/proprietary provider IDs are modeled as explicit external references and report-only diagnostics, not internal supported render semantics.
- Realtime preview and export diagnostics are driven by registry support states so unsupported and degraded paths cannot satisfy product success implicitly.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Added validation coverage for typed retiming and external effect references**
- **Found during:** Task 1
- **Issue:** The plan introduced typed persisted retime/effect contracts, but invalid speed ratios and malformed external references needed validation so `.veproj/project.json` could not accept broken semantics.
- **Fix:** Added validation paths and schema fixture updates for invalid speed numerator/denominator and external reference data.
- **Files modified:** `crates/draft_model/src/validation.rs`, `crates/draft_model/tests/draft_schema.rs`
- **Verification:** `cargo test -p draft_model production_effects_contracts -- --nocapture`
- **Committed in:** `e82d7a4`

**2. [Rule 3 - Blocking] Regenerated command schema drift caused by typed Draft and Segment shapes**
- **Found during:** Task 2
- **Issue:** Updating the draft schema also changed command schema definitions that embed `Draft` and `Segment` shapes; leaving `schemas/command.schema.json` stale would fail contract drift checks.
- **Fix:** Included the generated command schema update with the Rust schema and TypeScript exports.
- **Files modified:** `schemas/command.schema.json`
- **Verification:** `cargo test -p draft_model schema_exports -- --nocapture` and `pnpm run test:contracts`
- **Committed in:** `5579578`

**3. [Rule 3 - Blocking] Updated downstream typed-contract call sites outside the narrow Task 3 file list**
- **Found during:** Task 3
- **Issue:** Engine normalization, artifact resource indexing, and existing graph/preview test fixtures still expected legacy string filter/transition fields or video layers without retime intent.
- **Fix:** Routed those call sites through typed capability IDs, copied segment retiming into normalized segments, and updated test fixtures to construct typed filters, transitions, masks, blends, and retime intents.
- **Files modified:** `crates/engine_core/src/normalize.rs`, `crates/artifact_store/src/resource_index.rs`, `crates/render_graph/tests/render_graph_snapshots.rs`, `crates/realtime_preview_runtime/tests/capability_matrix.rs`, `crates/realtime_preview_runtime/tests/gpu_subset.rs`
- **Verification:** `cargo test -p render_graph production_effects -- --nocapture` and `cargo test -p realtime_preview_runtime capability_matrix -- --nocapture`
- **Committed in:** `5899d2e`

---

**Total deviations:** 3 auto-fixed (1 missing critical, 2 blocking)
**Impact on plan:** All deviations were required to keep the new typed contracts valid, generated, and consumable by existing runtime layers. No package installs, dependency upgrades, or architecture changes were introduced.

## Issues Encountered

- `pnpm run test:contracts` warned that local Node is `v24.15.0` while package metadata asks for `24.12.0`; the contract drift check passed.
- Rust tests emitted an existing macOS AVFoundation deprecation warning in `media_runtime_desktop`; it is outside this plan's scope and did not affect verification.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None.

## Verification

- `cargo test -p draft_model production_effects_contracts -- --nocapture`
- `cargo test -p draft_model schema_exports -- --nocapture`
- `cargo test -p render_graph production_effects -- --nocapture`
- `cargo test -p realtime_preview_runtime capability_matrix -- --nocapture`
- `pnpm run test:contracts`

## Next Phase Readiness

Phase 19 can now build concrete retime, transition, effect, mask, and blend behavior on top of Rust-owned typed contracts. Runtime consumers receive explicit supported/degraded/unsupported capability facts instead of inferring support from renderer strings.

## Self-Check: PASSED

- Summary file exists at `.planning/phases/19-production-effects-retiming-and-transition-semantics/19-02-SUMMARY.md`.
- Created contract files exist: `crates/draft_model/src/effects.rs` and `crates/render_graph/src/effects.rs`.
- Generated contract artifacts exist: `schemas/draft.schema.json`, `schemas/command.schema.json`, and `apps/desktop-electron/src/generated/Draft.ts`.
- Task commits exist in git: `e82d7a4`, `5579578`, and `5899d2e`.

---
*Phase: 19-production-effects-retiming-and-transition-semantics*
*Completed: 2026-06-25*
