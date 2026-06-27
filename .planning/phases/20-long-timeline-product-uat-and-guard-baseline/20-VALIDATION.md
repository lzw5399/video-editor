---
phase: 20
slug: long-timeline-product-uat-and-guard-baseline
status: passed
nyquist_compliant: true
wave_0_complete: true
created: 2026-06-28
---

# Phase 20 - Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

## Test Infrastructure

| Property | Value |
|----------|-------|
| Framework | Cargo/Rust tests plus Playwright Test for packaged Electron product UAT |
| Config file | `Cargo.toml`, `apps/desktop-electron/playwright.config.ts` |
| Quick run command | `cargo test -p testkit --test large_timeline_incremental phase20_blocking_1000_segments_per_track_keeps_localized_diff_bounded -- --nocapture` plus the narrow Playwright grep created in Wave 0 |
| Full suite command | `pnpm run test:phase20` |
| Estimated runtime | Diagnostic quick loop under 5 minutes; full packaged gate may run longer and is blocking for closeout |

## Sampling Rate

- **After every task commit:** Run the narrow Rust or Playwright command tied to the touched helper or spec.
- **After every plan wave:** Run `pnpm run test:no-product-fallback && bash scripts/phase20-source-guards.sh` plus the Phase 20 Playwright spec when created.
- **Before `$gsd-verify-work`:** Run `pnpm run test:phase20` and keep the evidence bundle paths in the phase summary.
- **Max feedback latency:** Rust/source guard feedback should stay under 5 minutes; packaged UAT latency is accepted as longer because it is the blocking product proof.

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 20-W0-rust-fixture | TBD | 0 | UAT11-01, UAT11-02, LONG11-01 | T20-01 | Generated `.veproj` uses canonical project_store semantics and bundle-safe material URIs | Rust integration | `cargo test -p testkit --test long_timeline_product_fixture -- --nocapture` and `cargo test -p testkit --test large_timeline_incremental phase20_blocking_1000_segments_per_track_keeps_localized_diff_bounded -- --nocapture` | yes | passed |
| 20-W0-evidence | TBD | 0 | UAT11-02, GATE11-01 | T20-02 | Evidence helpers reject derived artifacts, fallback preview, and file-exists-only export proof | Playwright helper tests/source guard | `pnpm run test:no-product-fallback && bash scripts/phase20-source-guards.sh` | yes | passed |
| 20-W1-product-uat | TBD | 1 | UAT11-01, LONG11-01 | T20-03 | Packaged app performs normal-user edits against the long project and records compositor evidence | Playwright packaged E2E | `pnpm --filter @video-editor/desktop package:dir && pnpm --filter @video-editor/desktop exec playwright test tests/product-long-timeline-uat.spec.ts --reporter=line --workers=1` | yes | passed |
| 20-W1-canonical-cycles | TBD | 1 | UAT11-02 | T20-04 | Two save/reopen/export cycles preserve normalized canonical draft facts and keep derived artifacts out of `project.json` | Playwright packaged E2E | `pnpm --filter @video-editor/desktop exec playwright test tests/product-long-timeline-uat.spec.ts -g canonical --reporter=line --workers=1` | yes | passed |
| 20-W1-pressure | TBD | 1 | LONG11-02 | T20-05 | Export/probe/artifact/cache pressure does not block scrub, inspector edit, preview delivery, commit, or cancel | Playwright stress | `pnpm --filter @video-editor/desktop exec playwright test tests/product-long-timeline-uat.spec.ts -g pressure --reporter=line --workers=1` | yes | passed |
| 20-W2-aggregate | TBD | 2 | UAT11-01, UAT11-02, LONG11-01, LONG11-02, GATE11-01 | T20-06 | Phase aggregate fails fallback/source-only success and proves Rust, contract, product, and export gates | Aggregate | `pnpm run test:phase20` | yes | passed |

## Wave 0 Requirements

- [ ] `crates/testkit/tests/long_timeline_product_fixture.rs` or an extension to `crates/testkit/tests/large_timeline_incremental.rs` for the 180 x 3 product fixture, 1000 segments/track blocking gate, and 3000 segments/track diagnostic path.
- [ ] `apps/desktop-electron/tests/product-long-timeline-uat.spec.ts` for the packaged long-session UAT.
- [ ] `apps/desktop-electron/tests/helpers/longTimelineEvidence.ts` for semantic summaries, evidence bundle writing, ffprobe/sample collection, and budget assertions.
- [ ] `scripts/phase20-source-guards.sh` for long UAT, no-fallback, preview evidence, export evidence, and canonical project source guards.
- [ ] Root/package scripts: `test:phase20-rust`, `test:phase20-source-guards`, `test:phase20-desktop`, and `test:phase20`.

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Product-readable failure wording | D-17 | Error-copy quality is partly editorial, though failure artifacts are automated | Inspect one intentionally failed local run and confirm the product summary names workflow, segment/time/export stage, and developer-detail artifact paths |
| 3000 segments/track pressure run | D-04 | Explicitly non-blocking diagnostic; excluded from `test:phase20` and automated task verification | Optionally run `cargo test -p testkit --test large_timeline_incremental phase20_diagnostic_3000_segments_per_track_reports_structural_stats -- --ignored --nocapture` when collecting pressure diagnostics |

## Validation Sign-Off

- [x] All phase requirements have automated gates or Wave 0 test infrastructure work.
- [x] Sampling continuity avoids three consecutive implementation tasks without an automated check.
- [x] Wave 0 covers every missing test/evidence file named in research.
- [x] No watch-mode flags are used in blocking commands.
- [x] Full closeout requires `pnpm run test:phase20`.
- [x] `nyquist_compliant: true` is set in frontmatter.

**Approval:** passed 2026-06-28 after `pnpm run test:phase20`.
