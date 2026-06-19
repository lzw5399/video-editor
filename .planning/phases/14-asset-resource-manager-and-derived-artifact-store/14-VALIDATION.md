---
phase: 14
slug: asset-resource-manager-and-derived-artifact-store
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-06-19
---

# Phase 14 - Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test`, pnpm scripts, generated contract diff checks, Playwright Electron tests when binding-visible UI changes are introduced |
| **Config file** | `Cargo.toml`, `package.json`, `apps/desktop-electron/playwright.config.ts` |
| **Quick run command** | `cargo test -p artifact_store -- --nocapture` after Wave 0 creates the crate |
| **Full suite command** | `pnpm run test:phase14 && pnpm run test:contracts` |
| **Estimated runtime** | ~180 seconds after Phase 14 scripts exist |

---

## Sampling Rate

- **After every task commit:** Run the focused crate/module test named in the task.
- **After every plan wave:** Run `pnpm run test:phase14`.
- **Before `$gsd-verify-work`:** `pnpm run test:phase14 && pnpm run test:contracts && pnpm run test:phase13` must be green.
- **Max feedback latency:** 180 seconds for focused phase gates; full workspace checks may run only at final phase close or when public APIs change.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 14-01-01 | 01 | 1 | ASSET-02 | T-14-01 | SQLite schema uses foreign keys, schema version, deterministic migrations, and project-contained paths | Rust integration | `cargo test -p artifact_store sqlite_schema -- --nocapture` | W0 | pending |
| 14-01-02 | 01 | 1 | ASSET-02 | T-14-02 | Blob writes are project-contained, atomic, fingerprinted, and repairable after interruption | Rust integration | `cargo test -p artifact_store blob_store -- --nocapture` | W0 | pending |
| 14-02-01 | 02 | 2 | ASSET-01 | T-14-03 | Materials, fonts, effects, proxies, thumbnails, and waveforms receive stable resource IDs and project-relative refs | Rust unit/integration | `cargo test -p artifact_store resource_index -- --nocapture` | W0 | pending |
| 14-03-01 | 03 | 3 | ASSET-03 | T-14-04 | Replace/relink/rename/delete invalidates only dependency-matched artifact rows unless overflow/unknown requires full-draft fallback | Rust integration | `cargo test -p artifact_store invalidation -- --nocapture` | W0 | pending |
| 14-04-01 | 04 | 4 | ASSET-04 | T-14-05 | Derived generation stores chunk/job status and cancellation/resume state without blocking preview service contracts | Rust integration | `cargo test -p artifact_store artifact_jobs -- --nocapture` | W0 | pending |
| 14-04-02 | 04 | 4 | ASSET-04 | T-14-05 | Proxy, thumbnail, and waveform generation facades produce non-empty BlobStore-backed artifacts while job/chunk rows update durably | Rust integration | `cargo test -p artifact_store artifact_generation -- --nocapture` | W0 | pending |
| 14-04-03 | 04 | 4 | ASSET-04 | T-14-05 | Generation cancellation/resume skips completed chunks, avoids duplicate blobs, and exposes preview-safe status summaries | Rust integration | `cargo test -p artifact_store artifact_jobs -- --nocapture && cargo test -p artifact_store artifact_generation -- --nocapture` | W0 | pending |
| 14-05-01 | 05 | 5 | ASSET-05 | T-14-06 | GC/quota/sync manifest preserves live blobs and reports only project-relative/fingerprinted artifacts | Rust integration | `cargo test -p artifact_store gc_quota_manifest -- --nocapture` | W0 | pending |
| 14-06-01 | 06 | 6 | ASSET-01, ASSET-02, ASSET-04 | T-14-07 | Binding contracts expose status/progress/actions only; renderer does not compute artifact internals | Rust + contracts | `cargo test -p bindings_node artifact_store_commands -- --nocapture && pnpm run test:contracts` | W0 | pending |
| 14-07-01 | 07 | 7 | ASSET-01, ASSET-04, ASSET-05 | T-14-07 | Production UI exposes resource status/task/maintenance surfaces without forbidden debug internals | Playwright + source guard | `pnpm --filter @video-editor/desktop test:workspace -g "资源任务|资源维护|素材资源状态|缓存空间" && pnpm run test:phase14-source-guards` | W0 | pending |
| 14-ALL | all | all | ASSET-01..ASSET-05 | T-14-08 | Source guards reject renderer-owned artifact roots, cache keys, fingerprints, dirty ranges, SQLite, and FFmpeg command ownership | Shell guard | `pnpm run test:phase14-source-guards` | W0 | pending |

*Status values: pending, green, red, flaky.*

---

## Wave 0 Requirements

- [ ] `crates/artifact_store/` - SQLite schema, connection PRAGMAs, migration/version checks, blob path containment, atomic write/repair helpers, source/blob fingerprint helpers.
- [ ] `crates/artifact_store/tests/sqlite_schema.rs` - schema and dependency-row tests for ASSET-02.
- [ ] `crates/artifact_store/tests/blob_store.rs` - project-contained blob write, fingerprint, temp-file, and repair tests for ASSET-02.
- [ ] `crates/artifact_store/tests/resource_index.rs` - stable resource ID and project-relative reference tests for ASSET-01.
- [ ] `crates/artifact_store/tests/invalidation.rs` - exact dependency invalidation tests for ASSET-03.
- [ ] `crates/artifact_store/src/generation.rs` - Rust-owned proxy, thumbnail, and waveform generation facades/workers that commit outputs through `BlobStore` for ASSET-04.
- [ ] `crates/artifact_store/tests/artifact_jobs.rs` - chunk/resume/cancel generation state tests for ASSET-04.
- [ ] `crates/artifact_store/tests/artifact_generation.rs` - proxy, thumbnail, waveform generated blob row/file tests for ASSET-04, including cancellation and resume behavior.
- [ ] `crates/artifact_store/tests/gc_quota_manifest.rs` - live artifact preservation, quota, tombstone, and sync manifest tests for ASSET-05.
- [ ] `scripts/phase14-source-guards.sh` - renderer/source ownership guard for artifact store, `.veproj/derived`, SQLite, fingerprints, cache keys, graph node IDs, dirty ranges, and FFmpeg command construction.
- [ ] Root `package.json` scripts `test:phase14-rust`, `test:phase14-source-guards`, and `test:phase14`.
- [ ] Generated contract checks if binding-visible artifact status, generation, quota, or maintenance commands are added.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Jianying-style visual fit for new resource/artifact status surfaces | ASSET-01, ASSET-04, ASSET-05 | Automated tests can prove layout stability and forbidden-copy absence, but final product fit may require screenshot review against the provisional reference set | Capture `1280x800` and `1120x720` desktop screenshots with active/inactive resource tasks and confirm no debug paths, no overlap, and no extra dashboard/card-heavy UI |

---

## Validation Sign-Off

- [x] All planned tasks have an automated verify command or Wave 0 dependency.
- [x] Sampling continuity: no 3 consecutive tasks without automated verify.
- [x] Wave 0 covers all missing test references.
- [x] No watch-mode flags.
- [x] Feedback latency target is under 180 seconds for focused gates.
- [x] `nyquist_compliant: true` set in frontmatter.

**Approval:** approved 2026-06-19
