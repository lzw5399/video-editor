---
phase: 02-draft-and-material-system
reviewed: 2026-06-17T04:34:35Z
depth: standard
files_reviewed: 44
files_reviewed_list:
  - apps/desktop-electron/src/generated/CommandEnvelope.ts
  - apps/desktop-electron/src/generated/CommandResultEnvelope.ts
  - apps/desktop-electron/src/generated/Draft.ts
  - apps/desktop-electron/src/renderer/App.tsx
  - apps/desktop-electron/src/renderer/styles.css
  - apps/desktop-electron/tests/electron-smoke.spec.ts
  - crates/bindings_node/Cargo.toml
  - crates/bindings_node/src/lib.rs
  - crates/bindings_node/src/material_service.rs
  - crates/bindings_node/tests/binding_smoke.rs
  - crates/bindings_node/tests/material_service.rs
  - crates/draft_model/src/draft.rs
  - crates/draft_model/src/ids.rs
  - crates/draft_model/src/lib.rs
  - crates/draft_model/src/material.rs
  - crates/draft_model/src/time.rs
  - crates/draft_model/src/timeline.rs
  - crates/draft_model/src/validation.rs
  - crates/draft_model/tests/draft_fixtures.rs
  - crates/draft_model/tests/draft_schema.rs
  - crates/draft_model/tests/schema_exports.rs
  - crates/media_runtime/Cargo.toml
  - crates/media_runtime/src/lib.rs
  - crates/media_runtime/src/probe.rs
  - crates/media_runtime/tests/material_probe.rs
  - crates/project_store/Cargo.toml
  - crates/project_store/src/bundle.rs
  - crates/project_store/src/error.rs
  - crates/project_store/src/lib.rs
  - crates/project_store/src/paths.rs
  - crates/project_store/tests/project_bundle.rs
  - crates/testkit/src/lib.rs
  - crates/testkit/tests/material_fixtures.rs
  - docs/runtime-boundaries.md
  - fixtures/draft/negative/derived-artifact-in-project-json/project.json
  - fixtures/draft/negative/invalid-schema-version/project.json
  - fixtures/draft/negative/invalid-unknown-field/project.json
  - fixtures/draft/positive/materials-round-trip/project.json
  - fixtures/draft/positive/minimal-draft/project.json
  - fixtures/draft/positive/missing-material/project.json
  - justfile
  - package.json
  - schemas/command.schema.json
  - schemas/draft.schema.json
findings:
  critical: 0
  warning: 2
  info: 0
  total: 2
status: issues_found
---

# Phase 02: Code Review Report

**Reviewed:** 2026-06-17T04:34:35Z
**Depth:** standard
**Files Reviewed:** 44
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

Reviewed the Phase 2 generated TypeScript and JSON Schema contracts, renderer smoke UI, Electron smoke tests, Node binding commands, material service, draft model/schema/validation, media probe runtime, project-store persistence/path helpers, testkit media helpers, fixtures, package scripts, and runtime boundary documentation.

The four prior findings are closed in the reviewed code: `save_project_bundle` validates material URIs before serializing/writing, project replacement uses a Windows `MoveFileExW(..., MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)` path, `media_runtime` duration parsing rejects malformed fractions and overflow with checked arithmetic, and `media_runtime` frame-rate normalization rejects zero numerators. Targeted verification passed:

- `cargo test -p project_store save_project_bundle -- --nocapture`
- `cargo test -p media_runtime material_probe_rejects_malformed_json_and_invalid_frame_rates -- --nocapture`
- `cargo test -p draft_model schema_exports_generated_contract_artifacts_from_rust -- --nocapture`

Remaining issues are contract/test-harness robustness gaps, not direct project-data loss in the current save/open path.

## Warnings

### WR-01: Testkit still accepts malformed and overflowing ffprobe durations

**Classification:** WARNING
**File:** `crates/testkit/src/lib.rs:626`
**Issue:** The production probe parser was hardened, but the public testkit parser still uses `saturating_mul`/`saturating_add` and filters non-digits out of the fractional component. A value like `1.-5` is interpreted as `1_500_000` microseconds, and an overflowing whole-second value clamps instead of failing. Because `probe_media_metadata` is a reusable smoke helper, this can let malformed ffprobe metadata pass testkit gates and mask regressions in runtime normalization.
**Fix:**
```rust
let whole_micros = whole
    .parse::<u64>()
    .map_err(|error| SmokeError::new(format!("invalid duration seconds `{value}`: {error}")))?
    .checked_mul(1_000_000)
    .ok_or_else(|| SmokeError::new(format!("duration is too large `{value}`")))?;
if !fractional.bytes().all(|byte| byte.is_ascii_digit()) {
    return Err(SmokeError::new(format!("invalid duration fraction `{value}`")));
}
let fraction_micros = fraction.parse::<u64>().map_err(|error| {
    SmokeError::new(format!("invalid duration fraction `{value}`: {error}"))
})?;
whole_micros
    .checked_add(fraction_micros)
    .ok_or_else(|| SmokeError::new(format!("duration is too large `{value}`")))
```

### WR-02: JSON Schema contracts allow frame rates the Rust model rejects

**Classification:** WARNING
**File:** `schemas/draft.schema.json:249`
**Issue:** The committed draft schema and command schema define `RationalFrameRate.numerator` and `denominator` with `"minimum": 0`, while `validate_draft` rejects either field when it is zero. A `.veproj/project.json` containing `{"frameRate":{"numerator":0,"denominator":1}}` therefore passes the published JSON Schema but fails Rust migration/validation. That makes the contract artifacts unreliable for clients and fixture validation.
**Fix:** Patch the schema export step to rewrite `RationalFrameRate.numerator` and `RationalFrameRate.denominator` to `"minimum": 1` in both schema outputs, update `schemas/draft.schema.json` and `schemas/command.schema.json`, and add a negative schema fixture or unit assertion proving zero numerator and zero denominator fail schema validation.

---

_Reviewed: 2026-06-17T04:34:35Z_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_
