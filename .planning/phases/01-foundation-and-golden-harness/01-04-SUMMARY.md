---
phase: 01-foundation-and-golden-harness
plan: 04
subsystem: rust-bindings
tags: [rust, napi-rs, node-api, serde, contracts]

requires:
  - phase: 01-02
    provides: Rust-owned command/result envelope contracts in draft_model
provides:
  - napi-rs bindings_node crate configured as a Cargo workspace member
  - Node-API functions for ping, version, and execute_command
  - Rust-level binding smoke tests for standardized ok/error/events envelopes
affects: [phase-1-foundation, electron-shell, bindings-node, command-contracts]

tech-stack:
  added: [napi-3.9.2, napi-derive-3.5.6, napi-build-2.3.2]
  patterns:
    - Node-API functions return serde_json values serialized from draft_model envelopes
    - execute_command classifies unknown command names before returning structured errors
    - Binding smoke tests exercise the Rust boundary before Electron IPC is introduced

key-files:
  created:
    - crates/bindings_node/Cargo.toml
    - crates/bindings_node/build.rs
    - crates/bindings_node/src/lib.rs
    - crates/bindings_node/tests/binding_smoke.rs
  modified:
    - Cargo.toml
    - Cargo.lock

key-decisions:
  - "Kept the Node-API surface limited to ping, version, and execute_command."
  - "Returned binding data by serializing draft_model CommandResultEnvelope values instead of defining JavaScript-owned contracts."
  - "Accepted raw JSON at execute_command so unsupported command names can return UnsupportedCommand instead of deserialization-only InvalidPayload."

patterns-established:
  - "bindings_node depends on draft_model for all command and result contracts."
  - "Rust binding smoke tests compare direct functions against execute_command for Phase 1 command parity."
  - "Unsupported native-boundary commands return ok: false, a structured error, and empty events."

requirements-completed: [FOUND-02]

duration: 5 min
completed: 2026-06-17
---

# Phase 1 Plan 04: Node-API Binding Crate Summary

**napi-rs binding crate exposing only ping, version, and execute_command over Rust-owned ok/error/events envelopes.**

## Performance

- **Duration:** 5 min
- **Started:** 2026-06-16T21:44:19Z
- **Completed:** 2026-06-16T21:49:42Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments

- Added `bindings_node` as a real workspace crate with approved napi-rs runtime, derive, and build dependencies.
- Implemented `ping`, `version`, and `execute_command` as the only Node-API exports in this plan.
- Added binding smoke tests proving direct functions and command execution return standardized Rust-owned envelopes.
- Ensured non-Phase-1 command names return `UnsupportedCommand` with deterministic empty events.

## Task Commits

1. **Task 01-W2-01: Configure napi-rs binding crate** - `3529344` (chore)
2. **Task 01-W2-02 RED: Add failing binding envelope smoke tests** - `190ad6b` (test)
3. **Task 01-W2-02 GREEN: Expose binding command envelopes** - `73339cc` (feat)
4. **Task 01-W2-02 REFACTOR: Format binding smoke tests** - `a1acd9d` (refactor)

## Files Created/Modified

- `Cargo.toml` - Added `crates/bindings_node` to the Rust workspace and cleared completed planned-member metadata.
- `Cargo.lock` - Locked approved napi-rs dependency resolution for the binding crate.
- `crates/bindings_node/Cargo.toml` - Configures the napi-rs crate, Rust testable `rlib`, and local `draft_model` dependency.
- `crates/bindings_node/build.rs` - Runs standard `napi_build::setup()`.
- `crates/bindings_node/src/lib.rs` - Exposes `ping`, `version`, and `execute_command` using `draft_model` envelopes and structured errors.
- `crates/bindings_node/tests/binding_smoke.rs` - Tests direct envelopes, command parity, and unsupported command errors.

## Verification

- `cargo check -p bindings_node --locked` - PASS
- `cargo test -p bindings_node -- --nocapture` - PASS, 4 binding smoke tests passed
- `grep -R "Split\\|Trim\\|Move\\|Delete\\|ImportMaterial\\|Export" crates/bindings_node/src crates/bindings_node/tests && exit 1 || true` - PASS
- `cargo fmt --all --check` - PASS

## TDD Gate Compliance

- **RED:** `190ad6b` added binding smoke tests; they failed because `ping`, `version`, and `execute_command` were missing from the crate root.
- **GREEN:** `73339cc` implemented the napi-rs functions and structured envelope handling; the targeted binding tests passed.
- **REFACTOR:** `a1acd9d` applied rustfmt cleanup with tests still passing.

## Decisions Made

- Kept the exported native surface narrow: `ping`, `version`, and `execute_command`.
- Used `serde_json::Value` at the napi boundary while serializing all responses from `draft_model::CommandResultEnvelope<T>`.
- Checked raw command names before full typed deserialization so unknown command names produce `UnsupportedCommand`, while malformed Phase 1 payloads still produce `InvalidPayload`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Updated root workspace membership and lockfile**
- **Found during:** Task 01-W2-01 (Configure napi-rs binding crate)
- **Issue:** The task file list did not include root `Cargo.toml` or `Cargo.lock`, but `cargo check -p bindings_node --locked` cannot run until the crate is a workspace member and the lockfile contains the approved napi-rs dependencies.
- **Fix:** Added `crates/bindings_node` to workspace members, removed it from completed planned-member metadata, refreshed `Cargo.lock`, and reran the locked check.
- **Files modified:** `Cargo.toml`, `Cargo.lock`
- **Verification:** `cargo check -p bindings_node --locked`
- **Committed in:** `3529344`

### Tooling Lookup Deviations

**1. Context7 CLI unavailable**
- **Found during:** Task 01-W2-01 dependency verification
- **Issue:** Required documentation lookup fallback command reported `ctx7 not found`.
- **Fix:** Used the already-approved versions from `01-RESEARCH.md` and verified local crate metadata with `cargo info napi@3.9.2`, `cargo info napi-derive@3.5.6`, and `cargo info napi-build@2.3.2`.
- **Files modified:** None
- **Verification:** `cargo check -p bindings_node --locked`

---

**Total deviations:** 1 auto-fixed (Rule 3: 1), 1 tooling lookup fallback.
**Impact on plan:** Required for executable verification and did not expand the binding scope beyond Phase 1.

## Issues Encountered

- The first locked binding check failed because `Cargo.lock` needed the new approved dependencies. Running the non-locked check once refreshed the lockfile, and the locked gate passed afterward.
- `cargo fmt --all --check` caught formatting drift in the RED test file; rustfmt cleanup was committed separately as the TDD refactor step.
- Context7/`ctx7` was unavailable, so crate version details were verified through Cargo metadata and the phase research audit.

## Known Stubs

None. The `planned-members = []` workspace metadata is intentional because this plan converted the final planned Rust crate in that list into a real workspace member.

## Threat Flags

None. The new JavaScript-to-Node-API addon boundary and command-envelope handling are covered by the plan threat model (`T-01-01`, `T-01-02`, `T-01-03`, `T-01-SC`).

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Ready for Plan 01-05 to connect runtime discovery failures into the command-envelope path without adding real editing commands or FFmpeg execution behavior to the binding surface.

## Self-Check: PASSED

- Key files exist on disk.
- Task commits `3529344`, `190ad6b`, `73339cc`, and `a1acd9d` exist in git history.
- Plan verification commands passed.
- Stub scan over touched files found no blocking placeholders, TODO/FIXME markers, or UI-facing empty data stubs.

---
*Phase: 01-foundation-and-golden-harness*
*Completed: 2026-06-17*
