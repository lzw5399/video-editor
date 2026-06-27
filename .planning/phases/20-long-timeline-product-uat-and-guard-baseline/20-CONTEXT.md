# Phase 20: Long Timeline Product UAT And Guard Baseline - Context

**Gathered:** 2026-06-28
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 20 creates the v1.1 product-truth baseline for long editing sessions. It must prove that a generated long `.veproj` can be opened in the packaged Electron product, edited through normal UI paths, previewed through the production compositor, saved, reopened, exported, and verified without fallback evidence or canonical project drift.

This phase does not close crop/export parity, Phase 19 effect parity, shortcut polish, or UI polish directly. It creates the product UAT, telemetry, no-fallback/source-guard, and evidence bundle that later phases must satisfy.

</domain>

<decisions>
## Implementation Decisions

### Long Timeline Scale

- **D-01:** Use a hybrid baseline. Product E2E runs a stable medium-long timeline; Rust/testkit runs larger pressure cases.
- **D-02:** Product E2E baseline is `180 segments/track x 3 tracks` covering video, audio, and text, about `540` total segments.
- **D-03:** Product E2E segment grain is `1s/segment`, producing about a 3-minute timeline. Export should validate metadata and sampled semantic frames rather than relying on file existence.
- **D-04:** Rust/testkit blocking gate should cover `1000 segments/track`; `3000 segments/track` is a non-blocking diagnostic pressure run.

### Product UAT Path

- **D-05:** Generate the long project through Rust/testkit or an equivalent Rust-owned fixture path, then open the generated `.veproj` in the product. Do not build all 540 segments manually through the UI in the long-session UAT.
- **D-06:** After opening the generated project, the product UAT must perform the core edit set through normal UI paths: selection, scroll/zoom, scrub/play, move, trim, split, undo/redo, inspector visual edit, save/reopen, and export.
- **D-07:** Phase 19 retime/effect/filter/mask/blend parity controls are not part of Phase 20's main long-session path. Phase 20 may include already-stable baseline semantics if present in the fixture, but detailed Phase 19 parity belongs to Phase 23.
- **D-08:** The long-session product UAT must run two reopen cycles and two exports: open generated project, edit and save, reopen and verify, continue editing and save, reopen and verify, then export twice.
- **D-09:** Packaged Electron long-session UAT is the blocking product gate. A dev workflow may run the same flow or a shortened diagnostic flow for faster debugging, but packaged evidence is required for Phase 20 completion.

### Performance And Telemetry Budgets

- **D-10:** Use pragmatic product interaction budgets for the Phase 20 baseline: inspector edit `<= 2.5s`; selection, move, trim, split, undo, and redo single-step operations `<= 2s`; scroll, zoom, and scrub visible feedback `<= 1.5s`.
- **D-11:** Formalize the existing scheduler stress baseline: scheduler queue latency `p95 <= 2s`, normal product work rejected count `0`, fallback count `0`, stale generations may be rejected but must never present, and visible preview center must change under pressure.
- **D-12:** Export should complete within a reasonable test timeout, but fixed wall-clock export duration is not a hard success budget. Success requires `ffprobe` duration/fps/resolution/audio validation plus sampled semantic frames around start, middle, tail, or edit points.
- **D-13:** Rust/testkit large-scale gates should prioritize bounded graph diff, dirty range, and cache invalidation assertions over wall-clock timing. Wall-clock data should be captured as diagnostics, not used as a hard gate in Phase 20.

### Evidence And Failure Artifacts

- **D-14:** Phase 20 uses a structured evidence bundle. Successful runs keep lightweight JSON summaries; failed runs retain full Playwright trace, screenshots, video, telemetry, project semantic summaries, native preview evidence, `ffprobe` output, and sampled frame evidence.
- **D-15:** Save/reopen evidence uses semantic normalized comparison of canonical draft facts: materials, tracks, segments, timing, visual, audio, text, and revisions. Comparisons must explicitly ignore derived artifacts, absolute temp paths, runtime-only handles, and other non-canonical facts.
- **D-16:** No-fallback/source guards are blocking Phase 20 gates and must be extended to cover the generated long `.veproj`, packaged UAT, long-session preview evidence, and export evidence. Fallback, mock, artifact, CPU probe, DOM overlay, native single-video proof, first-frame snapshot, or file-exists-only export success must fail.
- **D-17:** Failure diagnostics should include a product-readable summary plus developer details. Product summary names the workflow, segment/time/export stage when possible; developer details include telemetry, native command observations, realtime host state, scheduler counters, and `ffprobe`/stderr summaries.

### the agent's Discretion

Planner and executor may choose exact helper names, test file names, JSON evidence schema details, export timeout values, sampled-frame extraction implementation, and non-blocking diagnostic command wiring, as long as the decisions above and the architecture boundary are preserved.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase Scope And Requirements

- `.planning/PROJECT.md` — v1.1 milestone goals, architecture constraints, no-product-fallback policy, and Rust ownership decisions.
- `.planning/REQUIREMENTS.md` — Phase 20 mapped requirements: `UAT11-01`, `UAT11-02`, `LONG11-01`, `LONG11-02`, and `GATE11-01`.
- `.planning/ROADMAP.md` — Phase 20 goal, success criteria, dependencies, and position before Phase 21-24.
- `.planning/STATE.md` — current v1.1 state, Phase 20 concerns, and deferred scope.
- `.planning/research/SUMMARY.md` — v1.1 research synthesis, especially Phase 20 implications and product-truth risk framing.

### Product Evidence Policies

- `docs/product-e2e-acceptance-policy.md` — defines product E2E completion evidence and fixture expectations.
- `docs/no-product-fallback-policy.md` — defines fallback evidence that cannot satisfy product success.
- `docs/runtime-boundaries.md` — defines Electron/Rust/Node-API/render/export/cache ownership boundaries and portable runtime separation.

### Existing Phase 20-Relevant Code

- `crates/testkit/src/large_timeline.rs` — existing deterministic large-timeline fixture builder and scale knobs.
- `crates/testkit/tests/large_timeline_incremental.rs` — existing bounded graph diff, dirty range, and cache invalidation assertions.
- `apps/desktop-electron/tests/product-scheduler-stress.spec.ts` — existing scheduler pressure product test and telemetry budget pattern.
- `apps/desktop-electron/tests/product-user-journey.spec.ts` — existing no-fallback product journey and compositor evidence patterns.
- `apps/desktop-electron/tests/real-workflow.spec.ts` — existing dev/packaged no-mock import-preview-export workflow and reopen assertion pattern.
- `apps/desktop-electron/tests/helpers/userJourney.ts` — reusable product UI helpers, native command observations, realtime host evidence, scheduler telemetry readers, and media fixture constants.
- `apps/desktop-electron/tests/helpers/realWorkflow.ts` — reusable import/preview/export and reopen workflow helpers.
- `apps/desktop-electron/tests/helpers/packagedApp.ts` — packaged Electron launch helper for blocking product UAT.
- `apps/desktop-electron/src/main/nativeBinding.ts` — telemetry and native binding response shapes exposed to desktop tests.
- `crates/realtime_preview_runtime/src/telemetry.rs` — realtime preview scheduler telemetry fields including queue latency, rejected/canceled/stale counters, and pacing samples.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets

- `LargeTimelineConfig` and `build_large_timeline` in `crates/testkit/src/large_timeline.rs`: can generate deterministic video/audio/text long drafts with configurable segment counts, durations, track mix, and localized edit targets.
- `crates/testkit/tests/large_timeline_incremental.rs`: already proves large fixtures are deterministic and that localized edits keep graph diff, dirty ranges, and preview cache invalidation bounded.
- `apps/desktop-electron/tests/product-scheduler-stress.spec.ts`: already combines product preview, export pressure, import pressure, inspector edits, scheduler telemetry, and no-fallback assertions.
- `apps/desktop-electron/tests/helpers/userJourney.ts`: already exposes product media fixtures, UI actions, native command observations, project-session observations, realtime preview host observations, and scheduler telemetry readers.
- `apps/desktop-electron/tests/real-workflow.spec.ts` and helpers: already provide dev and packaged no-mock import-preview-export plus reopen assertions.

### Established Patterns

- Product behavior should be driven through visible UI or the same bridge the UI uses, not through direct Rust/unit calls when claiming user-visible success.
- Preview success requires `renderGraphGpuComposited` evidence, visible preview-region change, no fallback, and no `requestProjectSessionPreviewFrame` artifact fallback.
- Product scheduler tests already assert `queueLatencyUs.p95 <= 2_000_000`, rejected count `0`, fallback count `0`, and visible center hash changes under pressure.
- Rust/testkit large-timeline assertions already prefer structural boundedness over wall-clock timing; Phase 20 should extend that pattern instead of relying on fixed machine-specific time budgets.

### Integration Points

- Add or extend desktop Playwright coverage under `apps/desktop-electron/tests/` for a v1.1 long-session packaged UAT.
- Add or extend testkit helpers under `crates/testkit/` to materialize the `180 segments/track x 3 tracks` product fixture and larger `1000`/`3000 segments/track` pressure cases.
- Add or extend product evidence helpers near `apps/desktop-electron/tests/helpers/userJourney.ts` or `realWorkflow.ts` for semantic project comparison, evidence bundle writing, sampled frame validation, and telemetry summaries.
- Extend no-fallback/source guard scripts rather than relying only on runtime Playwright assertions.

</code_context>

<specifics>
## Specific Ideas

- Product long-session UAT should open a Rust-generated `.veproj` instead of constructing all 540 segments through the UI.
- The product UAT core edit set is intentionally limited to baseline editing operations. Detailed Phase 19 effect parity is deferred to Phase 23.
- The evidence bundle should be useful both for product acceptance and for debugging: lightweight on success, full trace/screenshot/video/details on failure.

</specifics>

<deferred>
## Deferred Ideas

- Crop/export parity decisions are deferred to Phase 22.
- Existing Phase 19 effect/filter/transition/mask/blend parity and diagnostics taxonomy are deferred to Phase 23.
- Shortcut and high-frequency interaction hardening are deferred to Phase 21.
- UI polish and final acceptance sweep are deferred to Phase 24.

</deferred>

---

*Phase: 20-Long Timeline Product UAT And Guard Baseline*
*Context gathered: 2026-06-28*
