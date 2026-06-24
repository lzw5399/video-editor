# Phase 17: Template Import Core And Kaipai Offline Adapter Foundation - Research

**Researched:** 2026-06-24 [VERIFIED: init.phase-op]
**Domain:** Provider-neutral template import, offline Kaipai adapter, resource localization, adaptation reports, project-session import application, preview/export evidence [VERIFIED: .planning/phases/17-template-import-core-and-kaipai-offline-adapter-foundation/17-CONTEXT.md]
**Confidence:** HIGH for repository architecture, old-branch asset inventory, and validation gates; MEDIUM for exact Kaipai formula field coverage because only sanitized old fixtures were inspected in this session. [VERIFIED: codebase grep; VERIFIED: git show origin/work/kaipai-adapter-poc; VERIFIED: .planning/phases/17-template-import-core-and-kaipai-offline-adapter-foundation/17-CONTEXT.md]

<user_constraints>
## User Constraints (from CONTEXT.md)

All entries in this copied section are sourced from `.planning/phases/17-template-import-core-and-kaipai-offline-adapter-foundation/17-CONTEXT.md`. [VERIFIED: 17-CONTEXT.md]

### Locked Decisions

## Implementation Decisions

### Product Target And Fidelity
- **D-01:** The core capability is generic editing/template import and rendering. Kaipai is only one external adapter that converts offline input into the application's canonical draft model.
- **D-02:** The fidelity target is high-quality approximate rendering: reasonable, previewable, editable, exportable results with explicit diagnostics. Pixel-level Kaipai parity is out of scope.
- **D-03:** Unsupported proprietary effects, complex text effects, complex transitions, and unavailable resources must be surfaced as adaptation report entries instead of hidden behind fake support.

### Core Ownership Boundary
- **D-04:** Core layers may consume only canonical `.veproj/project.json`, material/track/segment/keyframe/filter/transition/text/sticker semantics, local resource references, render graph/preview/export contracts, and provider-neutral import/adaptation reports.
- **D-05:** Core, render, preview, export, and session layers must not contain Kaipai API logic, Android worker integration, raw Kaipai formula interpretation, provider-specific template IDs as render semantics, or Kaipai-specific `safe_area` behavior.
- **D-06:** `templateId`, `recipeId`, formula task IDs, raw formula JSON, recognizer output, and Kaipai `safe_area` evidence may be preserved as adapter input/provenance or report evidence, but must not become canonical render semantics in `.veproj/project.json`.
- **D-07:** Generic text layout safe-area concepts may remain where they are already canonical editor behavior. The ban is on Kaipai-specific `safe_area` provider logic inside core/render crates.

### Old Branch Reuse
- **D-08:** Reuse the old `origin/work/kaipai-adapter-poc` branch as source material only. Valuable assets include `crates/adapter_kaipai/`, `fixtures/kaipai/`, `schemas/kaipai-formula-bundle.schema.json`, `schemas/compatibility-report.schema.json`, `.codex/skills/spike-findings-video-editor-kaipai-adapter/`, and the old `.planning/phases/03.1-*` artifacts.
- **D-09:** Do not merge the old integration layer directly. The old branch predates current main's project-session, resource, preview, scheduler, transform, font, and no-fallback architecture; Phase 17 must rewrite the integration against current main.
- **D-10:** Preserve old adapter contracts, fixtures, schema snapshots, validation ideas, and report taxonomy when they still match the new provider-neutral boundary. Rewrite naming and ownership where needed so Kaipai stays outside core semantics.

### Import Pipeline
- **D-11:** Define a provider-neutral `DraftImportPlan` before applying imported content to a project session. Adapters should emit this plan; they must not directly mutate arbitrary draft fields or write `.veproj/project.json` by hand.
- **D-12:** The project-session layer applies a validated `DraftImportPlan` into the canonical `Draft`, owns revision changes, and exposes a narrow Rust command/API for importing an offline Kaipai formula bundle.
- **D-13:** The target chain is:

```text
KaipaiFormulaBundle
  -> adapter_kaipai parse/validate
  -> resource localizer writes .veproj/resources
  -> DraftImportPlan
  -> project_session applies canonical Draft changes
  -> realtime preview
  -> export
  -> AdaptationReport
```

### Resource Localization
- **D-14:** Kaipai resources must be sanitized and localized into `.veproj/resources/...` before preview/export depends on them. Rendering must not rely on remote template URLs.
- **D-15:** Localized resources should enter the current artifact/resource indexing system where appropriate, preserving project-relative refs, sha256/fingerprint evidence, missing-resource diagnostics, and safe path validation.
- **D-16:** Resource localization must handle path traversal, remote render URLs, missing files, sha256 mismatch, duplicate destinations, and sanitized fixture data. No tokens, signed URLs, cookies, account IDs, or credentials may be committed.

### First Supported Subset
- **D-17:** First version supports canvas width/height/aspect/fps/background color.
- **D-18:** Main video maps source/target timeranges, crop or fit/fill, position, scale, opacity, and basic transform into generic draft segment semantics.
- **D-19:** PIP maps image/video overlays onto normal material-backed overlay tracks. Kaipai `level` maps to generic track ordering/z-order behavior, not provider-specific runtime logic.
- **D-20:** Basic stickers are treated first as image/video overlay segments. Do not introduce a native Kaipai sticker runtime in Phase 17.
- **D-21:** Text stickers support content, position, font size, color, stroke, shadow, basic layout, and font fallback via existing or extended `fontRef` semantics.
- **D-22:** BGM/audio maps audio material, volume, fade-in, and fade-out into generic audio segment semantics.
- **D-23:** Simple animation maps position, scale, and opacity keyframes. Complex curves, native motion presets, and proprietary animation effects are approximated or reported as unsupported/degraded.
- **D-24:** Simple transitions may map to opacity fade/dissolve when the canonical model and preview/export support it. Other transitions must be reported as degraded, unsupported, or `needsNativeEffect`.
- **D-25:** Native effects are not reproduced in Phase 17. They must be reported as `needsNativeEffect` or degraded.

### Generic Core Capability Gaps
- **D-26:** Phase 17 planning must audit and fill only the generic gaps needed by the supported subset: `DraftImportPlan`, resource localizer integration, overlay/sticker bounds and z-order semantics, font resource closure, center-anchor rotation parity between preview/export, constant speed mapping, and `AdaptationReport`.
- **D-27:** Constant speed support should map values such as `durationMsWithSpeed` into canonical source/target duration or explicit degraded diagnostics. Complex speed curves remain later production retiming work unless planning proves they are already supported generically.
- **D-28:** Rotation and anchor behavior should use a generic center-anchor model with preview/export consistency. Do not add Kaipai-specific placement hacks.
- **D-29:** Image/video sticker semantics may initially reuse material-backed segments, but the plan must make z-order, bounds, fit, opacity, and transform explicit enough for preview/export tests.
- **D-30:** Font handling must form a closed loop: local `fontRef` where available, fallback when unsupported, and report entries when a requested font cannot be localized or rendered consistently.

### Adaptation Report
- **D-31:** The report is a product-facing capability explanation, not merely a failure report. It must classify at least `supported`, `approximated`, `dropped`, `missingResource`, and `needsNativeEffect`.
- **D-32:** The report should be provider-neutral enough for future Jianying/CapCut adapters, while preserving external references as non-semantic provenance.
- **D-33:** Report snapshots are required for supported main video, PIP, text sticker, BGM, missing resource, and native effect degradation fixtures.

### Implementation Order
- **D-34:** Start by porting the old adapter ideas and fixtures into current main's shape, but rewrite the integration layer.
- **D-35:** First implement offline `KaipaiFormulaBundle` parsing/validation, fixture loading, and adaptation report output without UI.
- **D-36:** Add resource localization into `.veproj/resources` and resource index integration before mapped drafts depend on template assets.
- **D-37:** Add provider-neutral `DraftImportPlan` and map the supported subset into canonical draft semantics.
- **D-38:** Integrate with project session through a new Rust command/API for importing an offline Kaipai formula bundle.
- **D-39:** Add five golden fixture families before UI: main video, PIP, text sticker, BGM/audio, and native effect degradation.
- **D-40:** Add the desktop UI entry and report panel only after the offline import path, report, resource localization, project-session application, preview, and export gates are stable.

### Verification Gates
- **D-41:** Source guards must prove core/render crates do not import or interpret Kaipai provider code, Android worker code, raw formula JSON, live provider APIs, or provider-specific render semantics.
- **D-42:** Imported `.veproj/project.json` must not contain raw formula JSON or remote render URLs as required runtime dependencies.
- **D-43:** Preview and export must work without Android runtime, without live Kaipai API access, and without old artifact fallback paths.
- **D-44:** Each fixture export must produce a non-empty MP4 with correct layer ordering, visible text where expected, and audio stream presence for audio fixtures.
- **D-45:** Supported subset evidence must go through the realtime preview product path and export path. Old artifact fallback, mock, CPU readback, or Android oracle output cannot satisfy product success.
- **D-46:** Adaptation reports must explicitly identify approximated, dropped, missing resource, and native-effect-dependent features.

### the agent's Discretion
- Exact crate/module names are flexible as long as ownership remains clear. Likely candidates are an adapter crate for Kaipai, a provider-neutral import-plan module/crate, and report types shared at the import boundary.
- The planner may split generic core capability gaps across multiple Phase 17 plans, but it must not implement UI first or let adapter-specific shortcuts define core semantics.
- The planner may decide whether `AdaptationReport` evolves from the old `CompatibilityReport` schema or becomes a renamed provider-neutral contract, as long as the required classifications and snapshots are preserved.

### Deferred Ideas (OUT OF SCOPE)

## Deferred Ideas

- Live Kaipai API/provider integration, auth, submit/poll, retries, and rate limits are deferred until offline import is stable.
- Android worker replacement, ASR-to-`word_list`, independent `safe_area` generation, and provider formula acquisition are separate future spikes or phases.
- Pixel-perfect Kaipai/Jianying/CapCut parity, proprietary native effects, complex flower text, beauty/matting/AR effects, and complex transitions remain out of Phase 17.
- Full UI import entry and report panel should wait until backend import/report/preview/export gates are stable.
- Advanced retiming/effect/transition engines remain downstream production effects work unless Phase 17 only needs a generic minimal subset for approximate import.
</user_constraints>

<phase_requirements>
## Phase Requirements

Phase requirement IDs were `null / TBD`, so this table maps the nearest active project requirements that Phase 17 must support. [VERIFIED: user prompt; VERIFIED: .planning/REQUIREMENTS.md]

| ID | Description | Research Support |
|----|-------------|------------------|
| COMP-01 | User can import a supported Jianying/CapCut draft subset into `.veproj`. [VERIFIED: .planning/REQUIREMENTS.md] | Phase 17 should implement provider-neutral `DraftImportPlan` application and a Kaipai offline adapter as the first external adapter path. [VERIFIED: 17-CONTEXT.md D-01, D-11-D-13] |
| COMP-02 | User receives a compatibility report listing supported, degraded, and unsupported external draft features. [VERIFIED: .planning/REQUIREMENTS.md] | Rename/evolve the old compatibility report into `AdaptationReport` with `supported`, `approximated`, `dropped`, `missingResource`, and `needsNativeEffect` snapshots. [VERIFIED: 17-CONTEXT.md D-31-D-33; VERIFIED: git show origin/work/kaipai-adapter-poc:crates/adapter_kaipai/src/compatibility_report.rs] |
| PRODFX-05 | Complex Jianying/Kaipai-like template fixtures verify preview/export parity, fallback reports, and performance budgets. [VERIFIED: .planning/REQUIREMENTS.md] | First fixture families should cover main video, PIP, text sticker, BGM/audio, missing resources, and native effect degradation before UI. [VERIFIED: 17-CONTEXT.md D-33, D-39, D-44-D-46] |
| NO-FALLBACK-01 | Product success cannot be satisfied by fallback, mock, artifact, CPU, or legacy evidence. [VERIFIED: .planning/REQUIREMENTS.md; VERIFIED: docs/no-product-fallback-policy.md] | Imported drafts must preview through realtime render-graph GPU evidence and export through the normal export path; Android oracle and old artifacts are calibration only. [VERIFIED: 17-CONTEXT.md D-43-D-45; VERIFIED: docs/no-product-fallback-policy.md] |
| NO-FALLBACK-02 | Refactors remove or gate obsolete legacy implementations. [VERIFIED: .planning/REQUIREMENTS.md; VERIFIED: docs/refactor-and-legacy-cleanup-policy.md] | Do not merge old branch integration directly; port contracts/fixtures only and rewrite current-main integration. [VERIFIED: 17-CONTEXT.md D-08-D-10] |
| TEST-E2E-01 | Visible editing features changed after Phase 15.1 need product workflow coverage. [VERIFIED: .planning/REQUIREMENTS.md; VERIFIED: docs/product-e2e-acceptance-policy.md] | UI import/report panel should be planned only after backend path has preview/export gates; once visible, add product E2E. [VERIFIED: 17-CONTEXT.md D-40; VERIFIED: docs/product-e2e-acceptance-policy.md] |
</phase_requirements>

## Project Constraints (from AGENTS.md)

- UI emits commands and Rust core owns project/timeline semantics; UI code must not construct FFmpeg commands. [VERIFIED: AGENTS.md]
- Wrong preview, edit, render, session, media, or native-surface ownership boundaries must be replaced with production architecture instead of patched around. [VERIFIED: AGENTS.md]
- `.veproj/project.json` is the canonical semantic source of truth; render graphs, FFmpeg scripts, thumbnails, waveform data, proxies, and preview caches are derived artifacts. [VERIFIED: AGENTS.md]
- Product terminology should follow Jianying concepts such as draft/material/track/segment/keyframe/filter/transition. [VERIFIED: AGENTS.md]
- Core time math must use integer microseconds, frame indices, or rational frame rates, and persisted semantics must avoid naked floating-point time. [VERIFIED: AGENTS.md]
- Render Graph isolates editing semantics from FFmpeg; FFmpeg Runtime executes jobs and reports progress/errors without deciding editing behavior. [VERIFIED: AGENTS.md]
- Kdenlive and MLT are references only; do not copy GPL code, assets, XML definitions, presets, or UI implementation. [VERIFIED: AGENTS.md]
- External drafts go through adapters and produce compatibility/adaptation reports; proprietary IDs are external references, not internal render semantics. [VERIFIED: AGENTS.md]
- Every roadmap phase must define executable gates before implementation is complete. [VERIFIED: AGENTS.md]
- FFmpeg redistribution remains subject to LGPL/GPL/nonfree notices and commercial obligations review. [VERIFIED: AGENTS.md; VERIFIED: docs/runtime-boundaries.md]
- The production architecture review skill requires planning from the production target chain and rejects fallback or legacy success evidence. [VERIFIED: .agents/skills/production-architecture-review/SKILL.md]

## Summary

Phase 17 should be planned as a backend-first import foundation, not as a Kaipai UI or provider-integration feature. [VERIFIED: 17-CONTEXT.md D-34-D-40] The production target is: sanitized offline Kaipai input -> adapter validation -> resource localization into `.veproj/resources` -> provider-neutral `DraftImportPlan` -> project-session application -> realtime preview/export -> `AdaptationReport`. [VERIFIED: 17-CONTEXT.md D-13] This chain keeps Kaipai-specific formula interpretation outside core/render crates and uses existing current-main semantics for canvas, materials, tracks, segments, transforms, text, audio, keyframes, preview, export, artifacts, and scheduler. [VERIFIED: AGENTS.md; VERIFIED: crates/draft_model/src/timeline.rs; VERIFIED: crates/bindings_node/src/project_session_service.rs; VERIFIED: crates/render_graph/src/graph.rs]

The old `origin/work/kaipai-adapter-poc` branch is valuable source material but not mergeable as-is. [VERIFIED: 17-CONTEXT.md D-08-D-10] It contains a strict `KaipaiFormulaBundle`, localizer tests for traversal/remote URL/missing/sha mismatch/symlink cases, report taxonomy snapshots, and sanitized fixtures. [VERIFIED: git ls-tree origin/work/kaipai-adapter-poc; VERIFIED: git show origin/work/kaipai-adapter-poc:crates/adapter_kaipai/src/formula_bundle.rs; VERIFIED: git show origin/work/kaipai-adapter-poc:crates/adapter_kaipai/src/resource_localizer.rs] Its old gap inventory is partly stale because current main now has canvas, transform, text, audio fades, keyframes, resource index, realtime preview, audio engine, and scheduler. [VERIFIED: crates/draft_model/src/canvas.rs; VERIFIED: crates/draft_model/src/timeline.rs; VERIFIED: crates/artifact_store/src/resource_index.rs; VERIFIED: crates/realtime_preview_runtime/src/gpu/compositor.rs; VERIFIED: crates/task_runtime/src/scheduler.rs]

The main generic gaps to plan are `DraftImportPlan`, `AdaptationReport`, resource localization integration with the current resource index, importer-side material probing/metadata closure, explicit overlay/sticker z-order and bounds tests, font localization/fallback reporting, static rotation preview/export parity, constant speed mapping, and fixture-level product gates. [VERIFIED: 17-CONTEXT.md D-26-D-33; VERIFIED: crates/ffmpeg_compiler/tests/transform_snapshots.rs] Current realtime preview supports center-anchor rotation math, while current FFmpeg export snapshots still classify rotation as unsupported, so Phase 17 must not claim rotated import support unless it closes that generic export gap or reports the rotation as degraded. [VERIFIED: crates/realtime_preview_runtime/src/gpu/compositor.rs; VERIFIED: crates/ffmpeg_compiler/tests/transform_snapshots.rs]

**Primary recommendation:** Implement Phase 17 in waves: adapter/report fixture foundation, resource localization, provider-neutral import plan, project-session import API, preview/export fixture gates, then optional UI entry/report panel after backend gates pass. [VERIFIED: 17-CONTEXT.md D-34-D-40]

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|--------------|----------------|-----------|
| Offline Kaipai bundle selection | Electron main/preload | Renderer UI | File selection is shell/UI work, but the selected path should only invoke a narrow Rust import API. [VERIFIED: AGENTS.md; VERIFIED: apps/desktop-electron/src/preload/index.ts] |
| Kaipai formula parsing and validation | Adapter crate boundary | Fixture/schema tests | Raw provider JSON and provenance belong in `adapter_kaipai`, not core/render crates. [VERIFIED: 17-CONTEXT.md D-04-D-06; VERIFIED: git show origin/work/kaipai-adapter-poc:crates/adapter_kaipai/src/formula_bundle.rs] |
| Resource localization | Provider-neutral import/resource service | `project_store` and `artifact_store` | Resources must be copied/validated before draft semantics depend on them, and project-relative refs should enter resource indexing. [VERIFIED: 17-CONTEXT.md D-14-D-16; VERIFIED: crates/project_store/src/paths.rs; VERIFIED: crates/artifact_store/src/resource_index.rs] |
| `DraftImportPlan` validation | Provider-neutral Rust import core | `draft_model` validation | The adapter should emit a plan; project session applies it atomically into canonical draft semantics. [VERIFIED: 17-CONTEXT.md D-11-D-12] |
| Session revision/application | `bindings_node::project_session_service` | `project_store` | Current main already owns revisioned session state, bundle save/open, material import, and typed project intents here. [VERIFIED: crates/bindings_node/src/project_session_service.rs; VERIFIED: crates/project_store/src/bundle.rs] |
| Preview evidence | `realtime_preview_runtime` / `preview_service` | Electron native preview host | Product success must use render-graph GPU composited preview evidence, not fallback/artifact/Android output. [VERIFIED: docs/no-product-fallback-policy.md; VERIFIED: docs/runtime-boundaries.md] |
| Export evidence | `render_graph` -> `ffmpeg_compiler` -> `media_runtime` | `bindings_node` export service | Export must consume canonical draft/render graph semantics and produce validated MP4 output. [VERIFIED: AGENTS.md; VERIFIED: crates/render_graph/src/graph.rs; VERIFIED: crates/ffmpeg_compiler/src/filters.rs] |
| Adaptation report display | Renderer UI after backend gates | Rust report contract | The report is product-facing explanation data; UI should show it only after backend report snapshots and import/export gates are stable. [VERIFIED: 17-CONTEXT.md D-31-D-40] |

## Standard Stack

### Core

| Library / Module | Version | Purpose | Why Standard |
|------------------|---------|---------|--------------|
| Rust workspace | rustc 1.95.0 / edition 2024 | Implement import core, adapter, reports, tests, and binding route in the same ownership model as the editor core. | Workspace root requires Rust 1.95.0 and all semantic/runtime crates are Rust-owned. [VERIFIED: Cargo.toml; VERIFIED: rustc --version] |
| `draft_model` | 0.1.0 | Canonical draft/material/track/segment/canvas/text/audio/keyframe semantics and validation. | `.veproj/project.json` must stay canonical and provider-neutral. [VERIFIED: crates/draft_model/Cargo.toml; VERIFIED: crates/draft_model/src/lib.rs] |
| New provider-neutral import-plan module/crate | Internal 0.1.0 | Define `DraftImportPlan`, plan validation, localized resource mapping, and `AdaptationReport` contract. | Context requires adapters to emit a provider-neutral plan before project-session mutation. [VERIFIED: 17-CONTEXT.md D-11-D-13] |
| `adapter_kaipai` | Internal 0.1.0 | Parse/validate sanitized offline `KaipaiFormulaBundle`, map supported subset into import plan, and emit report evidence. | Old branch has reusable contracts/fixtures, but integration must be rewritten against current main. [VERIFIED: 17-CONTEXT.md D-08-D-10; VERIFIED: git show origin/work/kaipai-adapter-poc:crates/adapter_kaipai/Cargo.toml] |
| `project_store` | 0.1.0 | Bundle save/open, `.veproj/project.json` validation, bundle-relative path classification. | Existing path helpers reject parent traversal and classify bundle-relative/external URI refs. [VERIFIED: crates/project_store/Cargo.toml; VERIFIED: crates/project_store/src/paths.rs] |
| `artifact_store` resource index | 0.1.0 | Index localized materials/fonts/effects/filters/transitions and project-relative refs. | Existing `index_draft_resources` persists material/font/effect/filter/transition rows. [VERIFIED: crates/artifact_store/Cargo.toml; VERIFIED: crates/artifact_store/src/resource_index.rs; VERIFIED: crates/artifact_store/tests/resource_index.rs] |
| `bindings_node::project_session_service` | 0.1.0 | Apply import plan to a live session, own revision changes, save the bundle, and expose narrow command/API. | Current session service owns project session draft, revision, material import, canvas/text/audio/visual/keyframe intents, and bundle persistence. [VERIFIED: crates/bindings_node/src/project_session_service.rs] |
| `realtime_preview_runtime` / `render_graph` / `ffmpeg_compiler` | 0.1.0 | Verify imported drafts through production preview and export paths. | Product evidence must use existing render-graph GPU preview and export paths. [VERIFIED: docs/no-product-fallback-policy.md; VERIFIED: crates/realtime_preview_runtime/src/gpu/compositor.rs; VERIFIED: crates/ffmpeg_compiler/src/filters.rs] |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `serde` / `serde_json` | 1.0.228 / 1.0.150 | Strict JSON contracts for formula bundles, import plans, and reports. | Use for all adapter/import/report wire and fixture data. [VERIFIED: crates/draft_model/Cargo.toml; VERIFIED: Cargo.lock] |
| `schemars` | 1.2.1 | Generate committed JSON Schema for formula/import/report contracts. | Use for `kaipai-formula-bundle.schema.json`, `draft-import-plan.schema.json`, and `adaptation-report.schema.json` if exposed. [VERIFIED: crates/draft_model/Cargo.toml; VERIFIED: cargo search schemars] |
| `ts-rs` | 12.0.1 | Generate TypeScript transport/report types when desktop UI or tests need them. | Use only after a binding/UI surface needs import/report contracts. [VERIFIED: crates/draft_model/Cargo.toml; VERIFIED: cargo search ts-rs] |
| `jsonschema` | 0.46.5 locked; 0.46.6 current registry | Validate fixture JSON against generated schemas. | Reuse locked version unless the planner intentionally updates the lockfile. [VERIFIED: crates/draft_model/Cargo.toml; VERIFIED: Cargo.lock; VERIFIED: cargo search jsonschema] |
| `thiserror` | 2.0.18 | Typed adapter/import/localizer errors. | Use for new import/adapter crate errors, matching existing project boundary crates. [VERIFIED: crates/project_store/Cargo.toml; VERIFIED: package-legitimacy check] |
| `tempfile` | 3.27.0 | Isolated `.veproj` and localizer tests. | Use for resource copy/symlink/path traversal tests. [VERIFIED: crates/project_store/Cargo.toml; VERIFIED: cargo search tempfile] |
| `sha2` | 0.11.0 | SHA-256 validation for external resource evidence. | Use instead of copying the old branch's hand-rolled SHA-256 implementation. [CITED: https://docs.rs/sha2; VERIFIED: cargo search sha2; VERIFIED: package-legitimacy check] |
| `rg` source guards | ripgrep 15.1.0 | Guard against provider/core leakage, remote URL runtime deps, and fallback success. | Existing phase guards use comment-filtered `rg` checks and negative injections. [VERIFIED: rg --version; VERIFIED: scripts/phase16-source-guards.sh] |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Provider-neutral import-plan boundary | Let `adapter_kaipai` mutate `Draft` directly | Direct mutation would violate the locked `DraftImportPlan` decision and hide project-session revision/application semantics. [VERIFIED: 17-CONTEXT.md D-11-D-12] |
| New provider-neutral `AdaptationReport` | Keep old `CompatibilityReport` naming everywhere | Old taxonomy is useful, but Phase 17 needs provider-neutral wording for future Jianying/CapCut adapters. [VERIFIED: 17-CONTEXT.md D-31-D-33] |
| `sha2` crate | Copy old branch custom SHA-256 code | Resource hash verification is easy to get wrong and should not be hand-rolled. [VERIFIED: git show origin/work/kaipai-adapter-poc:crates/adapter_kaipai/src/resource_localizer.rs; CITED: https://docs.rs/sha2] |
| Project-session import API | Renderer sending many `executeProjectIntent` calls | Renderer-side orchestration would make the UI responsible for import semantics and partial failure behavior. [VERIFIED: AGENTS.md; VERIFIED: crates/bindings_node/src/project_session_service.rs] |
| Explicit degraded/native report entries | Smuggle proprietary effects into `Filter.parameters` | Context explicitly requires unsupported effects to be surfaced, not hidden as fake support. [VERIFIED: 17-CONTEXT.md D-03, D-25, D-46] |

**Installation:**

```bash
# Reuse existing locked workspace dependencies for serde/schemars/ts-rs/jsonschema/thiserror/tempfile.
# Add only if the planner implements SHA-256 validation in Rust:
cargo add sha2@0.11.0 -p <provider-neutral-import-crate-or-adapter_kaipai>
```

**Version verification:** `cargo search sha2 --limit 1` returned `sha2 = "0.11.0"` and the package-legitimacy seam returned `OK`. [VERIFIED: cargo search sha2; VERIFIED: package-legitimacy check] Existing workspace dependency versions were read from `Cargo.toml` and `Cargo.lock`. [VERIFIED: crates/draft_model/Cargo.toml; VERIFIED: Cargo.lock]

## Package Legitimacy Audit

This phase can reuse existing workspace dependencies, but SHA-256 validation should use `sha2` instead of copying the old branch's custom hash code. [VERIFIED: Cargo.toml; VERIFIED: git show origin/work/kaipai-adapter-poc:crates/adapter_kaipai/src/resource_localizer.rs; CITED: https://docs.rs/sha2]

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| `sha2` | crates.io | first published 2016-05-06 | 13,813,123/week | https://github.com/RustCrypto/hashes | OK | Approved if SHA-256 validation is implemented in Rust. [VERIFIED: package-legitimacy check; CITED: https://docs.rs/sha2] |
| Existing workspace deps (`serde`, `serde_json`, `schemars`, `ts-rs`, `jsonschema`, `thiserror`, `tempfile`, `blake3`, `rusqlite`) | crates.io | existing locked deps | checked by seam | package legitimacy returned OK | OK | Reuse locked versions; no new install required for these. [VERIFIED: Cargo.lock; VERIFIED: package-legitimacy check] |

**Packages removed due to [SLOP] verdict:** none. [VERIFIED: package-legitimacy check]
**Packages flagged as suspicious [SUS]:** none. [VERIFIED: package-legitimacy check]

## Architecture Patterns

### System Architecture Diagram

```text
Desktop file picker / test fixture path
  -> importOfflineKaipaiFormulaBundle(session_id, expected_revision, bundle_path, source_root)
    -> adapter_kaipai
      -> strict KaipaiFormulaBundle parse and sanitizer validation
      -> provider evidence/provenance retained for report only
    -> resource localizer
      -> copy/validate resources into .veproj/resources
      -> sha256/path/missing/duplicate diagnostics
      -> artifact_store resource index update
    -> provider-neutral DraftImportPlan
      -> validates materials, tracks, segments, timeranges, transforms, text, audio, keyframes
      -> classifies unsupported/degraded features
    -> project_session_service
      -> atomic revisioned Draft mutation
      -> save .veproj/project.json
      -> return view model + AdaptationReport
    -> realtime preview path
      -> renderGraphGpu product evidence
    -> export path
      -> non-empty MP4 + media validation
```

Android worker output and live Kaipai APIs stay outside the runtime path and may be used only as offline calibration/oracle evidence. [VERIFIED: 17-CONTEXT.md D-05, D-43-D-45]

### Recommended Project Structure

```text
crates/
├── adapter_kaipai/        # Offline Kaipai formula contract, sanitizer, mapper to DraftImportPlan. [VERIFIED: 17-CONTEXT.md D-08-D-10]
├── draft_import/          # Provider-neutral DraftImportPlan, AdaptationReport, validation helpers. [VERIFIED: 17-CONTEXT.md D-11-D-13]
├── project_store/         # Reuse bundle path and save/open helpers. [VERIFIED: crates/project_store/src/paths.rs]
├── artifact_store/        # Reuse resource index and project-relative refs. [VERIFIED: crates/artifact_store/src/resource_index.rs]
└── bindings_node/         # Add narrow project-session import API after Rust-side tests. [VERIFIED: crates/bindings_node/src/project_session_service.rs]

fixtures/
└── kaipai/
    ├── positive/
    ├── negative/
    ├── expected-reports/
    └── media/

schemas/
├── kaipai-formula-bundle.schema.json
├── draft-import-plan.schema.json
└── adaptation-report.schema.json
```

The exact provider-neutral crate/module name is flexible, but it must not move provider formula logic into `draft_model`, `engine_core`, `render_graph`, `ffmpeg_compiler`, preview, export, or session semantics. [VERIFIED: 17-CONTEXT.md D-04-D-12]

### Pattern 1: Adapter Emits A Plan, Not Draft Mutations

**What:** The adapter parses raw offline input and emits `DraftImportPlan` plus `AdaptationReport`; it does not save `.veproj/project.json` or mutate live session state. [VERIFIED: 17-CONTEXT.md D-11-D-13]

**When to use:** Use for Kaipai now and future Jianying/CapCut adapters later. [VERIFIED: 17-CONTEXT.md D-32]

**Example:**

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DraftImportPlan {
    pub materials: Vec<ImportMaterialPlan>,
    pub tracks: Vec<ImportTrackPlan>,
    pub segments: Vec<ImportSegmentPlan>,
    pub report: AdaptationReport,
}
```

This is a recommended shape derived from the locked import-plan boundary; the final field set should follow current `draft_model` semantics. [VERIFIED: 17-CONTEXT.md D-11-D-13; VERIFIED: crates/draft_model/src/timeline.rs]

### Pattern 2: Strict JSON Contracts And Sanitized Evidence

**What:** Use `serde` strict structs with `deny_unknown_fields`, schema drift tests, and negative fixtures for unsafe evidence. [VERIFIED: crates/draft_model/src/draft.rs; VERIFIED: git show origin/work/kaipai-adapter-poc:crates/adapter_kaipai/tests/fixtures.rs]

**When to use:** Use for `KaipaiFormulaBundle`, `DraftImportPlan`, `AdaptationReport`, and localized resource manifests. [VERIFIED: 17-CONTEXT.md D-35-D-39]

**Example:**

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KaipaiFormulaBundle {
    pub schema_version: FormulaBundleSchemaVersion,
    pub provenance: FormulaProvenance,
    pub source_media: FormulaSourceMedia,
    pub formula: serde_json::Value,
    pub resources: Vec<FormulaResourceRef>,
}
```

Source pattern: old adapter formula bundle contract. [VERIFIED: git show origin/work/kaipai-adapter-poc:crates/adapter_kaipai/src/formula_bundle.rs]

### Pattern 3: Resource Localization Before Draft Application

**What:** Copy or validate resources into `.veproj/resources`, reject unsafe paths/remote render URLs, validate hashes, then use project-relative material/font refs in the import plan. [VERIFIED: 17-CONTEXT.md D-14-D-16]

**When to use:** Run this before any mapped segment refers to template media, sticker, font, image, PIP, or BGM resources. [VERIFIED: 17-CONTEXT.md D-36-D-37]

**Example:**

```rust
pub fn classify_material_uri(
    bundle_path: impl AsRef<Path>,
    uri: &str,
) -> Result<MaterialUri, ProjectStoreError> {
    validate_bundle_relative_path(trimmed)?;
    Ok(MaterialUri {
        kind: MaterialUriKind::InBundleRelative,
        uri: trimmed.to_owned(),
        resolved_path: Some(bundle_path.as_ref().join(path)),
    })
}
```

Source pattern: existing project-store URI classification and traversal rejection. [VERIFIED: crates/project_store/src/paths.rs]

### Pattern 4: Session-Owned Atomic Application

**What:** Add a Rust project-session API that checks `session_id` and `expected_revision`, applies the import plan atomically, increments revision, saves the bundle, and returns a view model plus report. [VERIFIED: crates/bindings_node/src/project_session_service.rs]

**When to use:** Use after adapter/localizer/import-plan unit tests are green and before desktop UI work. [VERIFIED: 17-CONTEXT.md D-38-D-40]

**Example:**

```rust
pub fn execute_project_intent(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = serde_json::from_value::<ExecuteProjectIntentRequest>(request)
        .map_err(|error| invalid_payload(error, "executeProjectIntent"))?;
    with_project_session_registry(|registry| registry.execute_intent(request))
}
```

Source pattern: current project-session command boundary. [VERIFIED: crates/bindings_node/src/project_session_service.rs]

### Anti-Patterns to Avoid

- **Merging the old branch integration directly:** It predates current session/resource/preview/scheduler/no-fallback architecture and must be rewritten. [VERIFIED: 17-CONTEXT.md D-08-D-10]
- **Raw formula in `.veproj/project.json`:** Raw Kaipai JSON may be input/provenance/report evidence, not canonical render semantics. [VERIFIED: 17-CONTEXT.md D-04-D-06, D-42]
- **Remote template URLs as runtime dependencies:** Localize first; imported drafts must not require remote render URLs. [VERIFIED: 17-CONTEXT.md D-14-D-16, D-42]
- **Kaipai-specific safe-area logic in core/render:** Only generic text safe-area semantics may remain canonical. [VERIFIED: 17-CONTEXT.md D-05-D-07]
- **Provider-native effect smuggling:** Native effects must be reported as `needsNativeEffect` or degraded. [VERIFIED: 17-CONTEXT.md D-25, D-46]
- **UI-first implementation:** The desktop import entry/report panel comes after backend import/report/localization/preview/export gates. [VERIFIED: 17-CONTEXT.md D-35-D-40]
- **Export parity claims for rotation without closing the gap:** Current FFmpeg compiler tests still expect rotation diagnostics as unsupported. [VERIFIED: crates/ffmpeg_compiler/tests/transform_snapshots.rs]

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| JSON parsing and schema validation | Ad hoc string walkers over provider JSON | `serde`, `serde_json`, `schemars`, `jsonschema` | Existing contracts are Rust-owned, strict, and drift-tested. [VERIFIED: crates/draft_model/tests/schema_exports.rs; VERIFIED: git show origin/work/kaipai-adapter-poc:crates/adapter_kaipai/tests/schema_exports.rs] |
| SHA-256 validation | Copy old branch custom SHA-256 code | `sha2::Sha256` | Hash code is security-sensitive and old branch hand-rolled it only because no package was added. [VERIFIED: git show origin/work/kaipai-adapter-poc:crates/adapter_kaipai/src/resource_localizer.rs; CITED: https://docs.rs/sha2] |
| Path safety | Manual `contains("..")` checks | `project_store::classify_material_uri` style helpers plus canonicalization | Current helpers reject parent traversal and classify bundle-relative refs. [VERIFIED: crates/project_store/src/paths.rs] |
| Draft application | Adapter writes `.veproj/project.json` directly | Project-session service applies `DraftImportPlan` | Session owns revision, validation, save, and view-model responses. [VERIFIED: crates/bindings_node/src/project_session_service.rs] |
| Resource index | Parallel manifest detached from artifact store | `artifact_store::resource_index` | Existing index covers material/font/effect/filter/transition and project-relative refs. [VERIFIED: crates/artifact_store/src/resource_index.rs] |
| Preview/export evidence | Android oracle, old artifacts, mock frames, CPU readback | Realtime render-graph GPU preview and normal export path | Product success cannot be fallback or old branch evidence. [VERIFIED: docs/no-product-fallback-policy.md; VERIFIED: 17-CONTEXT.md D-43-D-45] |

**Key insight:** The hard part is not parsing Kaipai JSON; it is preserving the editor's ownership boundaries while turning an external template into a normal, editable, previewable, exportable `.veproj`. [VERIFIED: AGENTS.md; VERIFIED: 17-CONTEXT.md D-01-D-13]

## Common Pitfalls

### Pitfall 1: Copying Old Adapter Integration Directly
**What goes wrong:** Old branch code reintroduces stale assumptions and bypasses current project-session/resource/preview architecture. [VERIFIED: 17-CONTEXT.md D-08-D-10]
**Why it happens:** The old branch has passing tests and useful contracts, so it can look merge-ready. [VERIFIED: git show origin/work/kaipai-adapter-poc:.planning/phases/03.1-kaipai-compatibility-foundation-offline-formula-fixtures-com/03.1-VERIFICATION.md]
**How to avoid:** Port fixtures/contracts/test ideas first; rewrite import application through current main. [VERIFIED: 17-CONTEXT.md D-34-D-40]
**Warning signs:** Adapter writes project JSON, calls session internals directly, or imports provider types from core/render crates. [VERIFIED: 17-CONTEXT.md D-41-D-42]

### Pitfall 2: Reporting Approximation As Support
**What goes wrong:** Users see a successful import even though native effects, text effects, or missing resources were dropped. [VERIFIED: 17-CONTEXT.md D-03, D-25, D-46]
**Why it happens:** A mapper can create a visually plausible draft while losing unsupported semantics. [VERIFIED: git show origin/work/kaipai-adapter-poc:fixtures/kaipai/expected-reports/native-effect-needs-native-effect.report.json]
**How to avoid:** Snapshot report entries for supported, approximated, dropped, missing resource, and native effect fixtures. [VERIFIED: 17-CONTEXT.md D-31-D-33]
**Warning signs:** Generic `Filter.parameters` carries opaque native effect IDs or report summary counts stay zero for degraded fixtures. [VERIFIED: 17-CONTEXT.md D-25-D-46]

### Pitfall 3: Resource Localization After Draft Mapping
**What goes wrong:** The draft can point at remote URLs, missing files, unsafe paths, or unverified resources. [VERIFIED: 17-CONTEXT.md D-14-D-16]
**Why it happens:** Mapping fields first is simpler than establishing a bundle resource manifest first. [VERIFIED: git show origin/work/kaipai-adapter-poc:crates/adapter_kaipai/src/resource_localizer.rs]
**How to avoid:** Localize before `DraftImportPlan` application and block runtime dependency on remote template URLs. [VERIFIED: 17-CONTEXT.md D-36-D-37]
**Warning signs:** `.veproj/project.json` contains `http://`, `https://`, signed URLs, or paths outside `.veproj/resources`. [VERIFIED: 17-CONTEXT.md D-42]

### Pitfall 4: Preview/Export Divergence On Rotation
**What goes wrong:** Imported rotated overlays look correct in realtime preview but export without rotation or with diagnostics. [VERIFIED: crates/realtime_preview_runtime/src/gpu/compositor.rs; VERIFIED: crates/ffmpeg_compiler/tests/transform_snapshots.rs]
**Why it happens:** Current GPU compositor has center-anchor rotation geometry, while current FFmpeg compiler snapshots still classify rotation as unsupported. [VERIFIED: crates/realtime_preview_runtime/src/gpu/compositor.rs; VERIFIED: crates/ffmpeg_compiler/tests/transform_snapshots.rs]
**How to avoid:** Add a generic export rotation parity task before marking rotated imports supported, or classify rotation as approximated/degraded in the report. [VERIFIED: 17-CONTEXT.md D-26-D-28]
**Warning signs:** Fixture export tests assert only non-empty MP4 and skip visual rotation/layer ordering evidence. [VERIFIED: 17-CONTEXT.md D-44-D-45]

### Pitfall 5: UI Entry Before Backend Gates
**What goes wrong:** Users can click an import template action that depends on incomplete backend behavior or fallback evidence. [VERIFIED: docs/product-e2e-acceptance-policy.md; VERIFIED: docs/no-product-fallback-policy.md]
**Why it happens:** The visible feature is tempting to build first. [VERIFIED: 17-CONTEXT.md D-40]
**How to avoid:** Plan backend import/report/localization/project-session/preview/export gates first, then UI entry/report panel. [VERIFIED: 17-CONTEXT.md D-35-D-40]
**Warning signs:** Playwright tests check only report UI text or file existence without preview/export product evidence. [VERIFIED: docs/product-e2e-acceptance-policy.md]

## Code Examples

### Strict Bundle Contract

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KaipaiFormulaBundle {
    pub schema_version: FormulaBundleSchemaVersion,
    pub kind: FormulaBundleKind,
    pub provenance: FormulaProvenance,
    pub source_media: FormulaSourceMedia,
    pub recognizer_result: RecognizerResult,
    pub safe_area: SafeAreaEvidence,
    pub direct_materials: Vec<DirectMaterialRef>,
    pub formula: serde_json::Value,
    pub resources: Vec<FormulaResourceRef>,
}
```

Source: old branch `adapter_kaipai` formula contract. [VERIFIED: git show origin/work/kaipai-adapter-poc:crates/adapter_kaipai/src/formula_bundle.rs]

### Resource Index Hook

```rust
for material in &draft.materials {
    let resource_ref = resource_ref_for_material(material.material_id.as_str());
    let classified = classify_material_uri(bundle_path, &material.uri)?;
    let project_relative_ref = match classified.kind {
        MaterialUriKind::InBundleRelative => Some(classified.uri),
        MaterialUriKind::ExternalAbsolute | MaterialUriKind::ExternalUri => None,
    };
    upsert_indexed_resource(&mut index, IndexedResource { /* fields */ })?;
}
```

Source: current artifact resource indexing pattern. [VERIFIED: crates/artifact_store/src/resource_index.rs]

### SHA-256 Validation Without Custom Hash Code

```rust
use sha2::{Digest, Sha256};

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
```

Source: `sha2` documentation exposes `Sha256` through the `Digest` trait. [CITED: https://docs.rs/sha2; CITED: https://docs.rs/digest]

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Kaipai/dcoin flow used Android worker output as black-box render/oracle evidence. | Phase 17 treats Kaipai as an offline input adapter and uses Video Editor preview/export for product evidence. | Locked in Phase 17 context on 2026-06-24. [VERIFIED: 17-CONTEXT.md] | Planner must not include Android runtime, live Kaipai API, or oracle output in product success gates. [VERIFIED: 17-CONTEXT.md D-43-D-45] |
| Old branch compatibility foundation stopped at formula/report/localizer/gap inventory. | Current Phase 17 must apply mapped drafts through project session and prove preview/export. | Current main after Phases 7-16. [VERIFIED: .planning/STATE.md; VERIFIED: crates/bindings_node/src/project_session_service.rs] | Old fixtures/contracts are useful, but integration must be rewritten. [VERIFIED: 17-CONTEXT.md D-08-D-10] |
| Old gap inventory listed canvas, transform, font resources, stickers, and keyframes as missing. | Current main has canvas config, transforms, text styles, bundled font refs, audio fades, keyframes, resource index, realtime preview, and scheduler. | Phases 7-16 completed before Phase 17. [VERIFIED: .planning/REQUIREMENTS.md; VERIFIED: crates/draft_model/src/canvas.rs; VERIFIED: crates/draft_model/src/timeline.rs] | Planner should not re-plan solved generic semantics; it should audit remaining gaps only. [VERIFIED: 17-CONTEXT.md D-26] |
| Old localizer hand-rolled SHA-256. | Use `sha2` for SHA-256 if Rust hash validation is implemented. | Phase 17 research. [VERIFIED: git show origin/work/kaipai-adapter-poc:crates/adapter_kaipai/src/resource_localizer.rs; CITED: https://docs.rs/sha2] | Avoid copying security-sensitive custom hash code. [VERIFIED: package-legitimacy check] |

**Deprecated/outdated:**
- Old `CompatibilityReport` naming is useful source material but should become or feed provider-neutral `AdaptationReport`. [VERIFIED: 17-CONTEXT.md D-31-D-33]
- Old Draft v2 gap inventory is partially stale against current main and should not drive reimplementation of existing canvas/transform/text/audio/keyframe/resource features. [VERIFIED: git show origin/work/kaipai-adapter-poc:crates/adapter_kaipai/tests/draft_v2_gap_inventory.rs; VERIFIED: crates/draft_model/src/timeline.rs]
- Product success via Android oracle, artifact fallback, mock, CPU readback, or old branch output is invalid. [VERIFIED: docs/no-product-fallback-policy.md; VERIFIED: 17-CONTEXT.md D-43-D-45]

## Assumptions Log

All factual claims in this research were verified from project files, old branch git inspection, local tool output, package-legitimacy output, or cited official crate documentation. [VERIFIED: codebase grep; VERIFIED: git show origin/work/kaipai-adapter-poc; CITED: https://docs.rs/sha2]

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| - | None. | - | - |

## Open Questions

1. **Should Phase 17 implement generic static export rotation parity or classify rotated imports as degraded?** [VERIFIED: 17-CONTEXT.md D-26-D-28]
   - What we know: GPU preview has center-anchor rotation math, while FFmpeg compiler tests still expect rotation to be unsupported. [VERIFIED: crates/realtime_preview_runtime/src/gpu/compositor.rs; VERIFIED: crates/ffmpeg_compiler/tests/transform_snapshots.rs]
   - What's unclear: Whether the planner wants to spend Phase 17 scope closing this export gap or report rotation as approximated/degraded. [VERIFIED: 17-CONTEXT.md D-02, D-26-D-28]
   - Recommendation: Add a generic static rotation export parity task before claiming rotation support; keep animated rotation as degraded unless a later retiming/effects plan expands it. [VERIFIED: crates/ffmpeg_compiler/tests/transform_snapshots.rs; VERIFIED: 17-CONTEXT.md D-23-D-28]

2. **Where should provider-neutral report schemas live?** [VERIFIED: 17-CONTEXT.md D-31-D-33]
   - What we know: `draft_model` owns canonical draft semantics, and context allows report types shared at the import boundary. [VERIFIED: AGENTS.md; VERIFIED: 17-CONTEXT.md D-04, D-31-D-33]
   - What's unclear: Whether to create a new import crate or place report transport types in an existing schema-generation crate. [VERIFIED: crates/draft_model/tests/schema_exports.rs]
   - Recommendation: Prefer a new provider-neutral import crate/module that depends on `draft_model`, with its own schema export tests. [VERIFIED: 17-CONTEXT.md D-11-D-13]

3. **Which real Kaipai fixtures can be safely committed after sanitization?** [VERIFIED: 17-CONTEXT.md D-16, D-39]
   - What we know: Old branch fixtures are sanitized and deterministic, and old tests check credential-like fields. [VERIFIED: git show origin/work/kaipai-adapter-poc:fixtures/kaipai/positive/sanitized-formula-bundle.json; VERIFIED: git show origin/work/kaipai-adapter-poc:crates/adapter_kaipai/tests/fixtures.rs]
   - What's unclear: Whether there are newer real formulas/resources beyond the old branch corpus that should become Phase 17 goldens. [VERIFIED: 17-CONTEXT.md D-39]
   - Recommendation: Start with old fixtures, then require a human sanitization checkpoint before adding any newer real-provider samples. [VERIFIED: 17-CONTEXT.md D-16]

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| Rust | Rust crates/tests | yes | rustc 1.95.0 | Blocking if missing. [VERIFIED: rustc --version; VERIFIED: Cargo.toml] |
| Cargo | Workspace builds/tests/dependency add | yes | cargo 1.95.0 | Blocking if missing. [VERIFIED: cargo --version] |
| Node.js | Electron/pnpm scripts | yes | v24.15.0 | Package engine pins 24.12.0; use current if scripts pass, otherwise align with engine. [VERIFIED: node --version; VERIFIED: package.json] |
| pnpm | Desktop scripts/tests | yes | 10.32.1 | Blocking if missing. [VERIFIED: pnpm --version; VERIFIED: package.json] |
| ripgrep | Source guards | yes | 15.1.0 | Blocking for guard scripts. [VERIFIED: rg --version; VERIFIED: scripts/phase16-source-guards.sh] |
| Git | Old branch asset reads and phase commits | yes | 2.50.1 | Blocking for old-branch inspection. [VERIFIED: git --version] |
| Bundled FFmpeg | Export fixture validation | yes | 8.1.2 | No product fallback; missing bundled runtime blocks export evidence. [VERIFIED: apps/desktop-electron/runtime/ffmpeg/darwin-arm64/ffmpeg -version; VERIFIED: docs/runtime-boundaries.md] |
| Bundled ffprobe | Export/media validation | yes | 8.1.2 | No product fallback; missing bundled runtime blocks media evidence. [VERIFIED: apps/desktop-electron/runtime/ffmpeg/darwin-arm64/ffprobe -version; VERIFIED: docs/runtime-boundaries.md] |

**Missing dependencies with no fallback:** none found in this session. [VERIFIED: environment probes]

**Missing dependencies with fallback:** none found in this session. [VERIFIED: environment probes]

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust `cargo test` plus pnpm shell source guards and Playwright/Electron product tests. [VERIFIED: package.json] |
| Config file | `Cargo.toml`, `package.json`, Playwright config under desktop app. [VERIFIED: Cargo.toml; VERIFIED: package.json; VERIFIED: apps/desktop-electron/tests] |
| Quick run command | `pnpm run test:phase17-rust && pnpm run test:phase17-source-guards` should be added in Wave 0. [VERIFIED: package.json current absence of phase17 scripts] |
| Full suite command | `pnpm run test:phase17 && cargo check --workspace --locked && pnpm run test:contracts` should be added before closeout. [VERIFIED: package.json current phase gate pattern] |

### Phase Requirements -> Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| COMP-01 | Offline Kaipai bundle maps to a valid `.veproj/project.json` through `DraftImportPlan` and project session. [VERIFIED: .planning/REQUIREMENTS.md; VERIFIED: 17-CONTEXT.md D-11-D-13] | Rust integration | `cargo test -p bindings_node project_session_import_kaipai -- --nocapture` | No - Wave 0. |
| COMP-02 | `AdaptationReport` snapshots cover supported, approximated, dropped, missingResource, and needsNativeEffect. [VERIFIED: .planning/REQUIREMENTS.md; VERIFIED: 17-CONTEXT.md D-31-D-33] | Rust snapshot/schema | `cargo test -p draft_import adaptation_report -- --nocapture` | No - Wave 0. |
| PRODFX-05 | Main video, PIP, text sticker, BGM/audio, missing resource, and native effect fixtures produce expected reports and render/export evidence. [VERIFIED: .planning/REQUIREMENTS.md; VERIFIED: 17-CONTEXT.md D-39-D-46] | Rust + export smoke | `cargo test -p testkit template_import_exports -- --nocapture` | No - Wave 0. |
| NO-FALLBACK-01 | Preview/export success cannot use Android runtime, old artifacts, mock, CPU readback, or remote URLs. [VERIFIED: docs/no-product-fallback-policy.md; VERIFIED: 17-CONTEXT.md D-43-D-45] | Source guard + product evidence | `pnpm run test:no-product-fallback && pnpm run test:phase17-source-guards` | Partial - existing no-fallback guard exists; phase17 guard missing. |
| TEST-E2E-01 | If UI import/report panel is added, a user imports a fixture and sees report plus preview/export evidence. [VERIFIED: docs/product-e2e-acceptance-policy.md; VERIFIED: 17-CONTEXT.md D-40] | Playwright/Electron | `pnpm --filter @video-editor/desktop exec playwright test tests/template-import.spec.ts --reporter=line` | No - only after UI wave. |

### Sampling Rate

- **Per task commit:** Run the focused Rust test for the touched crate plus `pnpm run test:phase17-source-guards`. [VERIFIED: package.json phase gate pattern]
- **Per wave merge:** Run `pnpm run test:phase17-rust`, `pnpm run test:contracts`, and any added Playwright subset. [VERIFIED: package.json phase gate pattern]
- **Phase gate:** Run `pnpm run test:phase17`, `pnpm run test:no-product-fallback`, `cargo check --workspace --locked`, and export fixture validation before `$gsd-verify-work`. [VERIFIED: docs/no-product-fallback-policy.md; VERIFIED: package.json]

### Wave 0 Gaps

- [ ] `crates/draft_import/` or equivalent provider-neutral import module with `DraftImportPlan` and `AdaptationReport` contracts. [VERIFIED: 17-CONTEXT.md D-11-D-13]
- [ ] `crates/adapter_kaipai/` ported from old branch as source material, not direct merge. [VERIFIED: 17-CONTEXT.md D-08-D-10]
- [ ] `fixtures/kaipai/` current-main fixture corpus: main video, PIP, text sticker, BGM/audio, missing resource, native effect degradation. [VERIFIED: 17-CONTEXT.md D-33, D-39]
- [ ] `scripts/phase17-source-guards.sh` with Kaipai/raw-formula/live-provider/Android/remote URL/no-fallback negative checks. [VERIFIED: 17-CONTEXT.md D-41-D-45]
- [ ] `package.json` scripts `test:phase17-rust`, `test:phase17-source-guards`, `test:phase17-export-fixtures`, and `test:phase17`. [VERIFIED: package.json]
- [ ] Generated schema/TypeScript contract drift tests for any new import/report surfaces. [VERIFIED: crates/draft_model/tests/schema_exports.rs]

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | no | Live provider auth is out of scope. [VERIFIED: 17-CONTEXT.md Deferred Ideas] |
| V3 Session Management | no | Offline import does not establish provider sessions. [VERIFIED: 17-CONTEXT.md Deferred Ideas] |
| V4 Access Control | yes | Project-session import requires `session_id` plus `expected_revision`, matching existing session mutation patterns. [VERIFIED: crates/bindings_node/src/project_session_service.rs] |
| V5 Input Validation | yes | Strict serde contracts, JSON Schema fixtures, path canonicalization, hash validation, and source guards. [VERIFIED: git show origin/work/kaipai-adapter-poc:crates/adapter_kaipai/tests/fixtures.rs; VERIFIED: crates/project_store/src/paths.rs] |
| V6 Cryptography | yes | Use `sha2` for SHA-256 resource evidence; do not hand-roll hash code. [CITED: https://docs.rs/sha2; VERIFIED: package-legitimacy check] |
| V8 Data Protection | yes | No tokens, signed URLs, cookies, account IDs, or credentials in fixtures or reports. [VERIFIED: 17-CONTEXT.md D-16; VERIFIED: git show origin/work/kaipai-adapter-poc:crates/adapter_kaipai/tests/fixtures.rs] |
| V12 File and Resources | yes | Localizer must reject traversal, symlink escapes, duplicate destinations, remote render URLs, and unsafe paths. [VERIFIED: git show origin/work/kaipai-adapter-poc:crates/adapter_kaipai/tests/resource_localizer.rs] |

### Known Threat Patterns for Offline Template Import

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Path traversal in resource URIs | Tampering | Canonicalize source/bundle paths, reject parent traversal and absolute/remote refs, test symlink escapes. [VERIFIED: crates/project_store/src/paths.rs; VERIFIED: git show origin/work/kaipai-adapter-poc:crates/adapter_kaipai/tests/resource_localizer.rs] |
| Remote template URL runtime dependency | Information disclosure / Tampering | Sanitizer and source guard reject `http://`, `https://`, signed URLs, and remote render URL fields in runtime drafts. [VERIFIED: 17-CONTEXT.md D-14-D-16, D-42] |
| Credential leakage in fixtures | Information disclosure | Fixture tests reject credential-like keys and committed data scans block tokens/cookies/account IDs. [VERIFIED: git show origin/work/kaipai-adapter-poc:crates/adapter_kaipai/tests/fixtures.rs] |
| Raw provider formula semantics in core/render | Tampering / Repudiation | Source guards block provider terms in `draft_model`, `engine_core`, `render_graph`, `ffmpeg_compiler`, and canonical draft schemas. [VERIFIED: 17-CONTEXT.md D-41] |
| Unsupported native effects hidden as support | Repudiation | `AdaptationReport` snapshots require native effect and dropped/degraded classifications. [VERIFIED: 17-CONTEXT.md D-31-D-33, D-46] |
| Stale session mutation | Tampering | Import API should require `expected_revision` and fail stale revisions like existing project-session reads/mutations. [VERIFIED: crates/bindings_node/src/project_session_service.rs] |

## Sources

### Primary (HIGH confidence)

- `.planning/phases/17-template-import-core-and-kaipai-offline-adapter-foundation/17-CONTEXT.md` - locked decisions, target chain, scope, gates. [VERIFIED: local read]
- `AGENTS.md` and `.agents/skills/production-architecture-review/SKILL.md` - project constraints and production architecture review posture. [VERIFIED: local read]
- `.planning/REQUIREMENTS.md`, `.planning/STATE.md`, `.planning/ROADMAP.md`, `.planning/PROJECT.md` - current requirements and phase history. [VERIFIED: local read]
- `docs/no-product-fallback-policy.md`, `docs/refactor-and-legacy-cleanup-policy.md`, `docs/product-e2e-acceptance-policy.md`, `docs/runtime-boundaries.md` - mandatory product evidence and boundary policies. [VERIFIED: local read]
- Current main code: `draft_model`, `project_store`, `artifact_store`, `bindings_node`, `engine_core`, `render_graph`, `ffmpeg_compiler`, `realtime_preview_runtime`, `task_runtime`. [VERIFIED: codebase grep]
- Old branch assets via `git show origin/work/kaipai-adapter-poc`: `crates/adapter_kaipai/`, `fixtures/kaipai/`, schemas, spike skill, Phase 03.1 artifacts. [VERIFIED: git show]

### Secondary (MEDIUM confidence)

- `https://docs.rs/sha2` and `https://docs.rs/digest` - official crate API/source documentation for SHA-256 hashing. [CITED: docs.rs]
- Package-legitimacy seam for crates: `sha2`, `serde`, `serde_json`, `schemars`, `ts-rs`, `jsonschema`, `tempfile`, `thiserror`, `blake3`, `rusqlite` returned `OK`. [VERIFIED: package-legitimacy check]
- `cargo search` for crate current versions. [VERIFIED: cargo search]

### Tertiary (LOW confidence)

- Websearch routed by the GSD research seam was not authoritative for repo-specific ownership questions and was not used for architecture decisions. [VERIFIED: research-store put; LOW: websearch]

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH for existing workspace crates and toolchain; MEDIUM for adding `sha2` because it is verified but still a new dependency decision. [VERIFIED: Cargo.toml; VERIFIED: package-legitimacy check; CITED: https://docs.rs/sha2]
- Architecture: HIGH because target chain, boundaries, and old-branch reuse rules are locked in context and confirmed in code. [VERIFIED: 17-CONTEXT.md; VERIFIED: codebase grep]
- Pitfalls: HIGH for old-branch merge risk, raw provider leakage, resource safety, and no-fallback evidence; MEDIUM for exact fixture field coverage beyond sanitized old samples. [VERIFIED: 17-CONTEXT.md; VERIFIED: git show origin/work/kaipai-adapter-poc]

**Research date:** 2026-06-24 [VERIFIED: current_date]
**Valid until:** 2026-07-24 for repository-bound architecture; re-check crate versions and tool availability before implementation. [VERIFIED: cargo search; VERIFIED: environment probes]
