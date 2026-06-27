---
phase: 02-draft-and-material-system
verified: 2026-06-17T04:50:31Z
status: passed
score: 29/29 must-haves verified
overrides_applied: 0
---

# Phase 2: Draft And Material System Verification Report

**Phase Goal:** Establish `.veproj` drafts, Jianying-aligned schema concepts, material import/probing, and save/open integrity.
**Verified:** 2026-06-17T04:50:31Z
**Status:** passed
**Re-verification:** No - initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | User can create, save, close, and reopen a `.veproj` draft without semantic changes. | VERIFIED | `project_store` creates/saves/opens `.veproj/project.json` through `create_project_bundle`, `save_project_bundle`, `open_project_bundle`, and `autosave_project_bundle` in `crates/project_store/src/bundle.rs:22`, `:30`, `:75`; save validates via `draft_model` at `:38`, and open migrates/validates at `:92`. Spot-check `cargo test -p project_store round_trip -- --nocapture` passed: 2/2 tests. |
| 2 | Draft schema uses Jianying terms consistently across Rust model, schema, commands, tests, and docs. | VERIFIED | Rust model defines `Draft`, `Material`, `Track`, `Segment`, `SourceTimerange`, `TargetTimerange`, `MainTrackMagnet`, `Keyframe`, `Filter`, and `Transition` in `crates/draft_model/src/draft.rs:43` and `crates/draft_model/src/timeline.rs:21-125`; generated `Draft.ts` exposes the same names at `apps/desktop-electron/src/generated/Draft.ts:3-24`; source guard `pnpm run test:phase2-source-guards` passed with no `Asset`/`Clip` or float-second drift. |
| 3 | User can import video, image, and audio materials and see probed metadata in the material bin. | VERIFIED | `media_runtime::probe_material_metadata` normalizes ffprobe output in `crates/media_runtime/src/probe.rs:117`; generated fixture tests cover video/image/audio in `crates/media_runtime/tests/material_probe.rs:19`, `:37`, `:55`. `material_service` imports and saves materials through probe/path/registry/project-store wiring in `crates/bindings_node/src/material_service.rs:128-230`, and service tests cover all three material kinds at `crates/bindings_node/tests/material_service.rs:17`. Electron smoke calls generated `listMaterials` and renders duration/stream/status in `apps/desktop-electron/src/renderer/App.tsx:75`, `:111`, `:196-205`; Playwright asserts the row at `apps/desktop-electron/tests/electron-smoke.spec.ts:132-137`. |
| 4 | Missing material detection surfaces a recoverable state without corrupting the draft. | VERIFIED | Missing imports create `MaterialStatus::Missing` records and diagnostics instead of deleting material records in `crates/bindings_node/src/material_service.rs:151-170` and `:239-280`; diagnostics preserve original URI and resolved path at `:390-423`. Spot-check `cargo test -p bindings_node execute_command_reports_missing_material_diagnostics_without_corrupting_draft -- --nocapture` passed: 1/1 test. |
| 5 | Draft schema uses Jianying-aligned names directly. | VERIFIED | See Truth 2; `draft_model`/schema/generated contracts use the required names and source guard rejects `Asset`/`Clip`. |
| 6 | Persisted draft time values are integer microseconds or rational structures. | VERIFIED | `Microseconds` is the persisted time wrapper and `RationalFrameRate` is `{ numerator, denominator }` in `crates/draft_model/src/time.rs` and `crates/draft_model/src/material.rs:26-39`; generated `Draft.ts` exposes `Microseconds = number` and `RationalFrameRate` at `apps/desktop-electron/src/generated/Draft.ts:7`, `:10`; source guard passed for float seconds. |
| 7 | Schema version 1 loads through migration hooks and future versions fail recoverably. | VERIFIED | `migrate_draft_json` checks `schemaVersion`, accepts current version, and returns `InvalidSchemaVersion` for others at `crates/draft_model/src/validation.rs:61-85`; negative fixture test covers schema version failure at `crates/draft_model/tests/draft_fixtures.rs:75`. |
| 8 | Persisted `Draft` excludes derived thumbnails, waveforms, caches, render graphs, FFmpeg scripts, exports, and raw probe JSON. | VERIFIED | `reject_derived_artifact_fields` rejects derived top-level fields at `crates/draft_model/src/validation.rs:188-209`; generated/schema/source guard passed and negative fixture includes `renderGraph` rejection. |
| 9 | A new `.veproj` bundle contains valid `project.json`. | VERIFIED | `create_project_bundle` delegates to `save_project_bundle`, which writes only `project.json`, at `crates/project_store/src/bundle.rs:22-54`; tests cover create/open at `crates/project_store/tests/project_bundle.rs:12`. |
| 10 | Save/reopen preserves semantic equality. | VERIFIED | Round-trip tests compare `Draft` semantic equality in `crates/project_store/tests/project_bundle.rs`; spot-check passed. |
| 11 | `project_store` owns persistence/path classification only, not material import semantics. | VERIFIED | `project_store` uses `validate_draft`/`migrate_draft_json` and path helpers only; grep for `probe_material_metadata`, `FfmpegExecutor`, `ffprobe`, `ffmpeg`, and registry mutation helpers in `crates/project_store/src` returned no matches. |
| 12 | Relative bundle paths and external URIs are classified centrally. | VERIFIED | `classify_material_uri`, `resolve_material_uri`, and `material_uri_for_save` are in `crates/project_store/src/paths.rs:25`, `:59`, `:66`; project-store path tests are wired into root gates. |
| 13 | Video/image/audio can be probed through Rust `media_runtime`. | VERIFIED | Generated fixture probe tests cover all three media types in `crates/media_runtime/tests/material_probe.rs:19`, `:37`, `:55`. |
| 14 | Normalized probe output captures material metadata and classified errors. | VERIFIED | `MaterialProbeMetadata` includes kind, duration microseconds, dimensions, frame rate, stream flags, and audio fields in `crates/media_runtime/src/probe.rs:40-51`; error kinds are defined at `:54-65`; malformed JSON/frame-rate tests at `crates/media_runtime/tests/material_probe.rs:126`. |
| 15 | `media_runtime` does not persist drafts or mutate material registry. | VERIFIED | Source scan for `project_store`, `MaterialId`, `save_project_bundle`, and `create_project_bundle` in `crates/media_runtime/src` returned no matches. |
| 16 | Raw ffprobe JSON and generated media stay out of semantic draft state. | VERIFIED | Probe code returns typed `MaterialProbeMetadata`, fixtures are temp-dir-backed in `crates/testkit/src/lib.rs:120`, `:249`, `:299`, `:338`, and source/fixture guards passed. |
| 17 | User-selected materials import through a Rust-owned binding-facing service. | VERIFIED | `import_material` and `import_material_and_save` live in `crates/bindings_node/src/material_service.rs:128` and `:220`; tests import video/image/audio at `crates/bindings_node/tests/material_service.rs:17`. |
| 18 | Imported materials persist stable IDs, URI, duration, dimensions, fps, streams, audio metadata, and status. | VERIFIED | Service maps probe metadata to `Material` at `crates/bindings_node/src/material_service.rs:295-327`; deterministic ID helper exists at `:372`; tests assert IDs/status/duration/audio metadata at `crates/bindings_node/tests/material_service.rs:54-88`. |
| 19 | Missing materials remain in draft and return classified diagnostics. | VERIFIED | See Truth 4; service and binding smoke tests verify missing material preservation and diagnostics. |
| 20 | Material import orchestration is outside `project_store`. | VERIFIED | Service coordinates `material_uri_for_save`, `probe_material_metadata`, registry helpers, and `save_project_bundle` in `crates/bindings_node/src/material_service.rs:12-16`, `:128-230`; project-store grep guard passed. |
| 21 | Thumbnail/cache policy is documented as derived-artifact boundary. | VERIFIED | `docs/runtime-boundaries.md:78-95` states `.veproj/project.json` is canonical semantic state, and thumbnails/waveforms/raw probe JSON/preview caches/render artifacts remain derived outputs outside it. |
| 22 | Electron can request import/list/missing-material behavior through generated Rust-owned contracts. | VERIFIED | Rust command payload/result types are in `crates/draft_model/src/lib.rs:50-68`, `:132-206`; generated TS includes material command payloads/results at `apps/desktop-electron/src/generated/CommandEnvelope.ts:9-12` and `CommandResultEnvelope.ts:9-13`. |
| 23 | Binding handlers call `material_service` and Electron does not construct FFmpeg/ffprobe commands. | VERIFIED | `execute_command` routes material commands to service functions in `crates/bindings_node/src/lib.rs:88-167`; renderer source guard for `ffmpeg|ffprobe` passed and Playwright includes a renderer-source assertion at `apps/desktop-electron/tests/electron-smoke.spec.ts:144`. |
| 24 | Electron smoke displays material metadata through generated contracts. | VERIFIED | App calls `listMaterials` with generated `CommandEnvelope` and renders `MaterialRow` in `apps/desktop-electron/src/renderer/App.tsx:75-111`, `:196-205`; Playwright asserts display name, kind, duration, dimensions, and status at `apps/desktop-electron/tests/electron-smoke.spec.ts:132-137`. |
| 25 | Missing diagnostics return through standardized command envelopes. | VERIFIED | `CommandResultEnvelope` error/data shape includes missing diagnostics in generated TS; binding smoke asserts `listMissingMaterials` envelope and draft preservation at `crates/bindings_node/tests/binding_smoke.rs:150`. |
| 26 | Positive and negative `.veproj/project.json` fixtures cover draft/material behaviors. | VERIFIED | Six project fixtures exist under `fixtures/draft/positive` and `fixtures/draft/negative`; classifier and positive/negative tests live in `crates/draft_model/tests/draft_fixtures.rs:18`, `:39`, `:57`, `:75`. Spot-check `cargo test -p draft_model draft_fixtures -- --nocapture` passed: 4/4 tests. |
| 27 | Every Phase 2 requirement ID is covered by automated tests or committed fixtures/contracts/smoke tests. | VERIFIED | Requirement coverage table below maps DRAFT-01 through MAT-04 to code/tests. All nine IDs appear in plan frontmatter and `.planning/REQUIREMENTS.md` traceability. |
| 28 | Generated schema and TypeScript artifacts have no drift. | VERIFIED | `git diff --exit-code schemas apps/desktop-electron/src/generated` passed. `gsd-tools query verify.schema-drift 02 --raw` returned `drift_detected=false`. |
| 29 | Final gate passes through `just build`, `just test`, and contract drift checks. | VERIFIED | Orchestrator evidence: `PATH="$HOME/.cargo/bin:$PATH" just build` passed; `PATH="$HOME/.cargo/bin:$PATH" just test` passed; schema drift and code review clean. Direct spot-checks also passed for critical narrow behaviors. |

**Score:** 29/29 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `crates/draft_model/src/draft.rs` | Draft root, metadata, schema version, constructor | VERIFIED | Exists, 61 lines; wired through validation, schema export, project-store. |
| `crates/draft_model/src/material.rs` | Material schema, metadata, status, registry helpers | VERIFIED | Exists, 206 lines; used by material service and schema export. |
| `crates/draft_model/src/timeline.rs` | Track/segment/timerange/magnet/keyframe/filter/transition | VERIFIED | Exists, 146 lines; exported to schema/TS and fixture semantics. |
| `crates/draft_model/src/validation.rs` | Validation/migration/errors | VERIFIED | Exists, 281 lines; called by `project_store` and registry helpers. |
| `crates/project_store/src/bundle.rs` | Create/open/save/autosave `.veproj/project.json` | VERIFIED | Exists, 167 lines; calls `validate_draft` and `migrate_draft_json`. |
| `crates/project_store/src/paths.rs` | Material URI/path classification | VERIFIED | Exists, 229 lines; used by bundle warnings and material service. |
| `crates/media_runtime/src/probe.rs` | FFprobe-backed normalized metadata | VERIFIED | Exists, 421 lines; exported by `media_runtime/src/lib.rs` and used by tests/service. |
| `crates/testkit/src/lib.rs` | Generated video/image/audio fixture helpers | VERIFIED | Exists; fixture helpers at `:249`, `:299`, `:338`; used by media/runtime/service tests. |
| `crates/bindings_node/src/material_service.rs` | Import/list/missing material orchestration | VERIFIED | Exists, 452 lines; coordinates project-store, media-runtime, draft-model, and save. |
| `crates/bindings_node/src/lib.rs` | `execute_command` routes | VERIFIED | Exists, 301 lines; routes material commands to service functions. |
| `apps/desktop-electron/src/renderer/App.tsx` | Smoke material metadata display | VERIFIED | Exists, 244 lines; uses generated command and draft contracts. |
| `fixtures/draft/positive/*/project.json` and `fixtures/draft/negative/*/project.json` | Classified project fixtures | VERIFIED | Six project fixtures found and classified by `draft_fixtures.rs`. |
| `schemas/draft.schema.json`, `schemas/command.schema.json`, generated TS | Rust-generated contracts | VERIFIED | Drift check passed; generated files include Phase 2 material/draft contracts. |
| `justfile`, `package.json` | Phase 2 gate scripts | VERIFIED | `just test` invokes Phase 2 root scripts; `package.json` includes named source guards and subsystem gates. |

### Key Link Verification

| From | To | Via | Status | Details |
|---|---|---|---|---|
| `project_store/src/bundle.rs` | `draft_model/src/validation.rs` | `validate_draft`, `migrate_draft_json` | WIRED | Imports and calls at `bundle.rs:3`, `:38`, `:92`. |
| `bindings_node/src/material_service.rs` | `project_store/src/paths.rs` | URI classification/save URI helpers | WIRED | Imports/calls `material_uri_for_save` and `classify_material_uri` at `material_service.rs:16`, `:136`, `:251`, `:390`. |
| `bindings_node/src/material_service.rs` | `media_runtime/src/probe.rs` | `probe_material_metadata` | WIRED | Imported at `material_service.rs:12`; called at `:176`. |
| `bindings_node/src/material_service.rs` | `draft_model/src/material.rs` | Registry helpers | WIRED | Imports `upsert_material`, status markers at `:8`; calls at `:159`, `:179`, `:344`. |
| `bindings_node/src/lib.rs` | `bindings_node/src/material_service.rs` | command handlers | WIRED | Imports service functions at `lib.rs:18-21`; handlers call service at `:88-167`. |
| `App.tsx` | generated command/draft TS | Type imports and command payloads | WIRED | Imports generated types at `App.tsx:3-5`; uses `CommandEnvelope`, `Draft`, `Material`, and `ListMaterialsResponse`. |
| `draft_model/tests/draft_fixtures.rs` | `fixtures/draft` | explicit classifier | WIRED | Reads and classifies all nested `project.json` fixtures at `draft_fixtures.rs:18-75`. |
| `package.json` | `justfile` | named gate scripts | WIRED | `just test` invokes package scripts; root `test` script invokes Phase 2 guards. |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|---|---|---|---|---|
| `App.tsx` material row | `materials` state | `window.videoEditorCore.executeCommand<ListMaterialsResponse>(materialListCommand)` using generated `listMaterials` payload | Yes; returns `ListMaterialsResponse.materials` and maps to rendered rows | FLOWING |
| `bindings_node/src/lib.rs` material command result | `ImportMaterialResponse`, `ListMaterialsResponse`, `ListMissingMaterialsResponse` | `material_service::{import_material_and_save, list_materials, list_missing_materials}` | Yes; tests assert real imported and missing material envelopes | FLOWING |
| `material_service.rs` imported material | `Material` | `probe_material_metadata` plus `material_from_probe`, then `upsert_material` and `save_project_bundle` | Yes; video/image/audio service test imports generated media and reopens saved draft | FLOWING |
| `project_store/src/bundle.rs` opened draft | `Draft` | Read `.veproj/project.json`, `migrate_draft_json`, `validate_draft` | Yes; round-trip tests compare semantic equality | FLOWING |
| `draft_fixtures.rs` fixture draft | `Draft` | Committed positive/negative project fixtures and generated JSON Schema | Yes; direct spot-check passed 4 fixture tests | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| Project fixtures classify and validate positive/negative `.veproj/project.json` | `cargo test -p draft_model draft_fixtures -- --nocapture` | 4 passed, 0 failed | PASS |
| Project save/open preserves semantic equality | `cargo test -p project_store round_trip -- --nocapture` | 2 passed, 0 failed | PASS |
| Missing material command diagnostics preserve draft | `cargo test -p bindings_node execute_command_reports_missing_material_diagnostics_without_corrupting_draft -- --nocapture` | 1 passed, 0 failed | PASS |
| Phase 2 source guard catches terminology/time/runtime/persistence regressions | `pnpm run test:phase2-source-guards` | passed | PASS |
| Generated contracts have no drift | `git diff --exit-code schemas apps/desktop-electron/src/generated` | exit 0 | PASS |
| Schema drift verifier | `node $HOME/.codex/get-shit-done/bin/gsd-tools.cjs query verify.schema-drift 02 --raw` | `drift_detected=false` | PASS |
| Final full gates | `PATH="$HOME/.cargo/bin:$PATH" just build`; `PATH="$HOME/.cargo/bin:$PATH" just test` | Orchestrator evidence: both passed | PASS |

### Probe Execution

No `scripts/**/tests/probe-*.sh` probes were present or declared for this phase. Step 7c: SKIPPED (no phase probe scripts).

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|---|---|---|---|---|
| DRAFT-01 | 02-02, 02-06 | User can create a new `.veproj` draft bundle. | SATISFIED | `create_project_bundle` writes valid `project.json`; project-store create/open tests and minimal fixture. |
| DRAFT-02 | 02-02, 02-06 | User can open/save without semantic round-trip changes. | SATISFIED | `round_trip_save_open_preserves_semantic_draft_equality` and autosave tests passed. |
| DRAFT-03 | 02-01, 02-06 | Jianying-aligned draft/material/timeline concepts. | SATISFIED | Rust model, generated schema/TS, fixtures, and source guard enforce terms. |
| DRAFT-04 | 02-01, 02-02, 02-03, 02-04, 02-06 | `project.json` stores semantic state only; derived artifacts excluded. | SATISFIED | Validation rejects derived fields, docs define derived boundaries, generated/fixture source guards passed. |
| DRAFT-05 | 02-01, 02-02, 02-06 | Draft versioning and migration hooks exist. | SATISFIED | `DraftSchemaVersion`, `migrate_draft_json`, structured `InvalidSchemaVersion`, negative fixture. |
| MAT-01 | 02-03, 02-04, 02-05, 02-06 | User can import video, image, and audio materials into draft. | SATISFIED | Media probe and material-service tests cover video/image/audio imports through Rust. |
| MAT-02 | 02-03, 02-04, 02-05, 02-06 | Imported materials retain IDs, URI, duration, fps, size, stream, audio metadata. | SATISFIED | `material_from_probe` maps normalized metadata into `Material`; service and fixture tests assert stable IDs and metadata. |
| MAT-03 | 02-05, 02-06 | Material bin displays imported materials with basic metadata and generated thumbnails where applicable. | SATISFIED FOR PHASE 2 SCOPE | Roadmap SC narrows Phase 2 to probed metadata visibility in the material bin; Electron smoke displays metadata through generated contracts. Thumbnail/cache generation is explicitly treated as a derived-artifact concern outside `project.json` and reserved for preview-derived systems, not Phase 2 semantic persistence. |
| MAT-04 | 02-04, 02-05, 02-06 | Missing files present recovery/error state without corrupting draft. | SATISFIED | Service and binding tests preserve missing entries and return classified diagnostics; spot-check passed. |

All requirement IDs requested by the orchestrator are accounted for in plan frontmatter and code/test evidence. No extra Phase 2 requirements in `.planning/REQUIREMENTS.md` were orphaned.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---:|---|---|---|
| `crates/media_runtime/tests/material_probe.rs` | 99, 129 | `placeholder` byte string | INFO | Test-only fake executor input for failure-path tests; does not flow to user-visible UI or persisted draft semantics. |
| `apps/desktop-electron/src/main/nativeBinding.ts` | 73 | `return null` | INFO | Older native binding loader file outside Phase 2 reviewed files; not a Phase 2 implementation stub. |

No unreferenced `TBD`, `FIXME`, or `XXX` debt markers were found in Phase 2 implementation/reviewed files. Hardcoded empty/null matches were optional defaults or generated TypeScript nullability checks, not hollow data paths.

### Human Verification Required

None. The phase success criteria are covered by code inspection, generated contract drift checks, Rust tests, source guards, and Playwright smoke assertions. No `<human-check>` blocks were present in the Phase 2 plans.

### Gaps Summary

No blocking gaps found. The implementation satisfies the roadmap success criteria, all Phase 2 plan must-haves, and the requested requirement IDs against actual codebase evidence.

---

_Verified: 2026-06-17T04:50:31Z_
_Verifier: the agent (gsd-verifier)_
