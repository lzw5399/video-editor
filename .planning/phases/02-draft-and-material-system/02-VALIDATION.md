---
phase: 02
slug: draft-and-material-system
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-06-17
---

# Phase 02 - Validation Strategy

Per-phase validation contract for draft durability, material probing, generated contracts, and missing-material recovery.

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test`, `jsonschema` fixture validation, generated TS/schema drift checks, Electron smoke where contracts are exposed |
| **Config file** | `Cargo.toml`, `justfile`, `apps/desktop-electron/package.json`, `apps/desktop-electron/playwright.config.ts` |
| **Quick run command** | `cargo test -p draft_model && cargo test -p project_store` |
| **Full suite command** | `PATH="$HOME/.cargo/bin:$PATH" just test` |
| **Estimated runtime** | ~60-180 seconds after Phase 2 tests are added |

## Sampling Rate

- **After every task commit:** Run the narrowest affected Rust crate test, then `cargo test -p draft_model && cargo test -p project_store` once both crates contain Phase 2 tests.
- **After every plan wave:** Run `PATH="$HOME/.cargo/bin:$PATH" just test`.
- **Before `$gsd-verify-work`:** Run `PATH="$HOME/.cargo/bin:$PATH" just build`, `PATH="$HOME/.cargo/bin:$PATH" just test`, and `git diff --exit-code schemas apps/desktop-electron/src/generated`.
- **Max feedback latency:** 180 seconds for the full Phase 2 suite on the local machine.

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 02-01-W0 | 02-01 | 1 | DRAFT-03 | T-02-01 / T-02-02 | Strict serde/schema rejects unknown draft fields; domain types use Jianying terms | unit/schema | `cargo test -p draft_model draft_schema -- --nocapture` | no W0 | pending |
| 02-01-W1 | 02-01 | 1 | DRAFT-05 | T-02-03 | Unknown future schema versions fail with structured recoverable errors | unit | `cargo test -p draft_model migration -- --nocapture` | no W0 | pending |
| 02-02-W0 | 02-02 | 2 | DRAFT-01 | T-02-05 | New `.veproj/project.json` bundle is valid and saveable without UI mutation | integration | `cargo test -p project_store create_project_bundle -- --nocapture` | no W0 | pending |
| 02-02-W1 | 02-02 | 2 | DRAFT-02 | T-02-06 | Save/open compares Rust semantic equality and preserves deterministic persisted semantics | integration | `cargo test -p project_store round_trip -- --nocapture` | no W0 | pending |
| 02-02-W2 | 02-02 | 2 | DRAFT-04 | T-02-07 | Relative/external material URI handling is centralized in `project_store`; project_store does not orchestrate imports | integration | `cargo test -p project_store path_resolution -- --nocapture` | no W0 | pending |
| 02-03-W0 | 02-03 | 2 | MAT-01, MAT-02 | T-02-10 / T-02-11 | Video, image, and audio probes normalize metadata through `media_runtime`, not renderer or project-store ffprobe calls | integration | `cargo test -p media_runtime material_probe -- --nocapture` | no W0 | pending |
| 02-03-W1 | 02-03 | 2 | MAT-01 | T-02-13 | Generated video, image, and audio material test helpers stay temp-dir backed and uncommitted | integration | `cargo test -p testkit material -- --nocapture` | no W0 | pending |
| 02-04-W0 | 02-04 | 3 | MAT-01, MAT-02, MAT-04, DRAFT-04 | T-02-15 / T-02-16 / T-02-17 | Material import orchestration lives in the binding-facing Rust service, with pure draft_model registry helpers and project_store persistence only | integration | `cargo test -p bindings_node material_service -- --nocapture` | no W0 | pending |
| 02-04-W1 | 02-04 | 3 | DRAFT-03, DRAFT-04 | T-02-18 | Generated draft schema/TS artifacts expose material metadata without derived cache fields | contract | `git diff --exit-code schemas apps/desktop-electron/src/generated` | partial existing | pending |
| 02-05-W0 | 02-05 | 4 | MAT-01, MAT-02, MAT-03, MAT-04 | T-02-19 / T-02-20 / T-02-21 | Generated material commands route through the service and Electron displays smoke-level metadata only | contract/smoke | `cargo test -p bindings_node -- --nocapture` and `pnpm --filter @video-editor/desktop test` | partial existing | pending |
| 02-06-W0 | 02-06 | 5 | DRAFT-01, DRAFT-02, DRAFT-03, DRAFT-04, DRAFT-05, MAT-01, MAT-02, MAT-03, MAT-04 | T-02-23 / T-02-24 / T-02-25 / T-02-26 | Fixtures cover positive/negative drafts and final build/test/generated drift gates pass | fixture/contract | `PATH="$HOME/.cargo/bin:$PATH" just build`, `PATH="$HOME/.cargo/bin:$PATH" just test`, and `git diff --exit-code schemas apps/desktop-electron/src/generated` | partial existing | pending |

## Wave 0 Requirements

- [ ] Extend or add `crates/draft_model/tests/draft_schema.rs` to cover `Draft`, `Material`, `Track`, `Segment`, schema versioning, migrations, strict JSON, and Jianying terminology.
- [ ] Add `crates/project_store/tests/project_bundle.rs` for create/save/open/autosave, semantic equality, path resolution, and missing-material preservation.
- [ ] Add `crates/media_runtime/tests/material_probe.rs` for ffprobe metadata normalization across video, image, audio, and probe failure.
- [ ] Extend `crates/testkit` with deterministic generated image and audio-only fixture helpers.
- [ ] Add binding-facing material service tests proving import orchestration is outside `project_store`.
- [ ] Add `fixtures/draft/positive` and `fixtures/draft/negative` or an equivalent deterministic fixture layout documented by tests.
- [ ] Extend schema/TS generation to include draft/material contracts and fail on drift.

## Manual-Only Verifications

All Phase 2 behaviors have automated verification. Rich material-bin UI inspection is deferred to Phase 4.

## Validation Sign-Off

- [x] All tasks have automated verify targets or Wave 0 dependencies.
- [x] Sampling continuity: no 3 consecutive tasks without automated verify.
- [x] Wave 0 covers all missing test references.
- [x] No watch-mode flags.
- [x] Feedback latency target is below 180 seconds.
- [x] `nyquist_compliant: true` set in frontmatter.

**Approval:** approved 2026-06-17
