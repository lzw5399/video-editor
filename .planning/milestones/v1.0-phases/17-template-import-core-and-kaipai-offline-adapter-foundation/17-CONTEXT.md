# Phase 17: Template Import Core And Kaipai Offline Adapter Foundation - Context

**Gathered:** 2026-06-24
**Status:** Ready for planning
**Source:** User-confirmed Phase 17 direction plus current-main and old `origin/work/kaipai-adapter-poc` inspection.

<domain>
## Phase Boundary

Phase 17 establishes provider-neutral template import and approximate template rendering foundations, then brings Kaipai in only as an offline external input adapter. The product target is a local, editable `.veproj` draft that can preview and export through the current Video Editor pipeline with an explicit adaptation report. The phase does not pursue pixel-perfect Kaipai reproduction, live Kaipai API integration, Android worker runtime rendering, or provider-specific render semantics in core crates.

</domain>

<decisions>
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

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Phase Scope
- `.planning/ROADMAP.md` Phase 17 section - Current phase slot and dependency on Phase 16.
- `.planning/STATE.md` Current Position and urgent Phase 17 note - Confirms provider-neutral template import plus Kaipai offline adapter foundation.
- `.planning/PROJECT.md` Constraints and Key Decisions - `.veproj` canonical format, Rust-owned semantics, compatibility adapters/reports, no product fallback, no legacy compatibility by default, product E2E acceptance.
- `.planning/REQUIREMENTS.md` Compatibility and Production Effects sections - `COMP-01` through `COMP-03` remain external compatibility targets; `PRODFX-05` covers complex Jianying/Kaipai-like fixtures and fallback reports.

### Mandatory Policies
- `docs/no-product-fallback-policy.md` - Product success cannot rely on fallback, mock, artifact, CPU, or legacy evidence.
- `docs/refactor-and-legacy-cleanup-policy.md` - Replace old paths with current architecture instead of preserving obsolete compatibility.
- `docs/product-e2e-acceptance-policy.md` - User-visible behavior requires product workflow proof.
- `docs/runtime-boundaries.md` - Rust-owned preview/session/render boundaries and pure semantic crate constraints.

### Current Main Integration Points
- `crates/draft_model/src/canvas.rs` - Canonical canvas config, aspect ratio, frame rate, background, and normalized canvas coordinates.
- `crates/draft_model/src/timeline.rs` - Track/segment/text/audio/keyframe/transform/fit/crop/anchor/rotation/opacity semantics.
- `crates/draft_model/src/font_registry.rs` - Bundled font refs and font validation/fallback pattern.
- `crates/project_store/src/bundle.rs` - `.veproj/project.json` create/save/open and draft validation.
- `crates/project_store/src/paths.rs` - Material URI classification and bundle-relative path safety.
- `crates/artifact_store/src/resource_index.rs` - Resource index shape for materials, fonts, effects, filters, transitions, and derived resources.
- `crates/bindings_node/src/project_session_service.rs` - Current project-session command boundary, draft revision ownership, material import, canvas, text, audio, visual, and keyframe intents.
- `crates/render_graph/` - Renderer-neutral graph intent generated from canonical draft semantics.
- `crates/ffmpeg_compiler/` - Export compiler that must not interpret provider formulas directly.
- `crates/realtime_preview_runtime/` and `crates/preview_service/` - Supported realtime preview path and capability evidence boundaries.

### Old Branch Assets To Inspect, Not Blindly Merge
- `origin/work/kaipai-adapter-poc:crates/adapter_kaipai/` - Old offline formula bundle, resource localizer, compatibility report, and tests.
- `origin/work/kaipai-adapter-poc:fixtures/kaipai/` - Positive/negative fixture corpus and expected reports.
- `origin/work/kaipai-adapter-poc:schemas/kaipai-formula-bundle.schema.json` - Old input bundle schema.
- `origin/work/kaipai-adapter-poc:schemas/compatibility-report.schema.json` - Old report schema.
- `origin/work/kaipai-adapter-poc:.codex/skills/spike-findings-video-editor-kaipai-adapter/` - Spike findings and compatibility boundary.
- `origin/work/kaipai-adapter-poc:.planning/phases/03.1-kaipai-compatibility-foundation-offline-formula-fixtures-com/` - Old plans, research, validation, and verification history.

### Kaipai Research
- `KAIPAI_FORMULA_ADAPTER_RESEARCH.md` - Chinese research memo for dcoin chain, formula availability, supported fields, resource localizer, compatibility report, and suggested sequencing.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- Current `draft_model` already includes canvas, transform, crop, anchor, rotation, fit mode, opacity, text style, text stroke/shadow/background, audio fade, volume, keyframes, filters, transitions, and bundled font refs.
- `project_store` already validates `.veproj/project.json` and material URI safety; import work should extend or reuse this boundary rather than duplicating path logic in UI.
- `artifact_store::resource_index` can index materials, fonts, effects, filters, transitions, proxies, thumbnails, waveforms, graph snapshots, and preview artifacts. Phase 17 should connect localized template resources to this resource-index pattern where appropriate.
- `project_session_service` owns session revision, project read/apply behavior, material import, canvas update, text editing, audio editing, visual transform, and keyframe intents. It is the right integration surface for applying a provider-neutral import plan.
- Old `adapter_kaipai` tests provide useful patterns for schema validation, path traversal rejection, sha256 mismatch, missing resource diagnostics, supported-source reports, degraded text style, unsupported formula blocks, and native-effect report entries.

### Established Patterns
- UI emits commands; Rust owns draft/project/timeline/preview/export semantics.
- `.veproj/project.json` stores canonical semantics only. Reports, resources, render graphs, caches, thumbnails, waveform data, proxy files, and preview/export artifacts are adjacent or derived.
- Time values in persisted semantics use integer microseconds or rational frame rates.
- Renderer and Electron main must not construct FFmpeg commands, render graphs, cache keys, fallback ladders, or timeline semantics.
- Product evidence must come from the real preview/export path, not fallback or diagnostic artifacts.
- Refactors should remove or gate obsolete paths rather than keeping compatibility layers that normal users can still exercise.

### Integration Points
- Add or expose provider-neutral import-plan types near the Rust session/application boundary, then let adapters emit those plans.
- Add `adapter_kaipai` as an external adapter crate only after confirming Cargo workspace shape and current schema-generation patterns.
- Connect resource localization to `.veproj/resources/...`, `project_store` path helpers, and `artifact_store` resource indexing without putting provider logic into `project_store`.
- Add binding/API command(s) in `bindings_node` for offline import only after Rust-side plan application and report generation are tested.
- Add UI/report panel after backend gates pass; default product UI must show supported/degraded outcomes in Chinese without exposing raw provider JSON or backend diagnostics.

</code_context>

<specifics>
## Specific Ideas

- The old branch proves the offline formula bundle boundary is available, but Phase 17 should update the naming from compatibility-only language toward provider-neutral `DraftImportPlan` and `AdaptationReport`.
- The first user-visible value is opening a Kaipai-derived draft locally, seeing a reasonable approximation quickly, editing generic draft elements, and exporting through Video Editor without Android worker dependency.
- The first fixture set should cover main video, PIP, text sticker, BGM/audio, and native effect degradation. Additional missing-resource and unsafe-path fixtures should protect the localizer.
- Keep Android oracle output only for calibration evidence. It must never be product runtime or acceptance evidence.

</specifics>

<deferred>
## Deferred Ideas

- Live Kaipai API/provider integration, auth, submit/poll, retries, and rate limits are deferred until offline import is stable.
- Android worker replacement, ASR-to-`word_list`, independent `safe_area` generation, and provider formula acquisition are separate future spikes or phases.
- Pixel-perfect Kaipai/Jianying/CapCut parity, proprietary native effects, complex flower text, beauty/matting/AR effects, and complex transitions remain out of Phase 17.
- Full UI import entry and report panel should wait until backend import/report/preview/export gates are stable.
- Advanced retiming/effect/transition engines remain downstream production effects work unless Phase 17 only needs a generic minimal subset for approximate import.

</deferred>

---

*Phase: 17-template-import-core-and-kaipai-offline-adapter-foundation*
*Context gathered: 2026-06-24*
