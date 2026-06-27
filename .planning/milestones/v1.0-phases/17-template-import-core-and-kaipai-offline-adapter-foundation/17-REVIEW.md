---
phase: 17-template-import-core-and-kaipai-offline-adapter-foundation
reviewed: 2026-06-24T11:29:55Z
depth: standard
files_reviewed: 63
files_reviewed_list:
  - apps/desktop-electron/src/generated/TemplateImport.ts
  - apps/desktop-electron/src/main/index.ts
  - apps/desktop-electron/src/main/nativeBinding.ts
  - apps/desktop-electron/src/preload/index.ts
  - apps/desktop-electron/src/renderer/App.tsx
  - apps/desktop-electron/src/renderer/styles.css
  - apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx
  - apps/desktop-electron/src/renderer/workspace/WorkspaceShell.tsx
  - apps/desktop-electron/tests/helpers/foregroundProductApp.ts
  - apps/desktop-electron/tests/template-import.spec.ts
  - crates/adapter_kaipai/Cargo.toml
  - crates/adapter_kaipai/src/error.rs
  - crates/adapter_kaipai/src/formula_bundle.rs
  - crates/adapter_kaipai/src/lib.rs
  - crates/adapter_kaipai/src/mapper.rs
  - crates/adapter_kaipai/tests/fixtures.rs
  - crates/adapter_kaipai/tests/formula_bundle_contract.rs
  - crates/adapter_kaipai/tests/mapper.rs
  - crates/adapter_kaipai/tests/mapper_fixtures.rs
  - crates/adapter_kaipai/tests/schema_exports.rs
  - crates/artifact_store/src/resource_index.rs
  - crates/bindings_node/Cargo.toml
  - crates/bindings_node/src/lib.rs
  - crates/bindings_node/src/project_session_service.rs
  - crates/bindings_node/tests/project_session_import_kaipai.rs
  - crates/draft_import/Cargo.toml
  - crates/draft_import/src/adaptation_report.rs
  - crates/draft_import/src/import_plan.rs
  - crates/draft_import/src/lib.rs
  - crates/draft_import/src/resource_localizer.rs
  - crates/draft_import/src/validation.rs
  - crates/draft_import/tests/adaptation_report.rs
  - crates/draft_import/tests/import_plan.rs
  - crates/draft_import/tests/resource_localizer.rs
  - crates/draft_import/tests/schema_exports.rs
  - crates/ffmpeg_compiler/src/filters.rs
  - crates/ffmpeg_compiler/tests/transform_snapshots.rs
  - crates/render_graph/src/graph.rs
  - crates/testkit/Cargo.toml
  - crates/testkit/tests/preview_export_parity.rs
  - crates/testkit/tests/template_import_exports.rs
  - crates/testkit/tests/template_import_preview.rs
  - fixtures/kaipai/expected-reports/bgm-audio.report.json
  - fixtures/kaipai/expected-reports/main-video.report.json
  - fixtures/kaipai/expected-reports/missing-resource.report.json
  - fixtures/kaipai/expected-reports/native-effect.report.json
  - fixtures/kaipai/expected-reports/pip-overlay.report.json
  - fixtures/kaipai/expected-reports/text-sticker.report.json
  - fixtures/kaipai/negative/missing-resource.json
  - fixtures/kaipai/negative/native-effect.json
  - fixtures/kaipai/negative/unknown-top-level-field.json
  - fixtures/kaipai/negative/unsafe-formula-evidence.json
  - fixtures/kaipai/positive/bgm-audio.json
  - fixtures/kaipai/positive/main-video.json
  - fixtures/kaipai/positive/pip-overlay.json
  - fixtures/kaipai/positive/sanitized-formula-bundle.json
  - fixtures/kaipai/positive/sanitized-formula-with-direct-materials.json
  - fixtures/kaipai/positive/text-sticker.json
  - fixtures/kaipai/resources/README.md
  - schemas/adaptation-report.schema.json
  - schemas/draft-import-plan.schema.json
  - schemas/kaipai-formula-bundle.schema.json
  - scripts/phase17-source-guards.sh
findings:
  critical: 2
  warning: 1
  info: 0
  total: 3
status: issues_found
---

# Phase 17: Code Review Report

**Reviewed:** 2026-06-24T11:29:55Z
**Depth:** standard
**Files Reviewed:** 63
**Status:** issues_found

## Summary

Reviewed the scoped Electron, Rust adapter/import/session/render, schemas, fixtures, and test files for the Phase 17 Kaipai offline template import work. The UI correctly routes the template import through the Rust-owned project-session API and does not construct draft or render semantics itself, but the adapter and compiler still contain correctness defects that lose imported content or silently mis-render supported transforms.

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: BLOCKER - Kaipai mapper drops supported sections in mixed templates

**File:** `crates/adapter_kaipai/src/mapper.rs:200`
**Issue:** `map_formula` classifies the formula into exactly one `FixtureFamily` and each match arm maps only that family. The classifier prioritizes `nativeEffectList`, text stickers, `bgm`, `pipList`, then `videoClipList` at `crates/adapter_kaipai/src/mapper.rs:782`, so a real mixed template can silently lose supported content: a formula with text stickers maps only stickers, a formula with BGM maps only audio, and a formula with native effects maps only video plus native-effect diagnostics. Phase 17 claims main video, PIP overlays, text/stickers, and BGM are all supported offline adapter inputs; silently dropping the other supported lists is incorrect behavior and violates the adapter boundary by making fixture taxonomy decide draft semantics. The existing tests only map isolated fixture families, while `fixtures/kaipai/positive/sanitized-formula-with-direct-materials.json` already contains `videoClipList`, `pipList`, and `bgm` together without a mapper regression.
**Fix:**
```rust
fn map_formula(
    &mut self,
    formula: &Map<String, Value>,
    canvas_config: &DraftCanvasConfig,
) -> Result<(), AdapterKaipaiError> {
    self.map_video_clip_list(formula, canvas_config, true)?;
    self.map_pip_list(formula, canvas_config)?;
    self.map_sticker_list(formula, canvas_config)?;
    self.map_bgm(formula)?;
    self.report_native_effects(formula)?;
    Ok(())
}
```
Add a mixed-template mapper test that imports video + PIP + text/sticker + BGM + native-effect evidence in one formula and asserts all supported tracks/materials are present while only unsupported native effects are reported as dropped/needs-native-effect.

### CR-02: BLOCKER - Full-canvas rotated layers bypass the rotation compiler

**File:** `crates/ffmpeg_compiler/src/filters.rs:312`
**Issue:** `compile_visual_layer` returns the full-canvas fast path before applying transform filters whenever `is_full_canvas_identity` is true. That predicate checks fit, crop, scale, position, anchor, and opacity at `crates/ffmpeg_compiler/src/filters.rs:500`, but it does not check `visual.transform.rotation.degrees`. Render graph diagnostics now allow static center-anchor rotation and only reject non-center anchors at `crates/render_graph/src/graph.rs:833`, so a full-canvas Stretch segment with rotation 90 degrees is classified as supported but exported with only `scale=...` and no `rotate=...`. This silently loses a supported visual transform in export output.
**Fix:**
```rust
fn is_full_canvas_identity(visual: &SegmentVisual) -> bool {
    visual.fit_mode == SegmentFitMode::Stretch
        && !crop_is_active(&visual.transform.crop)
        && visual.transform.scale.x_millis == 1_000
        && visual.transform.scale.y_millis == 1_000
        && visual.transform.position.x == 0
        && visual.transform.position.y == 0
        && visual.transform.anchor.x_millis == 500
        && visual.transform.anchor.y_millis == 500
        && visual.transform.rotation.degrees.rem_euclid(360) == 0
        && visual.transform.opacity.value_millis == 1_000
}
```
Add a compiler regression test for a full-canvas Stretch layer with center anchor, unit scale, zero position, full opacity, and `rotation.degrees = 90`; assert the filter script contains `rotate=` and `ow=rotw`/`oh=roth`.

## Warnings

### WR-01: WARNING - Failed import persistence can leave copied template resources orphaned

**File:** `crates/bindings_node/src/project_session_service.rs:2282`
**Issue:** `map_kaipai_bundle_to_import_plan` localizes resources before persistence by calling `localize_template_resources` at `crates/adapter_kaipai/src/mapper.rs:69`, and `CopyRenderableResources` writes files into the `.veproj/resources/template-import/...` tree at `crates/draft_import/src/resource_localizer.rs:314`. If `index_draft_resources_with_extra_refs` fails after `save_project_bundle`, the session code rolls back `project.json` and the resource index at `crates/bindings_node/src/project_session_service.rs:2291`, but it never removes the files copied during localization. The tests cover no partial `project.json` or index rows, but they do not assert the resource directory is clean. This leaves unreferenced bundle files after a failed import commit and weakens the project bundle's canonical/derived artifact separation.
**Fix:** Stage localized resources in a transaction-specific temporary directory and only move them into `resources/template-import/...` after both project save and resource-index persistence succeed. Alternatively, track the current transaction's copied `project_relative_ref` values and remove only those paths on every post-localization failure, taking care not to delete paths that belonged to a previous successful import with the same import id.

---

_Reviewed: 2026-06-24T11:29:55Z_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_
