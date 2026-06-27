# Phase 04: Jianying-Style Desktop Workspace - Research

**Researched:** 2026-06-17 [VERIFIED: local date/environment]
**Domain:** Electron desktop renderer, React/TypeScript workspace UI, Rust-generated command contracts, Playwright Electron validation [VERIFIED: .planning/phases/04-jianying-style-desktop-workspace/04-CONTEXT.md; apps/desktop-electron/package.json]
**Confidence:** HIGH [VERIFIED: local code grep; CITED: https://playwright.dev/docs/api/class-electron; CITED: https://electronjs.org/docs/latest/tutorial/context-isolation; CITED: https://react.dev/learn/updating-objects-in-state]

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
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

### Deferred Ideas (OUT OF SCOPE)
- Deterministic preview frames, playback cache, waveform generation, render graph compilation, FFmpeg script generation, and MP4 export belong to Phase 5.
- Packaged app release smoke, bundled runtime/license manifest, and offline packaged import-preview-export tests belong to Phase 6.
- Mobile UI, server renderer, Jianying/CapCut draft adapters, advanced effects, masks, stickers, transitions, filters, text bubbles, text effects, keyframes, and pixel-level proprietary effect parity remain post-MVP or later roadmap work.
</user_constraints>

## Project Constraints (from AGENTS.md)

- UI emits commands; Rust core owns project and timeline semantics; UI code must not directly construct FFmpeg commands. [VERIFIED: AGENTS.md]
- `.veproj/project.json` is the canonical semantic source of truth; render graphs, FFmpeg scripts, thumbnails, waveform data, proxy files, and preview caches are derived artifacts. [VERIFIED: AGENTS.md]
- Product language, desktop code, Rust domain types, IPC commands, docs, schema, and tests should follow Jianying concepts; prefer draft/material/track/segment/keyframe/filter/transition terms. [VERIFIED: AGENTS.md]
- Core time math must use integer microseconds, frame indices, or rational frame rates; persisted semantics must avoid naked floating-point time. [VERIFIED: AGENTS.md]
- Render Graph isolates editing semantics from FFmpeg; FFmpeg Runtime executes jobs and reports progress/errors without deciding editing behavior. [VERIFIED: AGENTS.md]
- Kdenlive and MLT are conceptual references only; do not copy GPL code, assets, XML definitions, presets, or UI implementation. [VERIFIED: AGENTS.md]
- External drafts go through adapters and compatibility reports; proprietary IDs are external references, not internal render semantics. [VERIFIED: AGENTS.md]
- Each roadmap phase must define executable gates before implementation is complete. [VERIFIED: AGENTS.md]
- FFmpeg distribution requires LGPL/GPL/nonfree build-option, notice, and commercial-obligation review before release/distribution work. [VERIFIED: AGENTS.md]
- Direct repo edits should start through GSD workflow entry points unless the user explicitly asks to bypass; this task is the GSD research artifact requested by the orchestrator/user. [VERIFIED: AGENTS.md; VERIFIED: user prompt]

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| UI-01 | Desktop editor first screen uses a Jianying-like workspace: top feature categories, left material/function panel, center preview, right inspector, and bottom multi-track timeline. [VERIFIED: .planning/REQUIREMENTS.md] | Use the approved Phase 04 UI grid: `52px` topbar, left panel, preview, inspector, `220-260px` timeline. [VERIFIED: .planning/phases/04-jianying-style-desktop-workspace/04-UI-SPEC.md] |
| UI-02 | MVP UI implements media/material, text, and audio panels while reserving visible categories for sticker, effect, transition, filter, and adjustment. [VERIFIED: .planning/REQUIREMENTS.md] | Implement `媒体`, `音频`, `文字` behavior and disabled/empty panels for `贴纸`, `特效`, `转场`, `滤镜`, `调节`. [VERIFIED: 04-UI-SPEC.md] |
| UI-03 | UI uses Jianying-style terms consistently and does not expose alternate internal jargon. [VERIFIED: .planning/REQUIREMENTS.md] | Add source guard for English key labels and forbidden user-facing `Asset`, `Clip`, `media bin`, `workbench`. [VERIFIED: 04-CONTEXT.md] |
| UI-04 | UI emits typed commands to Rust and cannot mutate the draft directly. [VERIFIED: .planning/REQUIREMENTS.md] | Use generated `CommandEnvelope`, `TimelineCommandResponse`, `Draft`, `CommandState`, and `TimelineSelection`; guard direct `Draft.tracks`/segment mutation. [VERIFIED: apps/desktop-electron/src/generated/CommandEnvelope.ts; apps/desktop-electron/src/generated/CommandResultEnvelope.ts; apps/desktop-electron/src/generated/Draft.ts] |
| UI-05 | Timeline controls have stable dimensions and do not shift layout during selection, hover, or playback updates. [VERIFIED: .planning/REQUIREMENTS.md] | Use fixed ruler, row, header, segment, and transport dimensions; verify bounding boxes at `1280x800` and `1120x720`. [VERIFIED: 04-UI-SPEC.md; CITED: https://playwright.dev/docs/test-snapshots] |
| UI-06 | Desktop UI user-facing language is Simplified Chinese by default, including panel titles, controls, empty states, errors, and test-visible copy. [VERIFIED: .planning/REQUIREMENTS.md] | Replace smoke English labels with the UI-SPEC copy table and Chinese ARIA region names. [VERIFIED: apps/desktop-electron/src/renderer/App.tsx; .planning/phases/04-jianying-style-desktop-workspace/04-UI-SPEC.md] |
| TEST-06 | Electron E2E test imports material, edits a timeline, previews, exports, and verifies output. [VERIFIED: .planning/REQUIREMENTS.md] | Phase 4 should cover the available subset: import/list material, command-only timeline edit, preview placeholder shell; preview/export verification remains Phase 5/6 because those requirements are deferred. [VERIFIED: .planning/ROADMAP.md; .planning/phases/04-jianying-style-desktop-workspace/04-CONTEXT.md] |
</phase_requirements>

## Summary

Phase 4 should replace the current smoke renderer in `apps/desktop-electron/src/renderer/App.tsx` with a compact Chinese desktop editor workspace while preserving the existing preload-only core bridge. [VERIFIED: apps/desktop-electron/src/renderer/App.tsx; VERIFIED: apps/desktop-electron/src/preload/index.ts; VERIFIED: apps/desktop-electron/src/main/index.ts] The implementation should use React + TypeScript already pinned in `apps/desktop-electron/package.json`, global CSS or locally split CSS, and no new UI package. [VERIFIED: apps/desktop-electron/package.json; VERIFIED: .planning/phases/04-jianying-style-desktop-workspace/04-UI-SPEC.md]

The central planning concern is state ownership. The renderer can own view-only UI state such as active category, form drafts, hover state, pending command status, and selected local controls, but accepted semantic state must be replaced only from Rust command responses: `Draft`, `CommandState`, and `TimelineSelection`. [VERIFIED: apps/desktop-electron/src/generated/CommandEnvelope.ts; VERIFIED: apps/desktop-electron/src/generated/CommandResultEnvelope.ts; CITED: https://react.dev/learn/updating-objects-in-state] Avoid direct mutation of `draft.tracks`, `track.segments`, timeranges, snapping, undo/redo stacks, and FFmpeg/render artifacts. [VERIFIED: .planning/phases/04-jianying-style-desktop-workspace/04-CONTEXT.md; VERIFIED: package.json]

Playwright Electron should extend the existing `_electron.launch({ args: [dist/main/index.cjs] })` harness, add window-size/layout checks for `1280x800` and `1120x720`, verify Chinese accessible regions, and prove at least one timeline edit updates UI from a `TimelineCommandResponse`. [VERIFIED: apps/desktop-electron/tests/electron-smoke.spec.ts; CITED: https://playwright.dev/docs/api/class-electron; CITED: https://playwright.dev/docs/test-snapshots]

**Primary recommendation:** Build a small typed renderer state machine plus command helper module, then implement the workspace in four slices: shell/categories, panels/inspector, command-only timeline, and Playwright/source-guard validation. [VERIFIED: .planning/ROADMAP.md; VERIFIED: apps/desktop-electron/src/generated/CommandEnvelope.ts]

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|--------------|----------------|-----------|
| Workspace layout and Chinese visible copy | Browser / Renderer | Electron shell | React renderer owns DOM layout/copy; Electron owns the desktop window and preload bridge. [VERIFIED: apps/desktop-electron/src/renderer/App.tsx; VERIFIED: apps/desktop-electron/src/main/index.ts] |
| Import material UI action | Browser / Renderer | API / Backend via Rust binding | Renderer triggers `importMaterial`/`listMaterials`; Rust binding updates draft/material state and returns generated responses. [VERIFIED: apps/desktop-electron/src/generated/CommandEnvelope.ts; VERIFIED: apps/desktop-electron/src/generated/CommandResultEnvelope.ts] |
| Draft/timeline semantics | API / Backend | Browser / Renderer display only | Phase 3 implemented add/select/move/split/trim/delete, undo/redo, snapping, text, audio, and volume semantics in Rust command crates. [VERIFIED: .planning/phases/03-timeline-command-core/03-VERIFICATION.md] |
| Timeline visualization and MVP controls | Browser / Renderer | API / Backend for accepted edits | Renderer draws rows/segments and collects user input; accepted edits must be Rust command responses. [VERIFIED: 04-CONTEXT.md; VERIFIED: apps/desktop-electron/src/generated/CommandEnvelope.ts] |
| Preview monitor shell | Browser / Renderer | Preview service in Phase 5 | Phase 4 displays a stable placeholder; deterministic preview frames and cache are explicitly deferred. [VERIFIED: 04-CONTEXT.md; VERIFIED: 04-UI-SPEC.md] |
| Visual/layout checks | Test runner | Browser / Renderer | Playwright can launch Electron, access windows, inspect locators, and compare screenshots. [CITED: https://playwright.dev/docs/api/class-electron; CITED: https://playwright.dev/docs/test-snapshots] |
| Electron privileged access | Electron main/preload | Browser / Renderer safe wrapper | Main/preload expose only `videoEditorCore`; renderer should not import Electron or Node. [VERIFIED: apps/desktop-electron/src/preload/index.ts; VERIFIED: apps/desktop-electron/src/main/index.ts; CITED: https://electronjs.org/docs/latest/tutorial/context-isolation] |

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| Electron | `42.4.1` pinned locally; npm latest `42.4.1`, modified 2026-06-16. [VERIFIED: apps/desktop-electron/package.json; VERIFIED: npm registry; slopcheck OK] | Desktop shell, main process, preload bridge, packaged renderer window. [VERIFIED: apps/desktop-electron/src/main/index.ts; CITED: https://electronjs.org/docs/latest/tutorial/context-isolation] | Existing shell already uses context isolation, sandbox, IPC allowlist, and file/loopback renderer controls. [VERIFIED: apps/desktop-electron/src/main/index.ts; apps/desktop-electron/src/preload/index.ts] |
| React | `19.2.7` pinned locally; npm latest `19.2.7`, modified 2026-06-16. [VERIFIED: apps/desktop-electron/package.json; VERIFIED: npm registry; slopcheck OK] | Renderer component model for workspace panels, inspector, and timeline. [VERIFIED: apps/desktop-electron/src/renderer/main.tsx; apps/desktop-electron/src/renderer/App.tsx] | React docs require replacing objects/arrays in state instead of mutating state directly, which fits command-response state replacement. [CITED: https://react.dev/learn/updating-objects-in-state; CITED: https://react.dev/learn/updating-arrays-in-state] |
| TypeScript | `6.0.3` pinned locally; npm latest `6.0.3`, modified 2026-06-17. [VERIFIED: apps/desktop-electron/package.json; VERIFIED: npm registry; slopcheck OK] | Type checking generated command/draft contracts and renderer helpers. [VERIFIED: apps/desktop-electron/tsconfig.json] | Strict TS is already enabled for source and tests. [VERIFIED: apps/desktop-electron/tsconfig.json] |
| Playwright Test | `1.61.0` pinned locally; npm latest `1.61.0`, modified 2026-06-17. [VERIFIED: apps/desktop-electron/package.json; VERIFIED: npm registry; slopcheck OK] | Electron E2E, source guard tests, layout assertions, screenshots. [VERIFIED: apps/desktop-electron/tests/electron-smoke.spec.ts; CITED: https://playwright.dev/docs/api/class-electron] | Existing test harness already launches the built Electron app and verifies the preload bridge. [VERIFIED: apps/desktop-electron/tests/electron-smoke.spec.ts] |
| Vite + `@vitejs/plugin-react` | Vite `8.0.16`, plugin `6.0.2` pinned locally and current on npm. [VERIFIED: apps/desktop-electron/package.json; VERIFIED: npm registry; slopcheck OK] | Build main, preload, and renderer bundles. [VERIFIED: apps/desktop-electron/vite.config.ts] | Existing Vite config separates main/preload CJS builds from renderer build and externalizes Electron/Node builtins. [VERIFIED: apps/desktop-electron/vite.config.ts] |
| Rust-generated TS contracts | Generated from Rust; `CommandEnvelope`, `CommandResultEnvelope`, and `Draft` are committed generated files. [VERIFIED: apps/desktop-electron/src/generated/CommandEnvelope.ts; apps/desktop-electron/src/generated/CommandResultEnvelope.ts; apps/desktop-electron/src/generated/Draft.ts] | Type-safe command payloads, timeline response state, draft/material/track/segment display data. [VERIFIED: generated files] | Existing root `test:contracts` checks drift with `git diff --exit-code schemas apps/desktop-electron/src/generated`. [VERIFIED: package.json] |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `@napi-rs/cli` | `3.7.2` pinned locally; npm latest `3.7.2`, modified 2026-06-14. [VERIFIED: apps/desktop-electron/package.json; VERIFIED: npm registry; slopcheck OK] | Builds the Rust Node-API binding before Electron build/test. [VERIFIED: apps/desktop-electron/package.json] | Use existing `pnpm --filter @video-editor/desktop build:native`; do not bypass with renderer-side native imports. [VERIFIED: apps/desktop-electron/package.json; apps/desktop-electron/src/main/nativeBinding.ts] |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Existing CSS + React components | shadcn/ui, Tailwind, icon libraries | Not approved for Phase 04; UI-SPEC says no component/icon library is present and no third-party UI blocks are approved. [VERIFIED: 04-UI-SPEC.md] |
| Command-only MVP buttons/inputs for timeline edits | Full pointer drag trim/move implementation | Full drag can be deferred if deterministic controls prove command flow first; UI-SPEC requires the surface to be shaped for richer drag later. [VERIFIED: 04-CONTEXT.md; 04-UI-SPEC.md] |
| Real preview playback | Placeholder monitor shell | Phase 5 owns deterministic preview frames/cache; Phase 4 should not invent a preview path. [VERIFIED: .planning/ROADMAP.md; 04-CONTEXT.md] |

**Installation:** No new external packages are recommended for Phase 04. [VERIFIED: apps/desktop-electron/package.json; 04-UI-SPEC.md]

```bash
# No install command needed. Use existing workspace dependencies.
pnpm install --frozen-lockfile
```

**Version verification:** Existing package versions were checked with `npm view <package> version time.modified repository.url license scripts.postinstall` and the npm downloads API on 2026-06-17. [VERIFIED: npm registry] `npm view` reported no `scripts.postinstall` fields for the listed direct packages in this session output. [VERIFIED: npm registry]

## Package Legitimacy Audit

> Phase 04 should not add packages; this audit covers existing packages the plan will rely on. [VERIFIED: apps/desktop-electron/package.json]

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| `electron` | npm | Created 2022-01-26; modified 2026-06-16. [VERIFIED: npm registry] | 4,716,757/week for 2026-06-09..2026-06-15. [VERIFIED: npm downloads API] | `github.com/electron/electron` [VERIFIED: npm registry] | OK [VERIFIED: slopcheck output] | Approved existing dependency |
| `react` | npm | Created 2022-01-26; modified 2026-06-16. [VERIFIED: npm registry] | 143,595,274/week for 2026-06-09..2026-06-15. [VERIFIED: npm downloads API] | `github.com/facebook/react` [VERIFIED: npm registry] | OK [VERIFIED: slopcheck output] | Approved existing dependency |
| `react-dom` | npm | Created 2022-01-26; modified 2026-06-16. [VERIFIED: npm registry] | 134,434,843/week for 2026-06-09..2026-06-15. [VERIFIED: npm downloads API] | `github.com/facebook/react` [VERIFIED: npm registry] | OK [VERIFIED: slopcheck output] | Approved existing dependency |
| `@playwright/test` | npm | Created 2022-01-27; modified 2026-06-17. [VERIFIED: npm registry] | 41,476,311/week for 2026-06-09..2026-06-15. [VERIFIED: npm downloads API] | `github.com/microsoft/playwright` [VERIFIED: npm registry] | OK [VERIFIED: slopcheck output] | Approved existing dependency |
| `@vitejs/plugin-react` | npm | Created 2022-01-27; modified 2026-05-14. [VERIFIED: npm registry] | 64,038,290/week for 2026-06-09..2026-06-15. [VERIFIED: npm downloads API] | `github.com/vitejs/vite-plugin-react` [VERIFIED: npm registry] | OK [VERIFIED: slopcheck output] | Approved existing dependency |
| `vite` | npm | Created 2022-01-28; modified 2026-06-15. [VERIFIED: npm registry] | 139,974,183/week for 2026-06-09..2026-06-15. [VERIFIED: npm downloads API] | `github.com/vitejs/vite` [VERIFIED: npm registry] | OK [VERIFIED: slopcheck output] | Approved existing dependency |
| `typescript` | npm | Created 2022-01-26; modified 2026-06-17. [VERIFIED: npm registry] | 218,034,542/week for 2026-06-09..2026-06-15. [VERIFIED: npm downloads API] | `github.com/microsoft/TypeScript` [VERIFIED: npm registry] | OK [VERIFIED: slopcheck output] | Approved existing dependency |
| `@napi-rs/cli` | npm | Created 2022-01-26; modified 2026-06-14. [VERIFIED: npm registry] | 1,149,779/week for 2026-06-09..2026-06-15. [VERIFIED: npm downloads API] | `github.com/napi-rs/napi-rs` [VERIFIED: npm registry] | OK [VERIFIED: slopcheck output] | Approved existing dependency |

**Packages removed due to slopcheck [SLOP] verdict:** none. [VERIFIED: slopcheck output]
**Packages flagged as suspicious [SUS]:** none. [VERIFIED: slopcheck output]

Note: `slopcheck install ... --json` is unsupported by installed `slopcheck 0.6.1`; normal output was used. The command performed an npm install side effect, which was cleaned from `package.json` and `package-lock.json`; no research recommendation depends on that install. [VERIFIED: terminal output; VERIFIED: git status]

## Architecture Patterns

### System Architecture Diagram

```text
User action in Chinese workspace
  -> React event handler / form commit
  -> command helper builds generated CommandEnvelope
  -> window.videoEditorCore.executeCommand(command)
  -> preload safe wrapper
  -> Electron main IPC allowlist
  -> native binding executeCommand
  -> Rust command core
  -> CommandResultEnvelope<TimelineCommandResponse>
  -> reducer replaces draft/commandState/selection from response
  -> renderer redraws panels, inspector, and timeline

Rejected command
  -> CommandResultEnvelope.ok=false
  -> keep prior accepted draft/commandState/selection
  -> show Chinese operation error in relevant region
```

This flow matches the existing preload bridge and generated command envelopes. [VERIFIED: apps/desktop-electron/src/preload/index.ts; apps/desktop-electron/src/main/index.ts; apps/desktop-electron/src/generated/CommandEnvelope.ts; apps/desktop-electron/src/generated/CommandResultEnvelope.ts]

### Recommended Project Structure

```text
apps/desktop-electron/src/renderer/
├── App.tsx                         # top-level workspace state machine and shell wiring [VERIFIED: current file exists]
├── styles.css                      # Phase 04 global layout tokens and region styles [VERIFIED: current file exists]
├── commandHelpers.ts               # generated CommandEnvelope builders and response application [RECOMMENDED: research]
├── workspace/
│   ├── WorkspaceShell.tsx           # top categories + region layout [RECOMMENDED: research]
│   ├── FeaturePanel.tsx             # material/text/audio/deferred panels [RECOMMENDED: research]
│   ├── PreviewMonitor.tsx           # Phase 04 placeholder monitor [RECOMMENDED: research]
│   ├── Inspector.tsx                # selection-aware command fields [RECOMMENDED: research]
│   └── Timeline.tsx                 # fixed-row timeline visualization and MVP controls [RECOMMENDED: research]
└── viewModel.ts                     # pure derived selectors/formatters; no mutation or commands [RECOMMENDED: research]
```

Keep generated files under `apps/desktop-electron/src/generated/` untouched; drift is checked by `test:contracts`. [VERIFIED: generated file headers; VERIFIED: package.json]

### Pattern 1: Renderer State Shape

**What:** Store one accepted semantic snapshot and separate ephemeral UI state. [VERIFIED: generated contracts; CITED: https://react.dev/learn/updating-objects-in-state]

**When to use:** Use for all workspace actions because Rust commands return `draft`, `commandState`, and `selection`. [VERIFIED: apps/desktop-electron/src/generated/CommandResultEnvelope.ts]

**Example:**

```typescript
// Source: generated contracts and React state docs.
import type { CommandState, TimelineSelection } from "../generated/CommandEnvelope";
import type { Draft } from "../generated/Draft";

type WorkspaceState = {
  draft: Draft;
  commandState: CommandState;
  selection: TimelineSelection;
  activeCategory: "媒体" | "音频" | "文字" | "贴纸" | "特效" | "转场" | "滤镜" | "调节";
  pendingCommand: string | null;
  lastError: string | null;
};
```

The renderer may derive display rows from `draft.tracks`, but it must not mutate the returned draft or arrays. [VERIFIED: 04-CONTEXT.md; CITED: https://react.dev/learn/updating-arrays-in-state]

### Pattern 2: Typed Command Helper

**What:** Centralize `CommandEnvelope` construction and result handling to keep every command consistent. [VERIFIED: apps/desktop-electron/src/generated/CommandEnvelope.ts]

**When to use:** Use for material, timeline, text, audio, volume, mute, undo, and redo actions. [VERIFIED: apps/desktop-electron/src/generated/CommandEnvelope.ts]

**Example:**

```typescript
// Source: apps/desktop-electron/src/generated/CommandEnvelope.ts and CommandResultEnvelope.ts.
import type { CommandEnvelope, CommandState, TimelineSelection } from "../generated/CommandEnvelope";
import type { CommandResultEnvelope, TimelineCommandResponse } from "../generated/CommandResultEnvelope";
import type { Draft, SegmentId, TrackId } from "../generated/Draft";

type CommandContext = {
  draft: Draft;
  commandState: CommandState;
  selection: TimelineSelection;
};

export function selectTimelineSegmentsCommand(
  context: CommandContext,
  segmentIds: SegmentId[],
  trackIds: TrackId[]
): CommandEnvelope {
  return {
    command: "selectTimelineSegments",
    payload: {
      kind: "selectTimelineSegments",
      draft: context.draft,
      commandState: context.commandState,
      selection: context.selection,
      segmentIds,
      trackIds
    },
    requestId: `select-${Date.now()}`
  };
}

export function applyTimelineResult(
  result: CommandResultEnvelope<TimelineCommandResponse>,
  previous: CommandContext
): CommandContext {
  if (!result.ok || result.data === null) {
    return previous;
  }
  return {
    draft: result.data.draft,
    commandState: result.data.commandState,
    selection: result.data.selection
  };
}
```

Do not generate IDs with timing if deterministic tests assert exact payloads; use stable fixture IDs or a small deterministic helper in tests. [VERIFIED: .planning/phases/03-timeline-command-core/03-CONTEXT.md]

### Pattern 3: View Model Selectors Are Read-Only

**What:** Put `findSelectedSegment`, `materialLabel`, `formatMicroseconds`, and track/segment display calculations in pure functions that return new view objects. [RECOMMENDED: research; CITED: https://react.dev/learn/updating-arrays-in-state]

**When to use:** Use for panel/timeline rendering and inspector binding. [VERIFIED: 04-UI-SPEC.md]

**Example:**

```typescript
// Source: generated Draft.ts semantics.
import type { Draft, Segment, SegmentId, Track } from "../generated/Draft";

export function findSegment(draft: Draft, segmentId: SegmentId): { track: Track; segment: Segment } | null {
  for (const track of draft.tracks) {
    const segment = track.segments.find((candidate) => candidate.segmentId === segmentId);
    if (segment !== undefined) {
      return { track, segment };
    }
  }
  return null;
}
```

Reading `draft.tracks` is allowed for display; assigning to it, pushing/splicing/sorting segment arrays, or editing timeranges is not allowed. [VERIFIED: 04-CONTEXT.md; VERIFIED: package.json]

### Pattern 4: Playwright Layout Geometry Checks

**What:** Use role/label locators for Chinese regions and bounding boxes to prove non-overlap at two Electron window sizes. [CITED: https://playwright.dev/docs/api/class-electron; CITED: https://playwright.dev/docs/test-snapshots]

**When to use:** Use in Phase 04 visual/layout tests after launching the built Electron app. [VERIFIED: apps/desktop-electron/tests/electron-smoke.spec.ts]

**Example:**

```typescript
// Source: Playwright Electron and screenshot docs.
await app.evaluate(async ({ BrowserWindow }) => {
  const [window] = BrowserWindow.getAllWindows();
  window.setSize(1280, 800);
});
await expect(page.getByRole("main", { name: "剪映式桌面工作区" })).toBeVisible();
await expect(page.getByRole("region", { name: "素材面板" })).toBeVisible();
await expect(page).toHaveScreenshot("workspace-1280x800.png", {
  animations: "disabled",
  maxDiffPixels: 250
});
```

Use bounding-box assertions for overlap invariants even if screenshot snapshots are noisy on different host fonts. [RECOMMENDED: research; CITED: https://playwright.dev/docs/api/class-pageassertions]

### Anti-Patterns to Avoid

- **Renderer mutates semantic draft:** Direct edits such as `draft.tracks =`, `track.segments.push`, `segment.targetTimerange.start =`, or local snapping repair break Rust ownership. [VERIFIED: 04-CONTEXT.md; VERIFIED: package.json]
- **Renderer imports privileged APIs:** `electron`, `node:*`, `fs`, `path`, native binding, or child process imports in renderer bypass the existing preload boundary. [VERIFIED: apps/desktop-electron/src/main/index.ts; CITED: https://electronjs.org/docs/latest/tutorial/security]
- **UI invents preview/export state:** Preview frames, waveform paths, render graphs, FFmpeg scripts, export jobs, and cache invalidation are Phase 5/6 concerns. [VERIFIED: .planning/ROADMAP.md; 04-CONTEXT.md]
- **English smoke copy leaks:** Existing labels such as `Video Editor`, `Media`, `Materials`, `Preview monitor`, and `Inspector` must be replaced or hidden from user/test-visible UI. [VERIFIED: apps/desktop-electron/src/renderer/App.tsx; VERIFIED: 04-UI-SPEC.md]
- **Generic dashboard layout:** Cards/hero/dashboard composition violates the approved UI-SPEC. [VERIFIED: 04-UI-SPEC.md]

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Timeline edit acceptance | Local reducer that edits `Draft.tracks` | Generated `CommandEnvelope` + Rust `TimelineCommandResponse` | Rust already owns atomic validation, snapping, main-track magnet, undo/redo, text, audio, and volume. [VERIFIED: 03-VERIFICATION.md] |
| Undo/redo | UI inverse operation stack | `undoTimelineEdit` / `redoTimelineEdit` commands | Command history is returned session state and is not persisted in `.veproj`. [VERIFIED: 03-VERIFICATION.md; apps/desktop-electron/src/generated/CommandEnvelope.ts] |
| IPC bridge | Direct `ipcRenderer` or native binding exposure | Existing `window.videoEditorCore` preload API | Electron docs recommend contextBridge APIs under context isolation; current code exposes only `ping`, `version`, and `executeCommand`. [CITED: https://electronjs.org/docs/latest/tutorial/context-isolation; VERIFIED: apps/desktop-electron/src/preload/index.ts] |
| Visual regression harness | Custom image diff tooling | Playwright `toHaveScreenshot` plus bounding-box assertions | Playwright Test has built-in screenshot comparisons and Electron app control. [CITED: https://playwright.dev/docs/test-snapshots; CITED: https://playwright.dev/docs/api/class-electron] |
| Time conversion semantics | Float seconds in state/payloads | Integer microseconds display formatter only | Generated `Microseconds` is numeric, and project constraints forbid naked persisted floating-point time. [VERIFIED: apps/desktop-electron/src/generated/Draft.ts; AGENTS.md] |
| FFmpeg probing/rendering from UI | UI command strings or `child_process` | Existing Rust media/runtime and later render pipeline | Renderer FFmpeg construction is already forbidden and source-guarded. [VERIFIED: AGENTS.md; package.json] |

**Key insight:** Phase 4 is a shell and command-integration phase, not a new semantics phase; every accepted edit should be observable as a generated Rust command response. [VERIFIED: .planning/ROADMAP.md; .planning/phases/04-jianying-style-desktop-workspace/04-CONTEXT.md]

## Common Pitfalls

### Pitfall 1: Mutating Returned Draft State

**What goes wrong:** UI changes appear locally but diverge from Rust command semantics and undo/redo history. [VERIFIED: 04-CONTEXT.md; 03-VERIFICATION.md]
**Why it happens:** React makes it easy to read nested arrays, and existing `Draft.tracks` is exposed to the renderer for display. [VERIFIED: apps/desktop-electron/src/generated/Draft.ts]
**How to avoid:** Treat `Draft`, `CommandState`, and `TimelineSelection` as immutable snapshots and replace them only from successful command results. [CITED: https://react.dev/learn/updating-objects-in-state; VERIFIED: apps/desktop-electron/src/generated/CommandResultEnvelope.ts]
**Warning signs:** `tracks.push`, `segments.splice`, `targetTimerange.start =`, local undo stack, or snapping math in renderer. [VERIFIED: package.json; 04-CONTEXT.md]

### Pitfall 2: Timeline Layout Shift Under Interaction

**What goes wrong:** Selection outlines, hover states, transport state, or error messages resize rows and break the editor feel. [VERIFIED: 04-UI-SPEC.md]
**Why it happens:** Content-driven row heights and border changes alter geometry. [ASSUMED]
**How to avoid:** Fixed timeline band, ruler, track header, row, segment min width, outline via `box-shadow`/absolute overlay, and scroll inside lanes only. [VERIFIED: 04-UI-SPEC.md]
**Warning signs:** `auto` row heights in timeline, hover borders that change box size, category labels wrapping, or panel empty states changing region width. [VERIFIED: 04-UI-SPEC.md]

### Pitfall 3: English Labels Survive in Accessibility/Test Copy

**What goes wrong:** Visual Chinese labels pass a glance check while ARIA labels/tests still expose `Material bin`, `Preview monitor`, or `Inspector`. [VERIFIED: apps/desktop-electron/src/renderer/App.tsx]
**Why it happens:** Existing smoke UI uses English accessible labels and test locators. [VERIFIED: apps/desktop-electron/src/renderer/App.tsx; apps/desktop-electron/tests/electron-smoke.spec.ts]
**How to avoid:** Replace visible and user-facing ARIA labels with UI-SPEC Chinese copy and make Playwright locate `顶部功能区`, `素材面板`, `预览窗口`, `属性检查器`, and `时间线`. [VERIFIED: 04-UI-SPEC.md]
**Warning signs:** `getByRole(..., { name: /Video|Media|Material|Preview|Inspector|Timeline/ })` in tests. [VERIFIED: apps/desktop-electron/tests/electron-smoke.spec.ts]

### Pitfall 4: Test Tries to Prove Phase 5/6 Behavior

**What goes wrong:** Phase 4 plans block on real preview/export implementation. [VERIFIED: .planning/ROADMAP.md]
**Why it happens:** `TEST-06` is broader than Phase 4's technical boundary. [VERIFIED: .planning/REQUIREMENTS.md; .planning/ROADMAP.md]
**How to avoid:** In Phase 4 validation, prove workspace, command-only timeline edit, and preview placeholder; explicitly mark real preview/export as deferred to Phase 5/6 in validation. [VERIFIED: 04-CONTEXT.md]
**Warning signs:** FFmpeg, render graph, preview cache, waveform, or export command code appears in renderer plans. [VERIFIED: 04-CONTEXT.md; AGENTS.md]

### Pitfall 5: Generated Contract Drift

**What goes wrong:** Renderer helpers compile against stale generated TS or manually edited generated files. [VERIFIED: generated file headers; package.json]
**Why it happens:** Generated files are committed and easy to edit accidentally. [VERIFIED: generated file headers]
**How to avoid:** Keep `git diff --exit-code schemas apps/desktop-electron/src/generated` in Phase 4 gates and add source guard that fails on edits to generated files not produced by Rust generation. [VERIFIED: package.json]
**Warning signs:** Manual changes under `apps/desktop-electron/src/generated/`. [VERIFIED: generated file headers]

## Material/Text/Audio Panel And Inspector Scope

| Area | Implement in Phase 4 | Defer |
|------|----------------------|-------|
| `媒体` panel | Import/list material actions, Chinese rows, type/status labels, duration/resolution/audio metadata display, missing/probe-failed states. [VERIFIED: 04-CONTEXT.md; 04-UI-SPEC.md; generated contracts] | Thumbnails, waveform previews, proxy files, preview caches. [VERIFIED: AGENTS.md; 04-CONTEXT.md] |
| `音频` panel | Add audio/BGM material to audio track, set segment volume, track mute toggle. [VERIFIED: apps/desktop-electron/src/generated/CommandEnvelope.ts; 03-VERIFICATION.md] | Waveform drawing, audio preview mix, export behavior. [VERIFIED: 04-CONTEXT.md] |
| `文字` panel | Add/edit text content and MVP style fields: font size, color, alignment, stroke, shadow, background. [VERIFIED: apps/desktop-electron/src/generated/Draft.ts; apps/desktop-electron/src/generated/CommandEnvelope.ts] | Deterministic text layout, pinned fonts, text bubbles, text effects. [VERIFIED: 04-CONTEXT.md; .planning/ROADMAP.md] |
| Deferred categories | Visible category buttons and Chinese empty states for `贴纸`, `特效`, `转场`, `滤镜`, `调节`. [VERIFIED: 04-UI-SPEC.md] | Functional sticker/effect/transition/filter/adjustment editing. [VERIFIED: 04-CONTEXT.md] |
| Inspector | No-selection Chinese empty state; selected segment properties for text, volume, track mute, source/target display; commands on explicit commit/debounce. [VERIFIED: 04-UI-SPEC.md] | Render-only transform/effect controls not represented in Phase 3 semantics. [VERIFIED: 03-CONTEXT.md] |

## Timeline MVP Interaction Approach

Use deterministic command buttons and simple numeric commits before full drag editing. [VERIFIED: 04-CONTEXT.md] The MVP should include: select segment, add selected material to first compatible track, move by fixed microsecond step or numeric target start, split at playhead/numeric time, trim left/right by fixed/numeric target range, delete selected segment, undo, redo, set segment volume, and toggle track mute. [VERIFIED: apps/desktop-electron/src/generated/CommandEnvelope.ts; 03-VERIFICATION.md]

Implementation details:

- Render track headers at fixed width and rows at UI-SPEC heights. [VERIFIED: 04-UI-SPEC.md]
- Derive segment block `left` and `width` from integer `targetTimerange` and a display scale, but do not write those values back to the draft. [VERIFIED: apps/desktop-electron/src/generated/Draft.ts; 04-CONTEXT.md]
- Keep playhead as renderer-only display state in Phase 4; do not treat it as preview/render truth. [VERIFIED: 04-CONTEXT.md]
- Use Rust command events for status display, especially snapping/magnet outcomes, instead of recomputing snapping in UI. [VERIFIED: 03-VERIFICATION.md]
- On command rejection, keep the last accepted snapshot and show `操作失败：{message}。请检查素材或撤销上一步后重试。` [VERIFIED: 04-UI-SPEC.md]

## Source Guards

Recommended Phase 4 source guard script should extend existing guards with these checks:

| Guard | Suggested Scan |
|-------|----------------|
| English key labels | Fail renderer/tests on user-facing `Video editor smoke workbench|Feature categories|Material bin|Preview monitor|Inspector|Timeline|Materials|Media|Text|Audio|Effects|Asset|Clip|workbench`, except generated files and source-guard allowlist. [VERIFIED: App.tsx current labels; 04-UI-SPEC.md] |
| Direct draft mutation | Fail renderer on `\\.tracks\\s*(=|\\[)`, `tracks\\.(push|splice|sort|reverse)`, `\\.segments\\s*(=|\\[)`, `segments\\.(push|splice|sort|reverse)`, `targetTimerange\\.[a-zA-Z]+\\s*=`, `sourceTimerange\\.[a-zA-Z]+\\s*=`, `mainTrackMagnet\\.[a-zA-Z]+\\s*=`. [VERIFIED: package.json existing Phase 3 guard pattern] |
| Electron/Node imports in renderer | Fail `apps/desktop-electron/src/renderer` on `from "electron"`, `from "node:`, `from "fs"`, `from "path"`, `child_process`, `createRequire`, `process\\.`. [VERIFIED: Electron boundary files; CITED: https://electronjs.org/docs/latest/tutorial/security] |
| FFmpeg leakage | Fail renderer on `ffmpeg|ffprobe|filter_complex|renderGraph|ffmpegScripts|previewCache|waveform`. [VERIFIED: AGENTS.md; package.json] |
| Generated drift | Keep `git diff --exit-code schemas apps/desktop-electron/src/generated`. [VERIFIED: package.json] |
| Generated manual edits | Fail if generated files lose header `This file was generated by Rust ts-rs declarations. Do not edit this file manually.` [VERIFIED: generated files] |

## Code Examples

Verified patterns from official/local sources:

### Execute Timeline Command And Preserve Previous State On Error

```typescript
// Source: generated CommandResultEnvelope.ts and Phase 04 UI-SPEC error contract.
async function runTimelineCommand(command: CommandEnvelope): Promise<void> {
  setWorkspace((state) => ({ ...state, pendingCommand: command.command, lastError: null }));
  const result = await window.videoEditorCore.executeCommand<TimelineCommandResponse>(command);
  setWorkspace((state) => {
    if (!result.ok || result.data === null) {
      return {
        ...state,
        pendingCommand: null,
        lastError: `操作失败：${result.error?.message ?? "未知错误"}。请检查素材或撤销上一步后重试。`
      };
    }
    return {
      ...state,
      draft: result.data.draft,
      commandState: result.data.commandState,
      selection: result.data.selection,
      pendingCommand: null,
      lastError: null
    };
  });
}
```

This pattern replaces semantic state only from `TimelineCommandResponse`. [VERIFIED: apps/desktop-electron/src/generated/CommandResultEnvelope.ts]

### Chinese Material Status Formatter

```typescript
// Source: generated Draft.ts material status union and UI-SPEC copy.
import type { Material, MaterialKind, MaterialStatus } from "../generated/Draft";

const materialKindLabels: Record<MaterialKind, string> = {
  video: "视频",
  image: "图片",
  audio: "音频",
  text: "文字",
  sticker: "贴纸"
};

const materialStatusLabels: Record<MaterialStatus, string> = {
  available: "可用",
  missing: "素材丢失",
  probeFailed: "解析失败"
};

export function materialRowLabel(material: Material): string {
  return `${materialKindLabels[material.kind]} ${material.displayName} ${materialStatusLabels[material.status]}`;
}
```

Use Chinese status labels required by UI-SPEC. [VERIFIED: apps/desktop-electron/src/generated/Draft.ts; .planning/phases/04-jianying-style-desktop-workspace/04-UI-SPEC.md]

### Bounding-Box Layout Assertion Helper

```typescript
// Source: Playwright Locator boundingBox API via Locator/Page docs; used with Electron-launched page.
async function expectNoOverlap(a: Locator, b: Locator): Promise<void> {
  const first = await a.boundingBox();
  const second = await b.boundingBox();
  expect(first).not.toBeNull();
  expect(second).not.toBeNull();
  if (first === null || second === null) return;
  const separated =
    first.x + first.width <= second.x ||
    second.x + second.width <= first.x ||
    first.y + first.height <= second.y ||
    second.y + second.height <= first.y;
  expect(separated).toBe(true);
}
```

Use alongside screenshot assertions because screenshots alone may not identify which region overlapped. [RECOMMENDED: research; CITED: https://playwright.dev/docs/test-snapshots]

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Exposing broad Electron APIs to renderer | Context isolation with a narrow `contextBridge` wrapper | Electron 12 made context isolation default; Electron docs currently recommend it. [CITED: https://electronjs.org/docs/latest/tutorial/context-isolation] | Preserve `window.videoEditorCore` as the only renderer bridge. [VERIFIED: apps/desktop-electron/src/preload/index.ts] |
| Passing `ipcRenderer` over the bridge | Safe wrapper methods only | Electron 29 disallows sending entire `ipcRenderer` over `contextBridge`. [CITED: https://electronjs.org/docs/latest/breaking-changes] | Do not expose raw IPC or Electron objects to renderer. [CITED: https://electronjs.org/docs/latest/api/ipc-renderer] |
| Hand-captured screenshots only | Playwright `toHaveScreenshot` and locator assertions | Playwright docs include screenshot comparison in the test runner. [CITED: https://playwright.dev/docs/test-snapshots] | Use snapshots plus geometry checks for Phase 4 layout. [RECOMMENDED: research] |
| Mutating nested React state | Replace objects/arrays with new state values | React current docs instruct treating object/array state as read-only. [CITED: https://react.dev/learn/updating-objects-in-state; CITED: https://react.dev/learn/updating-arrays-in-state] | Fits command-response replacement of `Draft`, `CommandState`, and `TimelineSelection`. [VERIFIED: generated contracts] |

**Deprecated/outdated:**
- English smoke UI labels in `App.tsx` are obsolete for Phase 4 because UI-SPEC requires Simplified Chinese default copy. [VERIFIED: apps/desktop-electron/src/renderer/App.tsx; 04-UI-SPEC.md]
- Direct smoke-only material list is obsolete as the first screen because Phase 4 requires full editor workspace regions. [VERIFIED: .planning/ROADMAP.md; 04-CONTEXT.md]

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Content-driven CSS row heights and border changes are common causes of layout shift. [ASSUMED] | Common Pitfalls | Low; UI-SPEC fixed dimensions and Playwright geometry checks mitigate the risk. |
| A2 | Splitting renderer files into `workspace/`, `commandHelpers.ts`, and `viewModel.ts` will reduce planning and implementation complexity. [RECOMMENDED: research] | Architecture Patterns | Low; planner may choose a different split within discretion if guards and command boundaries hold. |

## Open Questions (RESOLVED)

Resolution source: Phase 4 locked decisions D-01, D-05, D-07, D-11, D-14, and D-16 through D-19. The first screen is the real Simplified Chinese desktop editor workspace, deterministic command controls are acceptable before full pointer drag editing, and preview/export execution remains assigned to Phase 5/6.

1. **RESOLVED: Phase 4 test setup may use deterministic in-test draft fixtures, but accepted user-visible timeline edits must be proven through Rust command responses.** [VERIFIED: 04-CONTEXT.md]
   - What we know: Phase 4 may use mock local draft fixtures or real commands, but visible accepted timeline edits must be proven through Rust command responses. [VERIFIED: 04-CONTEXT.md]
   - Decision: Use deterministic in-test draft fixtures for timeline command coverage when fixture setup is needed, keep `importMaterial`/`listMaterials` coverage where local fixture paths are reliable, and require any visible accepted timeline edit to call `window.videoEditorCore.executeCommand` and apply `TimelineCommandResponse` per D-12, D-13, D-14, and D-16. [VERIFIED: 04-CONTEXT.md]
   - Boundary: Do not implement or fake deterministic preview frames, render graph execution, FFmpeg export, waveform generation, or packaged import-preview-export flow in Phase 4; those remain Phase 5/6 work per D-11 and the roadmap. [VERIFIED: 04-CONTEXT.md; VERIFIED: .planning/ROADMAP.md]

2. **RESOLVED: Hard visual gates are semantic visibility and geometry stability; screenshots are optional supporting evidence.** [CITED: https://playwright.dev/docs/test-snapshots]
   - What we know: Playwright screenshot assertions compare against reference images and support options like max-diff thresholds. [CITED: https://playwright.dev/docs/test-snapshots]
   - Decision: Use Playwright role/text assertions for the real Simplified Chinese editor workspace per D-01, D-05, and D-07, and use bounding-box assertions at `1280x800` and `1120x720` as the hard layout gate per D-17. Screenshots may be added with modest tolerance as visual regression evidence, but they are not the only required proof of layout correctness. [VERIFIED: 04-CONTEXT.md; VERIFIED: 04-UI-SPEC.md]

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| Node.js | pnpm/Vite/Electron build/test | Yes [VERIFIED: `node --version`] | `v24.12.0` [VERIFIED: environment command] | None needed |
| pnpm | package install/scripts | Yes [VERIFIED: `pnpm --version`] | `10.32.1` [VERIFIED: environment command] | None needed |
| Rust/Cargo | native binding and Rust tests | Yes [VERIFIED: `cargo --version`; `rustc --version`] | Cargo `1.95.0`; rustc `1.95.0` [VERIFIED: environment command] | None needed |
| just | public root gates | No [VERIFIED: environment command] | — | Run equivalent `pnpm run build` / `pnpm run test` scripts or install `just`; planner should note local missing CLI. [VERIFIED: justfile; package.json] |
| ctx7 | documentation lookup preference | No [VERIFIED: environment command] | — | Official docs and local source were used. [CITED: docs URLs in Sources] |
| slopcheck | package legitimacy gate | Yes, but no `--json` support [VERIFIED: terminal output] | `0.6.1` [VERIFIED: terminal output] | Use normal slopcheck output and npm registry checks. [VERIFIED: terminal output] |
| Playwright Electron | Phase 4 E2E/layout | Dependency pinned [VERIFIED: apps/desktop-electron/package.json] | `@playwright/test 1.61.0` [VERIFIED: npm registry] | None needed |

**Missing dependencies with no fallback:**
- None for research; `just` is missing locally but equivalent pnpm scripts exist. [VERIFIED: justfile; package.json]

**Missing dependencies with fallback:**
- `just`: use `pnpm run build` and `pnpm run test` directly, or install `just` before running public recipes. [VERIFIED: justfile; package.json; environment command]
- `ctx7`: official docs were used directly. [VERIFIED: environment command; CITED: docs URLs]

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Playwright Test `1.61.0` for Electron UI; Rust/cargo tests remain root gate dependencies. [VERIFIED: apps/desktop-electron/package.json; package.json] |
| Config file | `apps/desktop-electron/playwright.config.ts` with `testDir: "./tests"`, `timeout: 30000`, and `trace: "retain-on-failure"`. [VERIFIED: apps/desktop-electron/playwright.config.ts] |
| Quick run command | `pnpm --filter @video-editor/desktop playwright test tests/electron-smoke.spec.ts` after desktop build, or a new `pnpm run test:phase4-workspace` script. [RECOMMENDED: research; VERIFIED: apps/desktop-electron/package.json] |
| Full suite command | `pnpm run test` or `just test` where `just` is installed. [VERIFIED: package.json; justfile; environment command] |

### Phase Requirements -> Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| UI-01 | Workspace opens directly to Chinese top/left/preview/right/timeline regions. [VERIFIED: REQUIREMENTS.md] | Electron E2E + layout | `pnpm --filter @video-editor/desktop playwright test tests/electron-smoke.spec.ts -g "Chinese editor workspace"` [RECOMMENDED: research] | Existing test file yes, new test cases needed. [VERIFIED: apps/desktop-electron/tests/electron-smoke.spec.ts] |
| UI-02 | Media/text/audio panels work; deferred categories visible with Chinese empty states. [VERIFIED: REQUIREMENTS.md] | Electron E2E | Same Phase 4 workspace test command. [RECOMMENDED: research] | Wave 0 additions needed. [VERIFIED: 04-UI-SPEC.md] |
| UI-03 | No forbidden English/internal jargon in user-visible key labels. [VERIFIED: REQUIREMENTS.md] | Source guard + E2E text assertions | `pnpm run test:phase4-source-guards` [RECOMMENDED: research] | Script missing; add in Wave 0/Plan 04-04. [VERIFIED: package.json] |
| UI-04 | Timeline/text/audio edits call generated commands and update from `TimelineCommandResponse`. [VERIFIED: REQUIREMENTS.md] | Electron E2E with page-evaluated bridge spy or UI result assertions | `pnpm --filter @video-editor/desktop playwright test tests/electron-smoke.spec.ts -g "command-only timeline"` [RECOMMENDED: research] | Existing harness yes; new assertions needed. [VERIFIED: apps/desktop-electron/tests/electron-smoke.spec.ts] |
| UI-05 | Timeline/panels keep stable dimensions during selection/hover/playback placeholder updates. [VERIFIED: REQUIREMENTS.md] | Layout geometry + screenshot | `pnpm --filter @video-editor/desktop playwright test tests/electron-smoke.spec.ts -g "layout stability"` [RECOMMENDED: research] | Missing; add tests. [VERIFIED: 04-UI-SPEC.md] |
| UI-06 | Visible and test-visible copy is Simplified Chinese by default. [VERIFIED: REQUIREMENTS.md] | E2E text/ARIA assertions + source guard | `pnpm run test:phase4-source-guards && pnpm --filter @video-editor/desktop playwright test` [RECOMMENDED: research] | Missing; add tests/guard. [VERIFIED: App.tsx current English copy] |
| TEST-06 | Phase 4 subset of import/edit/preview shell proven; preview/export deferred. [VERIFIED: REQUIREMENTS.md; ROADMAP.md] | E2E smoke + documented validation limitation | `pnpm run test:desktop` plus new Phase 4 smoke. [RECOMMENDED: research] | Existing smoke file yes; broader import/export impossible until later phases. [VERIFIED: ROADMAP.md] |

### Sampling Rate

- **Per task commit:** Run TypeScript build for desktop or `pnpm --filter @video-editor/desktop build:electron` plus relevant focused Playwright test when UI changes. [RECOMMENDED: research; VERIFIED: apps/desktop-electron/package.json]
- **Per wave merge:** Run `pnpm --filter @video-editor/desktop test` after native build is available. [VERIFIED: apps/desktop-electron/package.json]
- **Phase gate:** Run `pnpm run build`, `pnpm run test`, `pnpm run test:phase4-source-guards`, Phase 4 Playwright layout tests, and generated drift check. [RECOMMENDED: research; VERIFIED: package.json]

### Wave 0 Gaps

- [ ] `apps/desktop-electron/tests/electron-smoke.spec.ts` or `workspace.spec.ts` needs Chinese workspace, command-only timeline, and layout stability cases. [VERIFIED: existing smoke file]
- [ ] `package.json` needs `test:phase4-source-guards` and should include it in root `test`. [VERIFIED: package.json]
- [ ] `apps/desktop-electron/package.json` can add a focused `test:workspace` script if planner wants faster feedback. [RECOMMENDED: research]
- [ ] Screenshot snapshots for `1280x800` and `1120x720` need to be generated and committed if visual comparison is used as a hard gate. [CITED: https://playwright.dev/docs/test-snapshots]

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | No [VERIFIED: REQUIREMENTS.md] | No user auth in Phase 4; do not introduce auth state. [VERIFIED: ROADMAP.md] |
| V3 Session Management | No [VERIFIED: REQUIREMENTS.md] | Local command state is editor session state, not authenticated session management. [VERIFIED: generated CommandState] |
| V4 Access Control | Yes, Electron privilege boundary [VERIFIED: apps/desktop-electron/src/main/index.ts; preload/index.ts] | Keep context isolation, sandbox, sender URL allowlist, and narrow `videoEditorCore` API. [CITED: https://electronjs.org/docs/latest/tutorial/security; CITED: https://electronjs.org/docs/latest/tutorial/context-isolation] |
| V5 Input Validation | Yes [VERIFIED: generated command payloads] | Generated Rust command contracts and Rust validation reject invalid edits; renderer does UI validation only for ergonomics. [VERIFIED: 03-VERIFICATION.md] |
| V6 Cryptography | No [VERIFIED: REQUIREMENTS.md] | No cryptography introduced in Phase 4. [VERIFIED: ROADMAP.md] |
| V8 Data Protection | Yes, local file/material paths [VERIFIED: generated import/list missing commands] | Do not expose filesystem/Node APIs in renderer; route privileged operations through Electron main/preload and Rust commands. [VERIFIED: preload/main files; CITED: https://electronjs.org/docs/latest/tutorial/security] |

### Known Threat Patterns for Electron/Renderer Stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Renderer gains raw IPC/native access | Elevation of privilege | Do not expose `ipcRenderer`; expose narrow contextBridge functions only. [CITED: https://electronjs.org/docs/latest/api/ipc-renderer; VERIFIED: preload/index.ts] |
| Untrusted navigation calls native bridge | Spoofing/Elevation of privilege | Main process URL checks and preload allowed-renderer URL checks remain. [VERIFIED: main/index.ts; preload/index.ts; electron-smoke.spec.ts] |
| UI hides rejected Rust command | Tampering/Repudiation | Preserve prior draft and show Chinese command error. [VERIFIED: 04-CONTEXT.md; 04-UI-SPEC.md] |
| Renderer constructs FFmpeg command strings | Tampering/Information disclosure | Source guards reject `ffmpeg`/`ffprobe` in renderer; FFmpeg stays in runtime/compiler layers. [VERIFIED: AGENTS.md; package.json] |
| Generated contract drift | Tampering | Keep generated files read-only and run drift check. [VERIFIED: generated headers; package.json] |

## Sources

### Primary (HIGH confidence)

- `AGENTS.md` - project constraints, stack, GSD workflow, terminology, rendering, testing, licensing. [VERIFIED: local file]
- `.planning/PROJECT.md`, `.planning/REQUIREMENTS.md`, `.planning/ROADMAP.md`, `.planning/STATE.md` - product identity, requirements, phase goal, phase status. [VERIFIED: local files]
- `.planning/phases/04-jianying-style-desktop-workspace/04-CONTEXT.md` - locked Phase 4 decisions, scope, verification gates. [VERIFIED: local file]
- `.planning/phases/04-jianying-style-desktop-workspace/04-UI-SPEC.md` - layout, copy, typography, color, component, and verification contract. [VERIFIED: local file]
- `.planning/phases/03-timeline-command-core/03-CONTEXT.md` and `03-VERIFICATION.md` - completed Rust command semantics and command boundary. [VERIFIED: local files]
- `apps/desktop-electron/src/generated/*.ts` - generated command/result/draft types. [VERIFIED: local files]
- `apps/desktop-electron/src/main/index.ts`, `src/preload/index.ts`, `src/renderer/App.tsx`, `tests/electron-smoke.spec.ts` - current Electron and smoke UI patterns. [VERIFIED: local files]
- npm registry and npm downloads API - package versions, publish metadata, downloads, repos, postinstall absence. [VERIFIED: npm registry]
- slopcheck output - all existing direct JS packages rated OK. [VERIFIED: slopcheck output]

### Primary Official Docs (HIGH confidence)

- https://playwright.dev/docs/api/class-electron - Electron launch/application/window control. [CITED: official docs]
- https://playwright.dev/docs/test-snapshots - screenshot comparison with `toHaveScreenshot`. [CITED: official docs]
- https://playwright.dev/docs/api/class-pageassertions - screenshot assertion behavior/options. [CITED: official docs]
- https://electronjs.org/docs/latest/tutorial/context-isolation - context isolation and `contextBridge`. [CITED: official docs]
- https://electronjs.org/docs/latest/tutorial/security - Electron security guidance. [CITED: official docs]
- https://electronjs.org/docs/latest/api/ipc-renderer - `ipcRenderer` should be used from preload and exposed via contextBridge when context isolation is enabled. [CITED: official docs]
- https://electronjs.org/docs/latest/breaking-changes - Electron 29 `ipcRenderer` contextBridge behavior change. [CITED: official docs]
- https://react.dev/learn/updating-objects-in-state - treat object state as read-only. [CITED: official docs]
- https://react.dev/learn/updating-arrays-in-state - treat array state as immutable. [CITED: official docs]

### Secondary (MEDIUM confidence)

- None; current recommendations are based on local project artifacts and official docs. [VERIFIED: research process]

### Tertiary (LOW confidence)

- No unverified web-only sources were used. [VERIFIED: research process]

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - versions and package legitimacy were checked locally, through npm registry/downloads, and slopcheck; no new packages are recommended. [VERIFIED: apps/desktop-electron/package.json; VERIFIED: npm registry; VERIFIED: slopcheck output]
- Architecture: HIGH - command/state boundaries are locked by Phase 4 context and generated contracts; Phase 3 verification confirms Rust command semantics. [VERIFIED: 04-CONTEXT.md; generated contracts; 03-VERIFICATION.md]
- Pitfalls: HIGH for command boundary, English copy, generated drift, and FFmpeg leakage because they are directly evidenced by local files and decisions; MEDIUM for CSS layout-shift root causes because those are partly experience-based. [VERIFIED: local files; ASSUMED]
- Validation: HIGH - existing Playwright Electron harness is present and official docs support Electron launch and screenshots. [VERIFIED: tests/electron-smoke.spec.ts; CITED: Playwright docs]

**Research date:** 2026-06-17 [VERIFIED: environment date]
**Valid until:** 2026-07-17 for local architecture and phase constraints; 2026-06-24 for fast-moving package/doc version assumptions. [ASSUMED]
