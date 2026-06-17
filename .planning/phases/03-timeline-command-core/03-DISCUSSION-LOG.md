# Phase 3: Timeline Command Core - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md - this log preserves the alternatives considered.

**Date:** 2026-06-17
**Phase:** 3-Timeline Command Core
**Areas discussed:** Timeline shape and stacking, Command API and atomic mutation, Source/target timerange edits, Snapping and MainTrackMagnet, Undo/redo command state, MVP text and audio semantics
**Mode:** auto-discuss

---

## Timeline Shape And Stacking

| Option | Description | Selected |
|--------|-------------|----------|
| Single root timeline sequence | Use `Draft.tracks` as the MVP's one editable sequence and keep multi-sequence work out of Phase 3. | yes |
| Add full multi-sequence model now | Introduce project-bin sequences, nested timelines, and multiple active timelines. | |
| Defer sequence semantics to UI | Let Electron arrange tracks visually while Rust stores only loose arrays. | |

**User's choice:** Auto-selected the recommended option from project constraints.
**Notes:** This preserves Phase 2 schema continuity and still satisfies the Phase 3 requirement for at least one editable sequence.

---

## Command API And Atomic Mutation

| Option | Description | Selected |
|--------|-------------|----------|
| Pure Rust command semantics | Implement add/move/split/trim/delete/select behavior in `draft_commands` with Rust-owned contracts. | yes |
| UI-owned timeline mutations | Let renderer mutate `Draft.tracks` directly and submit the result. | |
| Project-store-owned editing | Put edit semantics beside `.veproj` persistence. | |

**User's choice:** Auto-selected the recommended option from architecture constraints.
**Notes:** This follows the locked rule that UI emits commands and Rust owns editing semantics.

---

## Source/Target Timerange Edits

| Option | Description | Selected |
|--------|-------------|----------|
| Explicit integer timerange operations | Commands update `SourceTimerange` and `TargetTimerange` in microseconds. | yes |
| Floating seconds in command payloads | Use seconds in IPC and convert later. | |
| Renderer computes source offsets | Let UI calculate split/trim source offsets during drag/edit. | |

**User's choice:** Auto-selected the recommended option from the time-model constraint.
**Notes:** Split and trim behavior must be proven by exact state tests.

---

## Snapping And MainTrackMagnet

| Option | Description | Selected |
|--------|-------------|----------|
| Rust-owned deterministic snapping | Core computes snap candidates, threshold behavior, and magnet events. | yes |
| Renderer-only drag snapping | UI adjusts visual positions and writes the resulting times. | |
| Defer magnet behavior | Implement ordinary moves now and postpone snapping/magnet tests. | |

**User's choice:** Auto-selected the recommended option from `TIME-05`.
**Notes:** Phase 4 UI should display snapped results/events, not reimplement snapping logic.

---

## Undo/Redo Command State

| Option | Description | Selected |
|--------|-------------|----------|
| Rust-owned serializable command state | Core returns command history state; Electron stores/passes it without interpreting semantics. | yes |
| Electron-owned undo stack | UI records inverse operations or previous drafts. | |
| Native global singleton only | Keep command history in process-global binding state. | |

**User's choice:** Auto-selected the recommended option as the safest desktop-first and future-platform-compatible approach.
**Notes:** Undo history is session state, not part of `.veproj/project.json`.

---

## MVP Text And Audio Semantics

| Option | Description | Selected |
|--------|-------------|----------|
| Semantic text/audio command support | Add text content/style and audio volume/mute semantics, defer rendering/layout. | yes |
| Render-ready text layout now | Include pinned fonts, layout, preview parity, and renderer output in Phase 3. | |
| Defer text/audio entirely | Keep Phase 3 video-only and leave text/audio to later. | |

**User's choice:** Auto-selected the recommended option from Phase 3 requirements.
**Notes:** `TEXT-03`, preview/export parity, waveform caches, and rendering stay in Phase 5.

---

## the agent's Discretion

- Exact module names and payload/response struct names.
- Snapshot-based versus inverse-operation undo implementation, provided tests prove all MVP commands.
- Exact default snap threshold and snap-candidate priority, provided they are deterministic and named.

## Deferred Ideas

- Multiple sequences and nested timelines.
- Rich desktop UI and visual timeline interactions.
- Preview/export/render graph work.
- Advanced effects, transitions, stickers, text bubbles/effects, masks, and compatibility adapters.
