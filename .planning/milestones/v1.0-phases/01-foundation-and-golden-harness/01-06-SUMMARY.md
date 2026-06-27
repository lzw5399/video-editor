---
phase: 01-foundation-and-golden-harness
plan: 06
subsystem: contracts
tags: [rust, schemars, ts-rs, jsonschema, fixtures, generated-contracts]

requires:
  - phase: 01-02
    provides: Rust-owned command/result envelope contracts in draft_model
provides:
  - Generated command JSON Schema from Rust contract types
  - Generated TypeScript command envelope and result envelope contracts for Electron
  - Positive and negative command fixtures validated through serde and JSON Schema
affects: [phase-1-foundation, electron-shell, command-contracts, schema-fixtures]

tech-stack:
  added: [jsonschema-0.46.5]
  patterns:
    - Rust tests regenerate committed schema and TypeScript contract artifacts
    - Fixture validation classifies every fixtures/draft JSON file as positive or negative
    - Unknown command payload fields are rejected by both serde and generated JSON Schema

key-files:
  created:
    - schemas/command.schema.json
    - apps/desktop-electron/src/generated/CommandEnvelope.ts
    - apps/desktop-electron/src/generated/CommandResultEnvelope.ts
    - fixtures/draft/minimal-command.json
    - fixtures/draft/invalid-unknown-field.json
  modified:
    - Cargo.lock
    - crates/draft_model/Cargo.toml
    - crates/draft_model/src/lib.rs
    - crates/draft_model/tests/contract.rs
    - crates/draft_model/tests/schema_exports.rs

key-decisions:
  - "Generated command schema and TypeScript contracts from Rust tests instead of hand-written files."
  - "Made required TypeScript artifacts self-contained so Electron can consume the two planned generated files without uncommitted dependency files."
  - "Represented Phase 1 command payload variants as empty strict structs so payload-level unknown fields fail serde and schema validation."

patterns-established:
  - "Generated contract drift is checked by running cargo test -p draft_model schema and then git diff --exit-code schemas apps/desktop-electron/src/generated."
  - "fixtures/draft JSON files must be explicitly listed as positive or negative in schema_exports.rs."
  - "Negative command fixtures must fail both Rust deserialization and JSON Schema validation."

requirements-completed: [TEST-01, FOUND-04, FOUND-02]

duration: 9 min
completed: 2026-06-17
---

# Phase 1 Plan 06: Generated Contracts And Command Fixture Validation Summary

**Rust-generated command schema and TypeScript contracts with serde plus JSON Schema validation for positive and unknown-field fixtures.**

## Performance

- **Duration:** 9 min
- **Started:** 2026-06-16T21:54:47Z
- **Completed:** 2026-06-16T22:03:33Z
- **Tasks:** 2
- **Files modified:** 10

## Accomplishments

- Added a Rust schema export test that regenerates `schemas/command.schema.json` and the planned Electron TypeScript contract files.
- Committed generated `CommandEnvelope.ts` and generic `CommandResultEnvelope.ts` artifacts derived from `ts-rs` declarations.
- Added `fixtures/draft/minimal-command.json` and `fixtures/draft/invalid-unknown-field.json`.
- Validated every `fixtures/draft/*.json` through explicit positive/negative paths using Rust serde and `jsonschema`.

## Task Commits

1. **Task 01-W2-03 RED: Contract artifact export test** - `3909f19` (test)
2. **Task 01-W2-03 GREEN: Rust-owned generated contracts** - `12ed4d4` (feat)
3. **Task 01-W2-04 RED: Command fixture validation test** - `0f3be62` (test)
4. **Task 01-W2-04 GREEN: Fixture validation and strict payloads** - `478d663` (feat)

## Files Created/Modified

- `schemas/command.schema.json` - Generated JSON Schema for the Rust command envelope.
- `apps/desktop-electron/src/generated/CommandEnvelope.ts` - Generated self-contained TypeScript command envelope contract.
- `apps/desktop-electron/src/generated/CommandResultEnvelope.ts` - Generated self-contained generic TypeScript result envelope contract.
- `fixtures/draft/minimal-command.json` - Positive Phase 1 command fixture.
- `fixtures/draft/invalid-unknown-field.json` - Negative unknown-field command fixture.
- `crates/draft_model/tests/schema_exports.rs` - Regenerates contracts, checks fixtures, and validates every draft JSON fixture is classified.
- `crates/draft_model/src/lib.rs` - Uses strict empty payload structs so payload-level unknown fields fail.
- `crates/draft_model/tests/contract.rs` - Updates payload variant matching for strict payload structs.
- `crates/draft_model/Cargo.toml` - Adds approved `jsonschema` dev dependency.
- `Cargo.lock` - Locks `jsonschema` dependency graph.

## Verification

- `cargo test -p draft_model schema -- --nocapture` - PASS, 2 schema/export fixture tests passed.
- `git diff --exit-code schemas apps/desktop-electron/src/generated` - PASS after regeneration from Rust.
- `cargo test -p draft_model contract -- --nocapture` - PASS, 3 contract tests passed.
- `cargo fmt --all --check` - PASS.

## TDD Gate Compliance

- **Task 01-W2-03 RED:** `3909f19` failed because generated schema and TypeScript files were missing.
- **Task 01-W2-03 GREEN:** `12ed4d4` added Rust-owned generation and committed generated artifacts.
- **Task 01-W2-04 RED:** `0f3be62` failed because `fixtures/draft` did not exist.
- **Task 01-W2-04 GREEN:** `478d663` added fixtures, strict payload rejection, and passing validation.
- **REFACTOR:** No separate refactor commit was needed; rustfmt cleanup was included before the GREEN task commit.

## Decisions Made

- Generated the two required TypeScript files as self-contained declarations from `ts-rs::TS::decl`, avoiding uncommitted generated dependency files.
- Used `jsonschema` 0.46.5 with default features disabled because the tests validate local generated schemas only.
- Made `CommandPayload` variants wrap strict empty payload structs to make payload-level unknown fields fail both serde and schema validation.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] Rejected payload-level unknown fields**
- **Found during:** Task 01-W2-04 (Validate positive and negative command fixtures)
- **Issue:** A negative fixture with an extra field inside `payload` still deserialized successfully with the original unit enum payload contract.
- **Fix:** Changed payload variants to strict empty payload structs and regenerated schema/TypeScript artifacts.
- **Files modified:** `crates/draft_model/src/lib.rs`, `crates/draft_model/tests/contract.rs`, `schemas/command.schema.json`, `apps/desktop-electron/src/generated/CommandEnvelope.ts`
- **Verification:** `cargo test -p draft_model schema -- --nocapture`
- **Committed in:** `478d663`

**2. [Rule 1 - Test Bug] Removed parallel schema file read race**
- **Found during:** Task 01-W2-04 (Validate positive and negative command fixtures)
- **Issue:** The fixture validation test could read `schemas/command.schema.json` while the export test rewrote it during parallel test execution.
- **Fix:** Compiled the generated schema in memory for fixture validation while leaving the export test responsible for writing committed artifacts.
- **Files modified:** `crates/draft_model/tests/schema_exports.rs`
- **Verification:** `cargo test -p draft_model schema -- --nocapture`
- **Committed in:** `478d663`

### Tooling Lookup Deviations

**1. Context7 CLI unavailable**
- **Found during:** Task 01-W2-03 and Task 01-W2-04 documentation lookup
- **Issue:** The required `ctx7` CLI fallback was not installed in the environment.
- **Fix:** Used approved versions from `01-RESEARCH.md`, inspected local crate source/API after dependency resolution, and validated behavior with Cargo tests.
- **Files modified:** None
- **Verification:** `cargo test -p draft_model schema -- --nocapture`

---

**Total deviations:** 2 auto-fixed (Rule 2: 1, Rule 1: 1), 1 tooling lookup fallback.
**Impact on plan:** The fixes strengthened the planned threat mitigations without expanding Phase 1 beyond generated command contracts and command fixtures.

## Issues Encountered

- `ts-rs` 12.0.1 does not expose the planned `export_to` method name; it provides `export`, `export_all`, and `export_to_string`. The generator uses `TS::decl` to write the exact required file paths deterministically.
- The first TypeScript generation attempt emitted imports for dependency files not listed in the plan. The final generator writes self-contained declarations into the two planned files.

## Known Stubs

None.

## Threat Flags

None. The generated-contract and fixture-validation trust boundaries were already covered by the plan threat model.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Ready for remaining Phase 1 plans to wire FFmpeg discovery, render smoke, Electron shell consumption of the generated contracts, and final `just`/CI gates.

## Self-Check: PASSED

- Key files exist on disk.
- Task commits `3909f19`, `12ed4d4`, `0f3be62`, and `478d663` exist in git history.
- Plan verification commands passed.
- Stub scan over touched code, generated contracts, schema, and fixtures found no placeholder/TODO/FIXME or hardcoded empty UI data patterns.

---
*Phase: 01-foundation-and-golden-harness*
*Completed: 2026-06-17*
