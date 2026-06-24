---
phase: 17-template-import-core-and-kaipai-offline-adapter-foundation
plan: "02"
subsystem: template-import
tags: [draft-import, resource-localizer, sha256, veproj-resources, source-guards]

requires:
  - phase: 17-01
    provides: [provider-neutral AdaptationReport contract, Phase 17 source guards, draft_import crate]
provides:
  - Provider-neutral template resource localizer for `.veproj/resources/template-import/...`
  - SHA-256 validation and classified diagnostics for unsafe, missing, remote, duplicate, and mismatched resources
  - Localized resource manifest entries with bundle-relative refs and resource-index-compatible metadata
affects: [adapter-kaipai, draft-import-plan, project-session-import, artifact-resource-index, phase17-validation]

tech-stack:
  added: [sha2]
  patterns:
    - Provider-neutral resource localization before mapper/session draft mutation
    - Bundle-relative `.veproj/resources/template-import/<import-id>/...` resource refs
    - Localizer diagnostics emitted as AdaptationReportItem values

key-files:
  created:
    - crates/draft_import/src/resource_localizer.rs
    - crates/draft_import/tests/resource_localizer.rs
    - fixtures/kaipai/resources/README.md
  modified:
    - Cargo.lock
    - crates/draft_import/Cargo.toml
    - crates/draft_import/src/lib.rs
    - package.json

key-decisions:
  - "Resource localization returns provider-neutral diagnostics and never preserves remote URLs as runtime refs."
  - "Localized resources use deterministic bundle-relative refs under resources/template-import/<import-id>/ with resource-index-compatible metadata."
  - "SHA-256 validation uses the approved sha2 crate instead of copied hand-written hash code."

patterns-established:
  - "Unsafe resource inputs are represented as LocalizedResourceStatus plus AdaptationReportItem diagnostics instead of panics."
  - "Phase 17 aggregate Rust tests now include resource_localizer coverage."

requirements-completed: [COMP-01, COMP-02, NO-FALLBACK-01]

duration: 9 min
completed: 2026-06-24
status: complete
---

# Phase 17 Plan 02: Template Import Core And Kaipai Offline Adapter Foundation Summary

**Safe offline template resource localization into `.veproj/resources` with SHA-256 validation and provider-neutral diagnostics.**

## Performance

- **Duration:** 9 min
- **Started:** 2026-06-24T07:37:44Z
- **Completed:** 2026-06-24T07:46:20Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments

- Added `resource_localizer` to `draft_import` with public `localize_template_resources`, request/result, manifest, resource status, resource kind, and resource-index ref types.
- Localized resources into deterministic bundle-relative `resources/template-import/<import-id>/...` refs with source/destination canonicalization and symlink escape protection.
- Added focused tests for copy success, traversal, remote URLs, missing resources, SHA-256 mismatch, duplicate destinations, source/destination symlink escapes, and fixture secret scanning.

## Task Commits

1. **Task 1: Add localizer safety tests before implementation** - `f93b230` (test)
2. **Task 2: Implement bundle-local resource localization** - `35d97b5` (feat)

## Files Created/Modified

- `crates/draft_import/src/resource_localizer.rs` - Provider-neutral resource localization, path safety, SHA-256 validation, manifest output, and report diagnostics.
- `crates/draft_import/tests/resource_localizer.rs` - RED/GREEN safety tests for resource localization and fixture sanitization.
- `fixtures/kaipai/resources/README.md` - Sanitized resource fixture directory note.
- `crates/draft_import/src/lib.rs` - Public localizer module and re-exports for adapter/session consumers.
- `crates/draft_import/Cargo.toml`, `Cargo.lock` - Added approved `sha2` dependency.
- `package.json` - Added resource localizer tests to `test:phase17-rust`.

## Decisions Made

- Kept the localizer provider-neutral: external template evidence appears only in diagnostics/provenance, not canonical runtime refs.
- Returned remote resource URLs as `RemoteRenderUrl` diagnostics with no `projectRelativeRef`.
- Used localizer-owned resource-index-compatible metadata instead of adding provider-specific resource semantics to core crates.

## Verification

All verification passed:

- `cargo test -p draft_import resource_localizer -- --nocapture`
- `pnpm run test:phase17-source-guards`
- `pnpm run test:phase17`

`pnpm` emitted the existing Node engine warning (`wanted 24.12.0`, current `24.15.0`) but all commands exited successfully.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Fixed SHA-256 formatting for `sha2` 0.11**
- **Found during:** Task 2
- **Issue:** `format!("{:x}", Sha256::digest(...))` does not compile with `sha2` 0.11 because the digest array does not implement `LowerHex`.
- **Fix:** Converted digest bytes to lowercase hex explicitly.
- **Files modified:** `crates/draft_import/src/resource_localizer.rs`
- **Verification:** `cargo test -p draft_import resource_localizer -- --nocapture`
- **Committed in:** `35d97b5`

**2. [Rule 2 - Missing Critical] Added localizer tests to the Phase 17 aggregate Rust gate**
- **Found during:** Task 2
- **Issue:** `test:phase17-rust` still covered only Plan 17-01 report/schema tests, so the new localizer safety contract would be skipped by the phase aggregate gate.
- **Fix:** Added `cargo test -p draft_import resource_localizer -- --nocapture` to `test:phase17-rust`.
- **Files modified:** `package.json`
- **Verification:** `pnpm run test:phase17`
- **Committed in:** `35d97b5`

**Total deviations:** 2 auto-fixed (1 bug fix, 1 missing critical gate).
**Impact on plan:** Both fixes were required for the planned safety and validation behavior; no product scope was expanded.

## Issues Encountered

- `pnpm` reported the existing Node engine mismatch warning (`wanted 24.12.0`, current `24.15.0`). It did not block verification.

## User Setup Required

None - no external service configuration required.

## Known Stubs

None. Stub scan hit `placeholder` in the fixture README policy text; it is documentation wording, not runtime stub data.

## Threat Flags

None. The resource localization trust boundary and remote/secret/path cases are covered by the plan threat model and tests.

## TDD Gate Compliance

- RED commit present before GREEN: `f93b230` -> `35d97b5`
- RED failure was expected: unresolved imports for the not-yet-implemented localizer API.
- GREEN verification passed with 6 focused localizer tests.

## Next Phase Readiness

Ready for Plan 17-03 to consume localized bundle-relative resources from provider-neutral import planning without depending on remote template URLs.

## Self-Check: PASSED

- Files found: `Cargo.lock`, `crates/draft_import/Cargo.toml`, `crates/draft_import/src/lib.rs`, `crates/draft_import/src/resource_localizer.rs`, `crates/draft_import/tests/resource_localizer.rs`, `fixtures/kaipai/resources/README.md`, `package.json`.
- Commits found in git history: `f93b230`, `35d97b5`.
- Plan verification commands passed after both task commits.

---
*Phase: 17-template-import-core-and-kaipai-offline-adapter-foundation*
*Completed: 2026-06-24*
