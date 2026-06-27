---
phase: 17
slug: template-import-core-and-kaipai-offline-adapter-foundation
status: complete
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-24
---

# Phase 17 - Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test`, pnpm shell guards, schema/TypeScript drift checks, Playwright/Electron product tests if UI import/report surfaces are added |
| **Config file** | `Cargo.toml`, `package.json`, `apps/desktop-electron/playwright.config.ts` |
| **Quick run command** | `pnpm run test:phase17-rust && pnpm run test:phase17-source-guards` |
| **Full suite command** | `pnpm run test:phase17 && pnpm run test:no-product-fallback && cargo check --workspace --locked && pnpm run test:contracts` |
| **Estimated runtime** | ~600 seconds after fixture export gates exist |

---

## Sampling Rate

- **After every task commit:** Run the focused crate test for the touched files plus `pnpm run test:phase17-source-guards`.
- **After every plan wave:** Run `pnpm run test:phase17-rust`, `pnpm run test:contracts`, and any added Playwright subset.
- **Before `$gsd-verify-work`:** Full suite must be green: `pnpm run test:phase17 && pnpm run test:no-product-fallback && cargo check --workspace --locked && pnpm run test:contracts`.
- **Max feedback latency:** 900 seconds for the full Phase 17 gate once export fixtures are in place.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 17-W0-01 | 17-01 | 0 | COMP-02 / D-31-D-33 | T-17-01 | Report schema classifies supported, approximated, dropped, missingResource, and needsNativeEffect without raw provider secrets | unit/schema | `cargo test -p draft_import adaptation_report -- --nocapture` | yes | green |
| 17-W0-02 | 17-01 | 0 | D-41-D-43 | T-17-02 | Source guards reject raw Kaipai/provider/Android/live API semantics in core/render/session success paths | shell guard | `pnpm run test:phase17-source-guards` | yes | green |
| 17-W0-03 | 17-02 | 1 | D-14-D-16 | T-17-03 | Localizer rejects traversal, remote render URLs, sha256 mismatch, duplicate destinations, missing resources, and symlink escapes, and exports `localize_template_resources` | Rust integration | `cargo test -p draft_import resource_localizer -- --nocapture` | yes | green |
| 17-W0-04 | 17-03 | 2 | D-11-D-13 | T-17-04 | `DraftImportPlan` validates canonical materials/tracks/segments before project-session mutation | Rust unit/integration | `cargo test -p draft_import draft_import_plan -- --nocapture` | yes | green |
| 17-W0-05 | 17-07 | 1 | PRODFX-05 / D-26-D-28 | T-17-05 | Generic static center-anchor rotation export parity is closed before rotated imports are classified as supported | Rust parity | `cargo test -p ffmpeg_compiler transform -- --nocapture && cargo test -p testkit preview_export_parity -- --nocapture` | yes | green |
| 17-W0-06 | 17-05 | 3 | D-17-D-30 / D-33 / D-39 / D-46 | T-17-06 | Kaipai mapper fixture/report snapshot corpus is sanitized, explicitly cataloged, and covers supported/approximated/dropped/missing/native classifications | Rust snapshot | `cargo test -p adapter_kaipai mapper_fixture_snapshots -- --nocapture` | yes | green |
| 17-W0-07 | 17-10 | 4 | D-17-D-30 | T-17-07 | Offline Kaipai fixtures map only supported/approximated subset into generic draft semantics and report every dropped/native feature | Rust mapper | `cargo test -p adapter_kaipai offline_mapper -- --nocapture` | yes | green |
| 17-W0-08 | 17-06 | 5 | COMP-01 / D-12 / D-15 | T-17-08 | Project-session import requires `sessionId` and `expectedRevision`, applies atomically, persists localized resources into `artifact_store::resource_index`, and rejects stale revisions | Rust binding integration | `cargo test -p bindings_node project_session_import_kaipai -- --nocapture` | yes | green |
| 17-W0-09 | 17-08 | 6 | PRODFX-05 / D-44-D-45 | T-17-09 | Fixture exports produce non-empty MP4s with expected layer/text/audio evidence and no Android/fallback dependency | Rust export smoke | `cargo test -p testkit template_import_exports -- --nocapture` | yes | green |
| 17-W0-10 | 17-08 | 6 | NO-FALLBACK-01 / D-43-D-45 | T-17-10 | Supported preview evidence comes from realtime render-graph product path, not artifact/mock/CPU/Android evidence | Playwright/Rust product evidence | `pnpm run test:phase17-preview` | yes | green |
| 17-W0-11 | 17-09 | 7 | TEST-E2E-01 / D-40 | T-17-11 | If UI import/report exists, normal product workflow imports a fixture, shows Chinese report copy, previews, and exports | Playwright/Electron | `pnpm --filter @video-editor/desktop exec playwright test tests/template-import.spec.ts --reporter=line` | yes | green |

*Status values: pending, green, red, flaky*

---

## Wave 0 Requirements

- [x] `crates/draft_import/` or equivalent provider-neutral import module with `DraftImportPlan`, `AdaptationReport`, schema exports, and focused tests.
- [x] `crates/adapter_kaipai/` current-main adapter crate porting only old branch contracts/fixtures/ideas, not old integration behavior.
- [x] `fixtures/kaipai/` current-main fixture corpus for main video, PIP, text sticker, BGM/audio, missing resource, and native effect degradation.
- [x] `scripts/phase17-source-guards.sh` with negative checks for core/render provider leakage, raw formula leakage, Android/live API dependency, remote render URLs in runtime drafts, and fallback success evidence.
- [x] `package.json` scripts: `test:phase17-rust`, `test:phase17-source-guards`, `test:phase17-export-fixtures`, `test:phase17-preview`, and `test:phase17`.
- [x] Schema/TypeScript drift checks for any new import/report contracts exposed to desktop or fixtures.

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| New real Kaipai formula fixture admission | D-16 / D-39 | Real provider samples may contain secrets, account IDs, signed URLs, or personal media. | Before committing any new real sample, review sanitized JSON/resources manually and run the fixture secret scanner added by Phase 17. |
| Visual quality of approximate import result | D-02 / D-44 | Automated tests can prove layers/text/audio/output existence, but "high-quality approximate" needs human review before expanding support. | Open the imported draft, preview fixture cases, inspect text legibility/layer order/audio, and confirm every visible degradation is reflected in the report. |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies.
- [x] Sampling continuity: no 3 consecutive tasks without automated verify.
- [x] Wave 0 covers all MISSING references.
- [x] No watch-mode flags.
- [x] Feedback latency < 900s.
- [x] Product success evidence excludes Android worker, live Kaipai API, artifact fallback, mock, CPU readback, and remote render URLs.
- [x] `nyquist_compliant: true` set in frontmatter after Wave 0 tests exist and pass.

**Approval:** automated validation passed on 2026-06-24 with `pnpm run test:phase17`.
