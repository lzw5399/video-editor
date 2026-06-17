---
phase: 02-draft-and-material-system
reviewed: 2026-06-17T03:56:25Z
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
  critical: 1
  warning: 3
  info: 0
  total: 4
status: issues_found
---

# Phase 02: Code Review Report

**Reviewed:** 2026-06-17T03:56:25Z
**Depth:** standard
**Files Reviewed:** 44
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

Reviewed the Phase 2 draft/material model, project store path handling, media probe boundary, Node binding command envelopes, generated TypeScript/JSON Schema contracts, renderer smoke surface, fixtures, and tests. `Cargo.lock` was in the workflow input but excluded from source review as a lock file.

The main risk is persistence correctness: saves write the canonical `.veproj/project.json` in place. There are also contract/path portability issues that can make generated artifacts accept or advertise payload shapes that the Rust boundary does not actually support.

## Critical Issues

### CR-01: Canonical project saves can corrupt `project.json`

**Classification:** BLOCKER
**File:** `crates/project_store/src/lib.rs:47`

**Issue:** `StdPlatformFileSystem::write_string` writes directly to `project_json_path` through `std::fs::write`. `save_project_bundle` and `autosave_project_bundle` both use this path for `.veproj/project.json`, which AGENTS.md defines as the canonical source of truth. If the process exits, the OS errors, or storage fills after truncating the existing file, the previous draft can be lost or left as invalid JSON.

**Fix:**
```rust
fn write_string(&self, path: &Path, contents: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("project.json");
    let temp_path = path.with_file_name(format!(".{file_name}.tmp"));

    std::fs::write(&temp_path, contents)?;
    std::fs::rename(&temp_path, path)?;
    Ok(())
}
```
Use a same-directory temporary file, flush/sync if durability is required, rename atomically, and add a project-store test using a failing filesystem that proves an existing `project.json` remains readable when the replacement write fails.

## Warnings

### WR-01: Windows absolute material paths are classified as external URIs

**Classification:** WARNING
**File:** `crates/project_store/src/paths.rs:34`

**Issue:** `classify_material_uri` calls `has_uri_scheme` before checking `Path::is_absolute`. A Windows path such as `C:\Users\me\clip.mp4` or `C:/Users/me/clip.mp4` has a colon after `C`, so it is classified as `ExternalUri` with `resolved_path: None` instead of `ExternalAbsolute`. That breaks missing-material diagnostics and future preview/render path resolution for local Windows material paths.

**Fix:** Check platform absolute paths before URI schemes, and add a Windows-path regression test.

```rust
let path = Path::new(trimmed);
if path.is_absolute() {
    return Ok(MaterialUri {
        kind: MaterialUriKind::ExternalAbsolute,
        uri: trimmed.to_owned(),
        resolved_path: Some(path.to_path_buf()),
    });
}

if has_uri_scheme(trimmed) {
    return Ok(MaterialUri {
        kind: MaterialUriKind::ExternalUri,
        uri: trimmed.to_owned(),
        resolved_path: None,
    });
}
```

### WR-02: Generated TypeScript advertises `bigint` microseconds over a JSON number boundary

**Classification:** WARNING
**File:** `apps/desktop-electron/src/generated/Draft.ts:7`

**Issue:** `Microseconds` is generated as `bigint`, while `schemas/draft.schema.json` exposes it as a JSON integer and the Node binding accepts/returns `serde_json::Value`. The renderer already has to bypass the generated contract with `value as unknown as Microseconds` in `apps/desktop-electron/src/renderer/App.tsx:212`. A TypeScript caller that follows the generated type and sends `1000000n` will not match the JSON schema/serde boundary; a caller that sends `1000000` has to lie to TypeScript.

**Fix:** Pick one IPC representation and make all generated artifacts match it. For the current JSON envelope, export `Microseconds` to TypeScript as `number` (or a decimal string if large values must exceed JS safe integers), remove the renderer cast, and add a contract test that fails if generated `Draft.ts` contains `export type Microseconds = bigint`.

### WR-03: Draft JSON Schema accepts unsupported schema versions

**Classification:** WARNING
**File:** `schemas/draft.schema.json:58`

**Issue:** The generated draft schema defines `DraftSchemaVersion` as any uint32-like integer with minimum `0`, so JSON Schema validation accepts `schemaVersion: 2`. Rust migration rejects anything except `DraftSchemaVersion::CURRENT_VALUE` in `crates/draft_model/src/validation.rs:68`, and the fixture test explicitly skips schema validation for `negative/invalid-schema-version/project.json` at `crates/draft_model/tests/draft_fixtures.rs:88`. This creates a stale contract for tools that validate `.veproj/project.json` with the committed schema before handing it to Rust.

**Fix:** Customize the exported draft schema to constrain the current version, then remove the test skip.

```rust
fn draft_schema_json() -> String {
    let schema = schema_for!(Draft);
    let mut schema_value =
        serde_json::to_value(schema).expect("draft schema should serialize to JSON value");
    schema_value["$defs"]["DraftSchemaVersion"] =
        json!({ "type": "integer", "const": DraftSchemaVersion::CURRENT_VALUE });
    serde_json::to_string_pretty(&schema_value).expect("draft schema should serialize")
}
```

---

_Reviewed: 2026-06-17T03:56:25Z_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_
