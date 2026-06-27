# Phase 4: Jianying-Style Desktop Workspace - Context

**Gathered:** 2026-06-17
**Status:** Ready for planning
**Mode:** auto-discuss

<domain>
## Phase Boundary

Phase 4 replaces the current Electron smoke workbench with the first real desktop editor workspace. It delivers a Jianying/CapCut-like shell with top feature categories, left material/function panel, center preview region, right inspector, and bottom multi-track timeline; wires visible edit actions through generated Rust command contracts; uses Simplified Chinese for user-facing desktop copy; and adds Playwright/visual layout gates for the workspace. It does not implement preview frame rendering, waveform generation, render graph compilation, FFmpeg export, packaged app release, mobile UI, cloud rendering, or advanced proprietary effect parity.

</domain>

<decisions>
## Implementation Decisions

### Workspace Structure And Visual Direction
- **D-01:** The first screen is the editor workspace, not a landing page, dashboard, or marketing page. It should resemble a restrained Jianying desktop editor: top feature categories, left material/function library, center player/preview monitor, right property inspector, and bottom timeline.
- **D-02:** The UI may do MVP simplification, but it must still look intentionally editor-like. Avoid generic cards/dashboard composition, decorative hero sections, large marketing copy, floating page-section cards, gradient-orb decoration, and one-note palettes.
- **D-03:** Top categories should visibly reserve Jianying-style areas: media/material, audio, text, stickers, effects, transitions, filters, and adjustment. MVP may disable or show empty-state panels for later categories, but the category vocabulary and layout should be present early.
- **D-04:** Use compact professional controls with stable dimensions. Timeline rows, category buttons, toolbars, counters, transport controls, and inspector fields must not resize or shift during selection, hover, drag, or playback state changes.

### Chinese Product Language And Terminology
- **D-05:** Desktop UI visible copy is Simplified Chinese by default, including panel titles, buttons, empty states, error text, labels used by Playwright tests, and accessibility labels where they are user-facing.
- **D-06:** Product and UI terms should follow Jianying-style concepts consistently: draft, material, track, segment, source/target time range, main-track magnet, keyframe, text, sticker, effect, filter, transition, and adjustment. Do not introduce user-facing aliases such as Asset/Clip.
- **D-07:** Internal TypeScript identifiers may stay code-friendly and generated Rust contract names may remain English, but UI labels, test-visible text, and documentation for the desktop workflow should present the Chinese editing vocabulary.

### Panels, Inspector, And MVP Behavior
- **D-08:** The left panel should support material/media as the primary MVP category and include visible category affordances for text/audio/sticker/effect/transition/filter/adjustment. It should show imported materials with metadata and recoverable missing/probe-failed states from Rust responses.
- **D-09:** Text and audio panels should expose the Phase 3 command surface: add/edit text segment content/style, add audio/BGM material, segment volume, and track mute. Advanced text rendering, bubbles, text effects, waveform drawing, and export behavior remain later phases.
- **D-10:** The right inspector should reflect the current selection and edit only through Rust-owned command payloads. For no selection, show a useful Chinese empty state; for segment selection, expose properties supported by Phase 3 semantics without inventing preview/render-only state.
- **D-11:** The center preview region in Phase 4 is a stable monitor shell, not final preview rendering. It can show poster/placeholder state and binding/draft status, but real deterministic preview frames and playback cache belong to Phase 5.

### Timeline Interaction And Rust Command Boundary
- **D-12:** The renderer must treat `Draft`, `CommandState`, and `TimelineSelection` as state returned by Rust commands. It may display and pass them back, but it must not directly mutate `Draft.tracks`, interpret undo/redo inverse operations, compute snapping/MainTrackMagnet, or repair invalid timeline edits.
- **D-13:** Timeline actions for add/select/move/split/trim/delete, undo/redo, text edits, audio edits, volume, and track mute must call `window.videoEditorCore.executeCommand` with generated `CommandEnvelope` payloads and consume `TimelineCommandResponse`.
- **D-14:** Phase 4 drag interactions can be MVP-level. If full pointer drag editing is too large, use deterministic click/button controls plus clear visual segment/timeline state first, but keep the timeline surface shaped so richer drag behavior can be added without replacing the model.
- **D-15:** Invalid command results should surface as Chinese editor errors without local UI retries that hide the Rust error. The UI can keep the prior draft state when Rust rejects an edit.

### Verification And Quality Gates
- **D-16:** Phase 4 completion requires Playwright Electron coverage for the core workspace flow: app opens to Chinese Jianying-style workspace, material rows are visible, command-only timeline edits update the draft/timeline, inspector reflects selection, and no direct renderer mutation or FFmpeg construction appears.
- **D-17:** Visual/layout checks should cover at least desktop 1280x800 and a constrained minimum size. They must verify that top categories, left panel, preview, inspector, and timeline are visible, non-overlapping, stable, and using Chinese labels.
- **D-18:** Source guards should prevent direct mutation of `Draft.tracks`/segment arrays in renderer code, direct Electron/Node imports in renderer, renderer FFmpeg/ffprobe command construction, English-only user-facing labels for key Phase 4 UI, and generated contract drift.
- **D-19:** Use the existing `just build` and `just test` gates, and add Phase 4-specific scripts for workspace UI, source guards, and Playwright Electron checks before the phase is considered complete.

### the agent's Discretion
- The planner may choose the exact React component split, local reducer/store shape, CSS module/global CSS strategy, and fixture draft data used by Electron tests, as long as generated Rust contracts remain the source of truth.
- The planner may decide whether Phase 4 uses mock local draft fixtures or real project/material commands for every UI path, but any user-visible timeline edit acceptance must be proven through Rust command responses, not UI-only mutation.
- The planner may choose exact colors and spacing, but the result must read as a polished desktop video editor rather than a generic admin panel.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project Direction
- `.planning/PROJECT.md` - Product identity, Jianying terminology requirement, Simplified Chinese UI requirement, architecture constraints, and out-of-scope boundaries.
- `.planning/REQUIREMENTS.md` - Phase 4 requirements `UI-01` through `UI-06` and `TEST-06`.
- `.planning/ROADMAP.md` - Phase 4 goal, success criteria, dependency on Phase 3, and planned work slices.
- `.planning/STATE.md` - Current status, accumulated Phase 1-3 decisions, and Phase 4 focus.

### Prior Phase Artifacts
- `.planning/phases/01-foundation-and-golden-harness/01-CONTEXT.md` - Binding boundary, generated contracts, fixture gates, and pure semantic crate constraints.
- `.planning/phases/02-draft-and-material-system/02-CONTEXT.md` - `.veproj/project.json`, material metadata, missing material diagnostics, and Jianying-aligned schema vocabulary.
- `.planning/phases/03-timeline-command-core/03-CONTEXT.md` - Rust-owned timeline command decisions, undo/redo, snapping/MainTrackMagnet, text/audio semantics, and UI boundary constraints.
- `.planning/phases/03-timeline-command-core/03-VERIFICATION.md` - Evidence that Phase 3 command core is implemented and ready for UI integration.
- `.planning/phases/03-timeline-command-core/03-VALIDATION.md` - Phase 3 validation evidence and final automated gates.

### Research
- `.planning/research/SUMMARY.md` - MVP shape, Jianying-style editor experience, semantic pipeline, and test strategy.
- `.planning/research/ARCHITECTURE.md` - Layer responsibilities, Kdenlive/MLT/Jianying lessons, and semantic spine.
- `.planning/research/STACK.md` - Electron + React + TypeScript desktop recommendation, Rust binding strategy, and Playwright/Electron test direction.
- `.planning/research/PITFALLS.md` - Known traps around UI-owned semantics, duplicate state, terminology drift, time bugs, command drift, and preview/export drift.

### Local Source Boundaries
- `apps/desktop-electron/src/renderer/App.tsx` - Current smoke UI to replace with real workspace; shows existing generated contract imports and `window.videoEditorCore.executeCommand` usage.
- `apps/desktop-electron/src/renderer/styles.css` - Current global desktop layout CSS; can be replaced or expanded but should preserve stable desktop constraints.
- `apps/desktop-electron/src/preload/index.ts` - Safe preload bridge exposing only `videoEditorCore`.
- `apps/desktop-electron/src/main/index.ts` - Electron main process IPC sender allowlist and window constraints.
- `apps/desktop-electron/tests/electron-smoke.spec.ts` - Existing Playwright Electron smoke pattern to extend for Phase 4.
- `apps/desktop-electron/src/generated/CommandEnvelope.ts` - Generated command payloads for material and timeline commands.
- `apps/desktop-electron/src/generated/CommandResultEnvelope.ts` - Generated command result and `TimelineCommandResponse` types.
- `apps/desktop-electron/src/generated/Draft.ts` - Generated `Draft`, `Material`, `Track`, `Segment`, text, and volume semantic types.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `window.videoEditorCore` already exposes `ping`, `version`, and `executeCommand` through the preload bridge. Phase 4 should keep this as the only renderer-to-core path.
- Generated TypeScript contracts already include `importMaterial`, `listMaterials`, `listMissingMaterials`, add/select/move/split/trim/delete timeline commands, undo/redo, text commands, audio commands, segment volume, track mute, `CommandState`, and `TimelineSelection`.
- The existing Electron Playwright tests launch `dist/main/index.cjs`, inspect the preload bridge, and verify renderer source guards. Phase 4 should extend this pattern rather than invent a new E2E harness.
- The current CSS already establishes a desktop grid with topbar, media bin, preview, inspector, and timeline regions. It is only smoke-level and English, but it gives a starting structural skeleton.

### Established Patterns
- Rust-generated contracts are imported into renderer TypeScript; generated files are not edited manually.
- Electron renderer code must not import Electron/Node APIs; the preload bridge and main process own privileged access.
- Main process rejects untrusted IPC senders and non-loopback dev server URLs. Phase 4 UI work must preserve these security boundaries.
- Root gates flow through `just build` and `just test`, with package scripts for phase-specific checks.

### Integration Points
- `apps/desktop-electron/src/renderer/App.tsx` should be decomposed into workspace components, panels, timeline, inspector, and command service helpers.
- `apps/desktop-electron/src/renderer/styles.css` or local CSS files should define the Jianying-like layout, stable dimensions, and responsive desktop minimums.
- `apps/desktop-electron/tests/electron-smoke.spec.ts` should grow or delegate to Phase 4-specific Playwright tests that verify the Chinese workspace and command-driven UI flows.
- `package.json` and `justfile` should expose Phase 4 workspace/source-guard/Electron E2E scripts.
- `crates/bindings_node` should remain the existing command boundary; Phase 4 should not add UI semantics to Rust unless the generated command contracts require a small binding route already owned by Rust.

</code_context>

<specifics>
## Specific Ideas

- The user explicitly wants the desktop UI to look enough like Jianying while allowing MVP simplification. This is a visual/product requirement, not just a layout checkbox.
- The user explicitly added that desktop language is Chinese. Treat English visible labels in the current smoke app as technical debt to replace in Phase 4.
- The user wants each step testable. Phase 4 plans should include executable gates per slice, not leave UI quality until the final plan.
- Kdenlive and MLT remain architectural references; do not copy GPL code/assets/XML/presets or use their runtime.
- pyJianYingDraft remains useful for terminology alignment, but the current project format is still `.veproj/project.json`, not Jianying's proprietary draft as canonical storage.

</specifics>

<deferred>
## Deferred Ideas

- Deterministic preview frames, playback cache, waveform generation, render graph compilation, FFmpeg script generation, and MP4 export belong to Phase 5.
- Packaged app release smoke, bundled runtime/license manifest, and offline packaged import-preview-export tests belong to Phase 6.
- Mobile UI, server renderer, Jianying/CapCut draft adapters, advanced effects, masks, stickers, transitions, filters, text bubbles, text effects, keyframes, and pixel-level proprietary effect parity remain post-MVP or later roadmap work.

</deferred>

---

*Phase: 4-Jianying-Style Desktop Workspace*
*Context gathered: 2026-06-17*
