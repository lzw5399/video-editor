# Domain Pitfalls: v1.1 Usability And Export

**Domain:** Desktop-first Jianying-style video editor with Rust-owned core, realtime preview, and FFmpeg export  
**Researched:** 2026-06-27  
**Mode:** Project risk research for v1.1 PITFALLS/RISKS  
**Overall confidence:** MEDIUM for roadmap relevance; LOW if evaluated only by the generic local-source confidence seam. The findings are cross-checked against project-local v1.0 roadmap, requirements, validation, review, and audit artifacts, but no new v1.1 long-session stress run exists yet.

## Executive Summary

v1.1 should be treated as a product truth milestone, not a feature expansion milestone. v1.0 established the correct ownership direction: Rust project sessions own draft/timeline/preview/export semantics; high-frequency interactions are provisional Rust sessions; preview/export success cannot be proven by DOM, fallback, artifact, CPU, mock, or native single-video evidence; and external adapter data stays outside first-party render semantics. The main v1.1 risk is not "missing features"; it is false confidence from permissive UAT, isolated tests, or UI polish that makes partially backed behavior look complete.

The highest-risk areas are sustained editing pressure, not one-off operations. Long timelines, repeated save/reopen/export cycles, drag-heavy sessions, playhead scrubbing while export/artifact jobs run, effect parameter dragging, crop/export, and external template fixtures can expose gaps that v1.0 aggregate gates did not have to keep open for minutes at a time. Phase 17.1 proved the session model for representative interactions; v1.1 must prove it stays correct under messy, repeated product workflows.

The roadmap should make Phase 20+ gates fail the bad state. Passing unit tests or screenshot checks are insufficient when the product path can still regress to full graph recompute, repeated canonical commits, stale preview presentation, crop compiler errors, unsupported effect success, provider ID leakage, or product copy that hides a real implementation gap.

## Top Risks Ranked By Severity

| Rank | Risk | Severity | Confidence | Why It Matters For v1.1 |
|------|------|----------|------------|--------------------------|
| 1 | False product success from fallback, DOM, artifact, or permissive E2E evidence | Critical | MEDIUM | v1.1 is explicitly about exposing real gaps; permissive gates would hide the gap. |
| 2 | High-frequency interactions degrade into canonical command/save/undo storms | Critical | MEDIUM | Drag, scrub, crop, retime, and effect edits must be live without committing every sample. |
| 3 | Long timelines regress to full recompute, queue starvation, or stale state | Critical | MEDIUM | Phase 13/16 foundations exist, but v1.1 needs real product pressure gates. |
| 4 | Preview/export drift, especially crop/export mismatch and effect capability mismatch | Critical | MEDIUM | A known crop export limitation was deferred; effects parity must close reliability, not breadth. |
| 5 | Unsupported/degraded effects report as success or become invisible product failures | High | MEDIUM | Phase 19 relies on capability-backed diagnostics; v1.1 must make those diagnostics user-valid and test-valid. |
| 6 | UI polish hides architecture gaps | High | MEDIUM | v1.1 includes UI cleanup, but every visible control must remain backed, gated, or fail closed. |
| 7 | External adapter semantics leak into first-party core | High | MEDIUM | Kaipai/Jianying imports must remain adapters plus reports, not internal capability definitions. |
| 8 | Save/reopen/revision correctness erodes under repeated edits and autosave | High | MEDIUM | The canonical `.veproj/project.json` guarantee matters most after many edits, not after one happy path. |
| 9 | Interaction/session/native-handle lifetimes leak or outlive selection/project changes | High | MEDIUM | Prior Phase 19 review found and fixed orphaned production sessions; v1.1 should guard against recurrence. |
| 10 | Product diagnostics are either too raw for users or too thin for debugging | Medium | MEDIUM | v1.1 asks for better diagnostics; the risk is leaking backend jargon or suppressing actionable failure facts. |

## Critical Pitfalls

### Pitfall 1: False Product Success

**What goes wrong:** Product gates pass even though the real editing/rendering chain did not work. The test accepts DOM overlay movement, playhead advancement, first-frame snapshots, fallback artifacts, CPU hashes, mock/offscreen output, native single-video playback, or a file existing on disk as proof of preview/export behavior.

**How it shows up in product use:**

- User drags or edits and sees a UI overlay move, but native `renderGraphGpuComposited` output does not change.
- Playback time advances while the preview image is stale or black.
- Export creates a file, but it omits a visible crop, mask, retime, transition, filter, or text state.
- Screenshots look polished while actual media output still uses a fallback path.

**Requirement / phase guardrail:**

- Add `V11-UAT-01`: Every v1.1 visible behavior changed or claimed must be proven through normal Electron product flow with real fixture media, real timeline state, real preview evidence, saved `.veproj/project.json`, or exported media inspection.
- Add `V11-NOFALLBACK-01`: Product success cannot be satisfied by fallback/mock/artifact/CPU/DOM/screenshot-only evidence.
- Carry forward the v1.0 no-product-fallback guard as a required gate in every v1.1 phase.

**Tests / evidence that fail the bad state:**

- Product E2E must assert native preview evidence and visible preview-region motion after edit/time changes, not only playhead labels.
- Export E2E must extract representative frames/audio metadata from the output and compare against expected visible semantics.
- Source guards must reject tests that assert CSS transforms, DOM overlays, artifact paths, or generic file existence as preview/export success.
- Existing gates to preserve: `pnpm run test:no-product-fallback`, Phase 17.1 DOM-only preview guard, Phase 19 source guard against fallback success.

**Red flags:**

- Test names say "preview/export parity" but do not inspect preview evidence or exported media.
- A gate asserts `status: success` without checking capability diagnostics.
- UI screenshot regression is used as the only acceptance evidence for behavior.
- Developer diagnostics are toggled on during product UAT.

### Pitfall 2: High-Frequency Interaction Commit Storms

**What goes wrong:** Dragging transform/crop/keyframe/effect/retime/trim/playhead controls routes every mouse move through canonical project intents. This increments revision, saves/autosaves, invalidates caches, pushes undo entries, and schedules preview/export work for obsolete states.

**How it shows up in product use:**

- Timeline drag or slider movement stutters badly.
- Undo requires dozens of presses for one drag.
- `.veproj/project.json` is rewritten repeatedly during a pointer move.
- Export or preview queue fills with obsolete samples.
- Mouseup commits a value different from the visible final provisional state.

**Requirement / phase guardrail:**

- Add `V11-INTERACT-01`: Every drag/slider/scrub/keyframe/effect/crop/retime surface must use Rust-owned interaction sessions with base revision, generation, monotonic sequence, provisional deltas, stale rejection, cancel, and one canonical commit.
- Add `V11-UNDO-01`: During interaction updates, project revision, undo stack, redo stack, and save/autosave counters remain unchanged; commit applies exactly one canonical mutation where the interaction is draft-mutating.
- Add `V11-LIVE-01`: Live feedback must be driven by Rust provisional snapshots/deltas and realtime preview refresh, not by mouseup-only local ghosts.

**Tests / evidence that fail the bad state:**

- Simulated 300-1000 sample drags for transform, crop where exposed, trim, retime, effect slider, keyframe marker, and playhead scrub.
- Assertions: `revisionUnchanged` for updates, no save before commit, one undo item after commit, stale/out-of-order samples rejected, cancel leaves canonical draft unchanged.
- Source guard: reject pointer handlers that call canonical project intents, save, undo, revision increment, export, or direct draft mutation.
- Telemetry: coalesced obsolete samples, bounded queue latency, stale rejection count, final committed value equals last accepted provisional sample.

**Red flags:**

- Debounce is used as the ownership model instead of Rust interaction sessions.
- UI local state is the only live feedback.
- Tests only cover one short drag with a few samples.
- Pointer cancel/unmount/selection change paths lack explicit session cancel.

### Pitfall 3: Long Timeline Full-Recompute And Starvation

**What goes wrong:** Long drafts or mixed media timelines cause localized edits to invalidate the whole graph/cache/artifact set or starve interactive preview while export, probe, thumbnail, waveform, or artifact jobs run.

**How it shows up in product use:**

- 200+ segment timeline becomes unusable after a trim or effect parameter drag.
- Playhead scrubbing lags while export or artifact generation runs.
- Cache hit rate collapses after tiny edits.
- Preview shows stale frames because older scheduled jobs complete after newer seeks.
- Save/reopen appears correct for small fixtures but loses responsiveness with real user projects.

**Requirement / phase guardrail:**

- Add `V11-LONG-01`: v1.1 must include long-timeline product UAT with mixed video/image/audio/text/subtitle/effect/transition/retime segments and repeated localized edits.
- Add `V11-INCR-01`: Accepted command deltas must drive targeted dirty ranges and consumer domains for preview, export prep, audio, thumbnails, waveforms, proxies, and caches.
- Add `V11-SCHED-01`: Export/artifact/probe/filesystem work must not block playhead scrub, inspector interaction, or preview frame delivery on supported hardware.

**Tests / evidence that fail the bad state:**

- Rust large-timeline fixtures measuring graph diff cost, dirty range accuracy, and cache reuse after localized edit, undo, redo, material replacement, and retime changes.
- Packaged Electron stress flow: import fixture set, build long mixed timeline, scrub while export/artifact generation is active, drag edit controls, save/reopen, export.
- Telemetry budgets: queue latency p95, stale rejection count, dropped-frame budget, cache hit rate, first-frame/seek latency, export admission/cancel behavior.
- Gate fails if one localized edit marks full draft dirty without an explicit full-draft fallback reason.

**Red flags:**

- New code computes dirty/cache facts in React or Electron main.
- Tests use only single-video or tiny two-segment projects.
- Scheduler telemetry exists but no gate asserts budgets.
- Full invalidation is accepted without reason or without cost measurement.

### Pitfall 4: Preview/Export Drift And Crop Export Mismatch

**What goes wrong:** Preview and export consume the same high-level draft but diverge in implementation details: crop coordinate conversion, source dimensions, retime source mapping, mask alpha, blend math, effect parameter normalization, text fonts, transition overlap, or source time clipping. A known Phase 19 fixture removed crop to avoid an invalid FFmpeg crop against small desktop test media; v1.1 must close that gap directly.

**How it shows up in product use:**

- Crop looks correct in preview but export fails with invalid FFmpeg crop dimensions.
- Export succeeds but crop is clamped differently than preview.
- Retimed audio/video syncs in preview but drifts in export.
- Mask/blend/filter preview looks supported but export emits unsupported diagnostics or silently falls back.
- A template import fixture avoids crop, making parity look better than real user projects.

**Requirement / phase guardrail:**

- Add `V11-CROP-01`: Crop semantics must validate source-space dimensions against decoded source dimensions before FFmpeg runtime execution, with explicit diagnostics for impossible crops.
- Add `V11-PARITY-01`: Preview and export parity gates must cover crop, transform, retime, transition, first-party filters, masks, blend classifications, text, and audio follow-speed for the existing Phase 19 capability set.
- Add `V11-EXPORT-01`: Export validation must inspect representative media frames and classified diagnostics, not only duration/fps/resolution/file existence.

**Tests / evidence that fail the bad state:**

- Dedicated crop compiler tests for tiny, portrait, landscape, square, rotated, fit/fill/stretch, keyframed, and out-of-bounds crop cases.
- Product E2E re-enables crop in the Kaipai-like template fixture or adds a focused crop fixture that reaches real export.
- Export must fail before FFmpeg execution when semantic crop is invalid and must report a product-safe diagnostic.
- Preview/export frame comparison samples before, during, and after cropped segment ranges.

**Red flags:**

- A fixture removes crop "to keep the gate focused" in v1.1.
- Crop is implemented as UI-only rectangle math.
- FFmpeg runtime is allowed to discover crop invalidity first.
- Preview and export use different parameter normalization or rounding policies.

### Pitfall 5: Effects Diagnostics Become Fake Support

**What goes wrong:** Capability registry, preview support, and export support states blur together. Unsupported or degraded effects, transitions, masks, blends, or provider-native concepts either appear as supported in product UI or fail without enough diagnostic context.

**How it shows up in product use:**

- User applies an effect because the UI card is active, but preview/export silently omits it.
- Export failure gives raw FFmpeg/filter text or no explanation.
- Compatibility report says supported while canonical project JSON stores provider-native IDs.
- Unsupported blend mode uses normal overlay fallback but reports success.

**Requirement / phase guardrail:**

- Add `V11-FX-01`: Every exposed Phase 19 effect/retime/transition/mask/blend control must show product-safe preview/export support state derived from Rust capability facts.
- Add `V11-FX-02`: Unsupported/degraded export paths must fail closed or report degradation explicitly; they cannot mark product success true.
- Add `V11-FX-03`: v1.1 may improve reliability and diagnostics for existing Phase 19 capability set, but must not expand library breadth until preview/export parity for that set is stable.

**Tests / evidence that fail the bad state:**

- Capability matrix tests: supported preview + supported export, supported preview + unsupported export, degraded preview/export, unsupported provider-native.
- Desktop E2E asserts visible effect cards/controls are either backed by support facts or product-gated.
- Export metadata and compatibility report must include bounded support/degradation facts.
- Source guard rejects FFmpeg effect strings outside `ffmpeg_compiler`, renderer-owned effect evaluation, provider-native IDs as first-party capability kinds, and "normal overlay" success for unsupported blends.

**Red flags:**

- UI implementation adds more effect cards without adding capability rows and product E2E.
- Diagnostics are only visible in developer mode when the default user needs an actionable failure.
- "Supported" is inferred from the presence of a semantic field rather than registry support.
- Tests assert only that an effect parameter is persisted.

## High Pitfalls

### Pitfall 6: UI Polish Masks Architecture Gaps

**What goes wrong:** v1.1 UI cleanup improves layout, density, copy, or visual affordances while making unsupported or partially implemented behavior look usable. Product polish can unintentionally weaken the no-fallback policy by hiding failure states or exposing controls without production backing.

**How it shows up in product use:**

- Crop, transition, fade, fullscreen, ratio, advanced effect, or provider-template controls appear active but do not affect preview/export.
- Product copy says "ready" while diagnostics show unavailable backend/capability.
- Screenshot regression passes because the UI is clean, but source guards or media evidence would fail.
- UI work reintroduces renderer projections, direct draft reads, or local capability decisions.

**Requirement / phase guardrail:**

- Add `V11-UI-01`: UI polish phases must include source guards and product evidence for every behavior-changing control they touch.
- Add `V11-UI-02`: Unsupported default controls are hidden or product-gated; they cannot look like available production actions.
- Add `V11-UI-03`: Default UI remains product-safe, but developer diagnostics must be reachable when diagnosing export/effect failures.

**Tests / evidence that fail the bad state:**

- Screenshot regression at required desktop sizes plus behavior assertions for all visible controls touched by the phase.
- Source guard rejects product UI copy that exposes backend/cache/render-graph raw jargon by default and rejects unsupported controls styled as active.
- Accessibility labels/tooltips cannot leak raw diagnostics in default mode.
- Product E2E must click visible controls changed by UI work and prove backed behavior or disabled/gated state.

**Red flags:**

- A UI-only phase changes a control that has no backing test.
- "Temporarily disabled" controls are styled like normal active cards.
- Developer diagnostics are removed instead of moved behind an explicit diagnostic surface.
- New copy invents non-Jianying terminology for core concepts.

### Pitfall 7: External Adapter Leakage Into Core Semantics

**What goes wrong:** Kaipai/Jianying/CapCut provider IDs, native effect names, raw formula JSON, recognizer output, remote URLs, or proprietary transition/filter names become internal first-party draft/render/effect semantics. This creates a parity chase and weakens the self-owned core.

**How it shows up in product use:**

- `.veproj/project.json` stores provider-native IDs as if they are first-party effects.
- Export compiler branches on `kaipai` or provider-specific strings.
- Compatibility report raw payloads appear in product UI.
- Unsupported provider effects are represented as supported filters or transitions.
- Roadmap expands toward 1:1 proprietary parity before first-party reliability closes.

**Requirement / phase guardrail:**

- Add `V11-ADAPTER-01`: External draft/template adapters may output provider-neutral import plans, localized resources, and compatibility reports only; canonical draft/render/effect semantics remain first-party.
- Add `V11-ADAPTER-02`: Product E2E for imported templates must prove canonical project JSON is provider-ID free except bounded provenance/report evidence.
- Add `V11-SCOPE-01`: v1.1 does not chase 100% proprietary parity or live provider integration.

**Tests / evidence that fail the bad state:**

- Source guard scans core/render/session/export paths for provider-specific terms except explicit adapter/report boundaries.
- Imported template tests inspect canonical `.veproj/project.json`, render graph intent, FFmpeg compiler input, and exported media evidence.
- Compatibility report UI tests assert bounded product copy and no raw provider payload/URL exposure.
- Negative fixtures with provider-native effects must produce unsupported/degraded report facts, not first-party semantics.

**Red flags:**

- A first-party enum variant is named after a provider-native concept.
- `ffmpeg_compiler` or `realtime_preview_runtime` contains adapter-specific branches.
- Import success is measured by file creation without report and preview/export evidence.
- Roadmap language promises "same as Jianying" instead of supported/degraded/unsupported subsets.

### Pitfall 8: Save/Reopen/Revision Storms Under Real Editing

**What goes wrong:** The project remains correct for short command tests but fails under repeated edit/save/reopen/export cycles. Interaction updates, autosave, undo/redo, material probes, template imports, and export prep can race or clear active sessions incorrectly.

**How it shows up in product use:**

- Reopened draft differs from visible state before save.
- Undo stack contains interaction samples or misses the final committed action.
- Autosave writes while provisional state is active.
- Active interaction continues after another canonical mutation, template import, material probe completion, undo/redo, or project close.
- Export starts from a stale expected revision.

**Requirement / phase guardrail:**

- Add `V11-SAVE-01`: v1.1 UAT must repeat edit/save/reopen/export loops across long mixed timelines and compare canonical project JSON semantics before and after reopen.
- Add `V11-REV-01`: Canonical revision increments must clear or reject active interaction sessions; export/save APIs must use expected revision.
- Add `V11-UNDO-02`: Undo/redo restores semantic state and deterministic dirty facts after long sessions.

**Tests / evidence that fail the bad state:**

- Repeated product flow: import, build mixed timeline, perform 50+ interactions, save, close, reopen, scrub, undo/redo, save again, export.
- Assert project JSON is semantic-only: no render graphs, FFmpeg scripts, thumbnails, waveforms, preview caches, provider runtime refs, or provisional interaction state.
- Export start rejects stale expected revision and reads draft from Rust session, not renderer payload.
- Undo/redo checks include dirty range and preview/export invalidation after reopen.

**Red flags:**

- Save code receives a full draft from renderer.
- Provisional interaction state appears in persisted JSON.
- Tests use one save/reopen only at the end of a happy path.
- Active session cleanup is tied only to mouseup.

### Pitfall 9: Session, Handle, And Native Resource Lifetime Leaks

**What goes wrong:** Interaction sessions, preview/audio/export sessions, native frame/texture handles, or listener cleanup survive selection changes, unmount, project close, export cancel, surface detach, or stale playback generation changes.

**How it shows up in product use:**

- Changing selection mid-drag applies the final commit to the wrong segment/effect.
- Escape/cancel appears to work but a later async completion mutates state.
- Preview or audio keeps using old generation after seek/project switch.
- Native frame/texture/resource leak diagnostics appear on session close.
- Export cancel leaves queued jobs running.

**Requirement / phase guardrail:**

- Add `V11-LIFETIME-01`: Every session-like object must have deterministic owner, generation, cancel/close path, and stale completion rejection.
- Add `V11-TARGET-01`: Destructive confirmations and commits are scoped to stable target identities and revalidate before mutation.
- Add `V11-HANDLE-01`: Native media/preview handles remain opaque, owner/generation checked, explicitly released, and leak-reported on close.

**Tests / evidence that fail the bad state:**

- E2E: start drag/effect edit, change selection, unmount panel, press Escape, close project, import template, undo/redo, then assert no stale commit.
- Rust tests for wrong-owner, stale-generation, unknown-handle, double-release, and session-close leak diagnostics.
- Export/audio/preview cancellation tests assert queued stale jobs cannot present or mutate visible state.
- Source guards reject native pointers, GPU handles, frame bytes, or raw surfaces crossing to renderer.

**Red flags:**

- Async handler clears global active state without checking ownership.
- Confirmation state does not include target identity.
- Unmount cleanup cancels React state but not Rust sessions.
- Handle release is best-effort with no diagnostic count.

## Moderate Pitfalls

### Pitfall 10: Diagnostics Are Either Too Raw Or Too Thin

**What goes wrong:** Product-facing diagnostics leak raw backend terms, paths, FFmpeg filter strings, render graph/cache internals, or provider payloads; or the product hides all useful facts and gives the user no way to recover from export/effect/crop failures.

**How it shows up in product use:**

- Export failure displays raw FFmpeg command text but not the unsupported crop/effect reason.
- Effect card says unavailable with no preview/export distinction.
- Compatibility report lists raw provider JSON or private IDs.
- Product support cannot diagnose whether failure was missing material, unsupported capability, stale revision, invalid crop, or runtime unavailable.

**Requirement / phase guardrail:**

- Add `V11-DIAG-01`: Product diagnostics are bounded, localized, recovery-oriented, and derived from Rust typed diagnostics.
- Add `V11-DIAG-02`: Developer diagnostics remain available behind explicit developer mode for logs, queue telemetry, runtime state, and export traces.
- Add `V11-DIAG-03`: Unsupported/degraded/failure reports include machine-readable codes for tests and product-safe copy for users.

**Tests / evidence that fail the bad state:**

- UI tests assert default product copy hides raw backend/cache/render graph/path/log strings.
- Export/effect/crop failure E2E asserts user-visible message, machine-readable code, and developer detail availability.
- Report navigation tests assert supported rows focus/seek canonical targets and unsupported rows remain report-only.

**Red flags:**

- Raw FFmpeg stderr is the only diagnostic shown.
- Product UI has no way to distinguish unsupported export from runtime failure.
- Tests check text presence but not diagnostic code/source.

### Pitfall 11: Phase 19 Breadth Expands Before Reliability Closes

**What goes wrong:** v1.1 adds more effects, transitions, filters, templates, or provider mappings before closing existing preview/export reliability gaps, crop diagnostics, and support-state clarity.

**How it shows up in product use:**

- The library looks larger but more cards are unsupported or export-degraded.
- Regression surface grows faster than parity tests.
- Existing crop/effect failures remain buried behind new capability breadth.

**Requirement / phase guardrail:**

- Add `V11-SCOPE-02`: v1.1 may add only the minimum fixtures or controls needed to prove reliability/diagnostics for existing Phase 19 capability set.
- Add `V11-FX-04`: Any new effect/filter/transition added in v1.1 requires preview support, export support or explicit degraded diagnostics, product E2E, and compatibility report behavior in the same phase.

**Tests / evidence that fail the bad state:**

- Capability registry diff check in every v1.1 phase: new supported capability must include preview, export, diagnostics, and product E2E evidence.
- Roadmap review gate rejects phases whose primary output is effect-library breadth instead of reliability closure.

**Red flags:**

- Phase title says "more effects" before crop/export diagnostics are fixed.
- New cards are marked supported without exported-media evidence.
- Adapter fixture coverage increases but first-party parity matrix does not.

## Requirement Seeds For v1.1

| Proposed ID | Requirement | Catches |
|-------------|-------------|---------|
| V11-UAT-01 | Normal product E2E must cover long mixed timelines, repeated edits, save/reopen, scrub, and export with real fixture media. | False success, save/reopen drift |
| V11-NOFALLBACK-01 | Product success cannot be satisfied by fallback, mock, artifact, CPU probe, DOM overlay, first-frame snapshot, or file-exists-only proof. | Fallback masking |
| V11-LONG-01 | Long-timeline tests must verify graph diff cost, dirty range accuracy, queue latency, stale rejection, cache reuse, and export consistency. | Full recompute, starvation |
| V11-INTERACT-01 | High-frequency controls must use Rust interaction sessions with provisional updates, stale rejection, cancel, and one commit. | Commit/save/undo storms |
| V11-LIVE-01 | Live feedback must update from Rust provisional view models/deltas and preview snapshots before mouseup. | Mouseup-only ghost UI |
| V11-SAVE-01 | Repeated edit/save/reopen/export loops must preserve canonical semantics and reject stale expected revisions. | Autosave/revision drift |
| V11-CROP-01 | Crop export must validate/clamp/reject against decoded source dimensions before FFmpeg runtime execution. | Known crop export failure |
| V11-PARITY-01 | Preview/export parity must cover crop, transform, retime, transitions, filters, masks, blends, text, and audio for existing supported Phase 19 semantics. | Preview/export drift |
| V11-FX-01 | Every exposed effect/control must display Rust capability-backed preview/export support or be product-gated. | Fake support |
| V11-DIAG-01 | Export/effect/crop diagnostics must have typed codes, product-safe copy, and developer detail behind explicit diagnostics. | Bad diagnostics |
| V11-ADAPTER-01 | External adapter IDs and raw provider payloads remain adapter/report/provenance only. | Adapter leakage |
| V11-UI-01 | UI polish phases must click every visible changed control and prove backed behavior or gated state. | Polished gaps |
| V11-LIFETIME-01 | Sessions/handles/listeners must cancel deterministically on selection change, unmount, close, undo/redo, template import, and stale generation. | Leaks/stale commits |

## Recommended Phase Gates

### Phase 20: Real Editing UAT And Long Timeline Stress

**Purpose:** Make v1.1 start by exposing product truth under realistic editing pressure.

**Required gates:**

- Product E2E builds a long mixed timeline with video/image/audio/text/subtitle/effects/transitions/retime/crop where supported.
- Performs repeated move/trim/split/delete/undo/redo, playhead scrub, inspector drag, effect parameter drag, save/reopen, and export.
- Asserts preview evidence, exported media frames, canonical project JSON, queue telemetry, dirty ranges, and no fallback success.
- Fails if tests use seeded demo shortcuts not available in normal product flow.

### Phase 21: Interaction Session Hardening Under Pressure

**Purpose:** Extend Phase 17.1 interaction correctness from representative cases to sustained drag-heavy sessions.

**Required gates:**

- 300-1000 sample interactions for transform, trim, keyframe, playhead, retime, effect sliders, and crop where exposed.
- No revision/save/undo changes during updates.
- One canonical commit, one undo item, cancel/stale rejection, bounded queue latency.
- Source guard rejects canonical intent loops in high-frequency handlers.

### Phase 22: Crop, Export, And Existing Effects Parity Closure

**Purpose:** Close reliability gaps in the existing Phase 19 capability set before adding breadth.

**Required gates:**

- Focused crop compiler validation against decoded source dimensions.
- Preview/export parity matrix for crop/transform/retime/transition/filter/mask/blend/text/audio.
- Real export frame extraction and diagnostics assertions.
- Re-enable crop fixture coverage or add a dedicated crop export fixture.
- Capability registry diff gate blocks unsupported new effects from appearing as supported.

### Phase 23: Product UI And Diagnostics Polishing With Architecture Guards

**Purpose:** Improve usability without hiding unsupported behavior or backend gaps.

**Required gates:**

- Screenshot regression at 1280x800, 1120x720, and one long-timeline crowded state.
- Click-through E2E for every visible changed control.
- Product diagnostics tests for export/effect/crop failures.
- Source guards for default UI debug-copy absence, unsupported active controls, renderer-owned semantics, and adapter leakage.

## Red Flag Checklist For Roadmap Reviews

- A phase claims "usability" but has no long-session product UAT.
- A phase claims "export parity" but validates only file existence, duration, fps, and resolution.
- A phase claims "preview" but checks only playhead movement, DOM overlays, screenshots, or CPU hashes.
- A phase changes drag/slider/scrub code without explicit Rust interaction-session tests.
- A phase fixes crop by changing a fixture instead of compiler validation and diagnostics.
- A phase adds effect cards or adapter mappings without preview/export capability and diagnostics rows.
- A phase exposes default UI controls that are not supported, gated, or tested.
- A phase accepts provider-native IDs in core/render/session/export paths.
- A phase stores derived artifacts, render graph facts, FFmpeg scripts, or provisional state in `project.json`.
- A phase passes with developer diagnostics enabled but fails normal product mode.

## Deferred Risks Not In v1.1

These should remain explicit non-goals unless the user changes scope:

- Full proprietary Jianying/CapCut/Kaipai pixel parity.
- Live provider integrations, remote template downloads, signed URL handling, account-backed resources, or cloud provider auth.
- Broad new effect/preset marketplace or large effect-library expansion.
- Full mobile app productization, app-store packaging, mobile permission UX, and mobile realtime preview UI.
- Cloud rendering product UX, remote cache sync, collaborative editing, or server fleet operations.
- AI oral-video workflows, ASR, auto highlight generation, template intelligence, or digital-human workflows.
- Direct Kdenlive/MLT runtime integration or copied GPL presets/assets/XML definitions.
- Perfect parity for proprietary text bubbles, VIP fonts, provider-native effects, encrypted drafts, or private resource IDs.

## Source Notes

The requested `17.1-REVIEW.md` file was not present at the provided path. Adjacent Phase 17.1 review/validation artifacts were used: `17.1-UI-REVIEW.md`, `17.1-VALIDATION.md`, and `17.1-VERIFICATION.md`.

Primary local sources:

- `.planning/PROJECT.md`: v1.1 goal, active scope, architecture/no-fallback constraints, key decisions around sessions and adapter boundaries.
- `.planning/STATE.md`: accumulated decisions for session-owned view models, preview/export boundaries, no generic command IPC, scheduler ownership, adapter isolation, and Phase 19 capability discipline.
- `.planning/milestones/v1.0-ROADMAP.md`: Phase 13 large-timeline incremental requirements, Phase 16 scheduler requirements, Phase 17/17.1 adapter/session requirements, Phase 19 production effects requirements.
- `.planning/milestones/v1.0-REQUIREMENTS.md`: canonical v1.0/v2 requirements for preview/export parity, no fallback success, scheduler telemetry, interaction ownership, and production effects.
- `.planning/milestones/v1.0-MILESTONE-AUDIT.md`: v1.0 closure debt, especially known crop export limitation and deferred proprietary parity.
- `.planning/milestones/v1.0-phases/17.1-interaction-session-and-template-import-main-chain-hardening/17.1-UI-REVIEW.md`: original UI/session risks around local debounce, static reports, unsupported visible controls, and unit leakage.
- `.planning/milestones/v1.0-phases/17.1-interaction-session-and-template-import-main-chain-hardening/17.1-VALIDATION.md`: interaction matrix and guard expectations for no-save/no-undo, live provisional state, stale rejection, and DOM/fallback rejection.
- `.planning/milestones/v1.0-phases/17.1-interaction-session-and-template-import-main-chain-hardening/17.1-VERIFICATION.md`: passed evidence for Phase 17.1 after native rotate and hit-testing gap closure.
- `.planning/milestones/v1.0-phases/19-production-effects-retiming-and-transition-semantics/19-REVIEW.md`: resolved risks for orphaned sessions, destructive target drift, pointer save-loop guard, and aggregate coverage.
- `.planning/milestones/v1.0-phases/19-production-effects-retiming-and-transition-semantics/19-VALIDATION.md`: Phase 19 source guard, capability, preview/export, UI audit, and non-blocking warning evidence.
- `.planning/milestones/v1.0-phases/19-production-effects-retiming-and-transition-semantics/deferred-items.md`: known crop export limitation and suggested focused crop compiler guard.

External web search was attempted through the GSD research-plan seam. It did not provide authoritative project-specific architecture evidence, so the conclusions above rely on cross-checked project-local artifacts rather than generic web advice.
