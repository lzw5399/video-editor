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
| 02-01-W2 | 02-01 | 1 | DRAFT-04 | T-02-04 | Canonical `project.json` excludes derived thumbnails, waveforms, render graphs, FFmpeg scripts, and raw probe JSON | schema/fixture | `cargo test -p draft_model schema_fixtures -- --nocapture` | no W0 | pending |
| 02-02-W0 | 02-02 | 2 | DRAFT-01 | T-02-05 | New `.veproj/project.json` bundle is valid and saveable without UI mutation | integration | `cargo test -p project_store create_project_bundle -- --nocapture` | no W0 | pending |
| 02-02-W1 | 02-02 | 2 | DRAFT-02 | T-02-06 | Save/open compares Rust semantic equality and preserves deterministic persisted semantics | integration | `cargo test -p project_store round_trip -- --nocapture` | no W0 | pending |
| 02-02-W2 | 02-02 | 2 | DRAFT-04 | T-02-07 | Relative/external material URI handling is centralized in `project_store` | integration | `cargo test -p project_store path_resolution -- --nocapture` | no W0 | pending |
| 02-03-W0 | 02-03 | 2 | MAT-01 | T-02-08 | Video, image, and audio material import is Rust-owned and uses `media_runtime`, not renderer ffprobe calls | integration | `cargo test -p project_store import_material -- --nocapture` | no W0 | pending |
| 02-03-W1 | 02-03 | 2 | MAT-02 | T-02-09 | Material IDs and normalized metadata persist without raw ffprobe JSON | integration | `cargo test -p media_runtime material_probe -- --nocapture` | no W0 | pending |
| 02-03-W2 | 02-03 | 2 | MAT-03 | T-02-10 | Generated contracts expose basic material metadata for the later material bin surface | contract/smoke | `pnpm --filter @video-editor/desktop test` | partial existing | pending |
| 02-03-W3 | 02-03 | 2 | MAT-04 | T-02-11 | Missing materials remain recoverable diagnostics and do not corrupt or delete draft entries | integration | `cargo test -p project_store missing_material -- --nocapture` | no W0 | pending |
| 02-04-W0 | 02-04 | 3 | DRAFT-01, DRAFT-02, DRAFT-03, DRAFT-04, DRAFT-05, MAT-01, MAT-02, MAT-03, MAT-04 | T-02-12 | Fixtures cover positive/negative drafts and generated schema/TS artifacts have no drift | fixture/contract | `git diff --exit-code schemas apps/desktop-electron/src/generated` | partial existing | pending |

## Wave 0 Requirements

- [ ] Extend or add `crates/draft_model/tests/draft_schema.rs` to cover `Draft`, `Material`, `Track`, `Segment`, schema versioning, migrations, strict JSON, and Jianying terminology.
- [ ] Add `crates/project_store/tests/project_bundle.rs` for create/save/open/autosave, semantic equality, path resolution, and missing-material preservation.
- [ ] Add `crates/media_runtime/tests/material_probe.rs` for ffprobe metadata normalization across video, image, audio, and probe failure.
- [ ] Extend `crates/testkit` with deterministic generated image and audio-only fixture helpers.
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
