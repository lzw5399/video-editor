# Phase 4: Jianying-Style Desktop Workspace - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md - this log preserves the alternatives considered.

**Date:** 2026-06-17
**Phase:** 04-jianying-style-desktop-workspace
**Areas discussed:** Workspace structure, Chinese terminology, panels and inspector, timeline command boundary, verification gates

---

## Workspace Structure

| Option | Description | Selected |
|--------|-------------|----------|
| Jianying-like editor workspace | First screen is a production editor shell with top categories, left panel, preview, inspector, and timeline. | yes |
| Generic dashboard shell | Use generic cards and dashboard-style panels as the first UI. | |
| Landing/marketing page | Build a hero/landing page before the editor. | |

**User's choice:** Auto-selected recommended option based on prior user direction.
**Notes:** The user explicitly asked for a Jianying-like editor and warned that UI should not look bad. Project rules also say sites/apps should build the usable experience first, not a landing page.

---

## Chinese Terminology

| Option | Description | Selected |
|--------|-------------|----------|
| Simplified Chinese visible UI | Use Chinese for panel titles, controls, empty states, errors, accessibility labels where user-facing, and test-visible labels. | yes |
| Mixed English/Chinese | Keep current English smoke labels and only translate some product copy. | |
| English desktop UI | Use English labels for the first desktop MVP. | |

**User's choice:** User explicitly added that desktop language is Chinese.
**Notes:** Generated TypeScript/Rust names can remain English code identifiers, but visible UI must use Chinese and Jianying-style editing vocabulary.

---

## Panels And Inspector

| Option | Description | Selected |
|--------|-------------|----------|
| MVP real panels with reserved categories | Implement material/media, text, and audio panels now while visibly reserving sticker/effect/transition/filter/adjustment categories. | yes |
| Only material bin | Keep Phase 4 near the current smoke app and defer text/audio panels. | |
| Full advanced effect library | Attempt stickers/effects/transitions/filter presets in Phase 4. | |

**User's choice:** Auto-selected recommended MVP scope from ROADMAP and prior context.
**Notes:** Phase 4 must make the workspace feel like Jianying, but advanced effect rendering and preset parity are deferred.

---

## Timeline Command Boundary

| Option | Description | Selected |
|--------|-------------|----------|
| Command-only Rust integration | Renderer stores/display returned Draft, CommandState, and TimelineSelection, and all edits call generated command envelopes. | yes |
| UI-owned timeline mutation | Renderer directly mutates Draft.tracks and segment arrays, then syncs later. | |
| Partial local edit repair | UI retries or repairs invalid edits after Rust rejection. | |

**User's choice:** Auto-selected from locked architecture constraints and Phase 3 decisions.
**Notes:** The Rust core owns editing semantics, undo/redo, snapping, and MainTrackMagnet. Electron may display and pass state, but not interpret or mutate semantics.

---

## Verification Gates

| Option | Description | Selected |
|--------|-------------|----------|
| Playwright plus source and visual guards | Add Phase 4 scripts for workspace flow, Chinese UI labels, layout stability, command-only source guards, and generated contract drift. | yes |
| Manual UI review only | Rely on manual visual inspection and broad build/test gates. | |
| Final-only E2E | Skip per-slice UI tests until Phase 6. | |

**User's choice:** Auto-selected from the user's "每一步都怎么测试" requirement and roadmap `TEST-06`.
**Notes:** Phase 4 plans should add executable checks per slice and finish with Electron Playwright coverage.

---

## the agent's Discretion

- Exact React component split and CSS organization.
- Exact fixture draft and command helper shape for UI tests.
- Exact desktop colors/spacing, provided the result is a polished Jianying-style editor and not a generic dashboard.

## Deferred Ideas

- Preview frame rendering, waveform generation, render graph, export, packaged app tests, mobile UI, server rendering, and advanced proprietary effects remain later phases.
