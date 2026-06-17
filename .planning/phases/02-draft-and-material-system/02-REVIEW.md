---
phase: 02-draft-and-material-system
reviewed: 2026-06-17T04:24:25Z
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
  critical: 2
  warning: 2
  info: 0
  total: 4
status: issues_found
---

# Phase 02: Code Review Report

**Reviewed:** 2026-06-17T04:24:25Z
**Depth:** standard
**Files Reviewed:** 44
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

Reviewed the Phase 2 draft/material model, project-store persistence and path helpers, media probe boundary, Node binding commands, generated TypeScript/JSON Schema contracts, renderer smoke surface, fixtures, tests, `justfile`, package scripts, and runtime-boundary docs.

The prior findings for in-place writes, Windows drive-path classification, Microseconds JSON/TS mismatch, and draft schema version const semantics are substantially addressed: writes now use temp-file replacement, Windows drive paths are checked before URI schemes, `Microseconds` is generated as `number`, and both committed schemas constrain the draft schema version to `const: 1`. The remaining blockers are still in persistence correctness: the store can write material URIs that it later refuses to open, and the current temp-file rename does not replace existing `project.json` on Windows.

## Critical Issues

### CR-01: Save accepts invalid material URIs that make the project unopenable

**Classification:** BLOCKER
**File:** `crates/project_store/src/bundle.rs:38`

**Issue:** `save_project_bundle` validates draft semantics but never runs the project-store material URI classifier before serializing and writing `.veproj/project.json`. That means callers can save a draft containing a traversal URI such as `../outside.mp4`; the save succeeds, but `open_project_bundle` later calls `collect_warnings` and fails through `classify_material_uri`. The persistence boundary can therefore create a canonical project file it cannot reopen.

**Fix:**
```rust
validate_draft(draft).map_err(|source| semantic_error(&project_json_path, source))?;
for material in &draft.materials {
    classify_material_uri(bundle_path, &material.uri)?;
}
```
Add a project-store regression test that `save_project_bundle` rejects `../...` material URIs and preserves the previously readable `project.json`.

### CR-02: Existing project saves fail on Windows

**Classification:** BLOCKER
**File:** `crates/project_store/src/lib.rs:57`

**Issue:** `StdPlatformFileSystem::write_string` finishes the atomic write with `std::fs::rename(&temp_path, path)`. Rust documents replacement behavior for an existing destination as platform-specific; on Windows, `rename` fails if `project.json` already exists. The first project save can work, but autosave or a normal save over an existing `.veproj/project.json` fails on Windows, which is a desktop target for this Electron app.

**Fix:** Use a cross-platform atomic replace implementation instead of raw `std::fs::rename`, for example a vetted crate or a small platform-specific replacement helper. Keep the same-directory temp file and file sync, and add a Windows-covered test or abstraction-level fake proving replacement of an existing destination succeeds.

## Warnings

### WR-01: Duration parsing silently accepts malformed ffprobe values

**Classification:** WARNING
**File:** `crates/media_runtime/src/probe.rs:326`

**Issue:** `parse_decimal_seconds_to_microseconds` filters non-digit characters out of the fractional component and uses saturating arithmetic. A malformed ffprobe duration like `1.-5` becomes `1_500_000` microseconds instead of `InvalidDuration`, and an overflowing whole-second value clamps to `u64::MAX`. That can persist incorrect material duration metadata instead of surfacing a classified probe error.

**Fix:** Reject any non-digit fractional character and use checked arithmetic.

```rust
if !fractional.bytes().all(|byte| byte.is_ascii_digit()) {
    return Err(format!("invalid duration fraction `{value}`"));
}
let whole_micros = whole
    .parse::<u64>()
    .map_err(|error| format!("invalid duration seconds `{value}`: {error}"))?
    .checked_mul(1_000_000)
    .ok_or_else(|| format!("duration is too large `{value}`"))?;
```

### WR-02: Zero-numerator frame rates pass the runtime probe layer

**Classification:** WARNING
**File:** `crates/media_runtime/src/probe.rs:358`

**Issue:** `parse_rational_frame_rate` rejects a zero denominator but accepts `0/1`. The draft model later rejects zero frame-rate numerators, so an import can succeed at the runtime normalization layer and then fail as an invalid project mutation. This makes the error classification unstable and can turn malformed probe metadata into a project-validation error.

**Fix:** Reject zero numerators in `parse_rational_frame_rate` and add a probe test for `r_frame_rate: "0/1"`.

```rust
if numerator == 0 {
    return Err("frame rate numerator cannot be zero".to_string());
}
if denominator == 0 {
    return Err("frame rate denominator cannot be zero".to_string());
}
```

---

_Reviewed: 2026-06-17T04:24:25Z_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_
