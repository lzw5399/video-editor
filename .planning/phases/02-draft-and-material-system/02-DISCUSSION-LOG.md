# Phase 2: Draft And Material System - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md - this log preserves the alternatives considered.

**Date:** 2026-06-17T01:01:32.047Z
**Phase:** 2-Draft And Material System
**Areas discussed:** Draft bundle shape, Jianying schema vocabulary, Material probing, Missing material recovery
**Mode:** `--auto`

---

## Draft Bundle Shape

| Option | Description | Selected |
|--------|-------------|----------|
| Canonical `.veproj/project.json` | Use bundle directory with `project.json` as only semantic source of truth; derived artifacts live outside semantic model. | yes |
| Loose JSON file | Store a single JSON file without bundle structure. | |
| UI-owned draft state | Let Electron own early project state and move to Rust later. | |

**Auto choice:** Canonical `.veproj/project.json`
**Notes:** This follows PROJECT.md and architecture research. Phase 2 should prove create/save/open round trips before richer timeline commands.

---

## Jianying Schema Vocabulary

| Option | Description | Selected |
|--------|-------------|----------|
| Use Jianying-aligned names internally and externally | Rust/domain/schema/IPC/docs/tests use Draft, Material, Track, Segment, SourceTimerange, TargetTimerange, MainTrackMagnet, Keyframe, Filter, Transition. | yes |
| Translate to generic internal names | Use aliases such as Asset/Clip internally and map at UI boundary. | |
| Defer naming until UI work | Add generic persistence first, rename later. | |

**Auto choice:** Use Jianying-aligned names internally and externally
**Notes:** The user explicitly asked for internal and external terminology to stay aligned with Jianying concepts.

---

## Material Probing

| Option | Description | Selected |
|--------|-------------|----------|
| Rust-owned import with ffprobe metadata | Material import goes through Rust, stores stable IDs and ffprobe-derived metadata, and uses media_runtime boundaries. | yes |
| UI-only import registry | Renderer stores imported media and sends raw paths later. | |
| Metadata later | Store paths only in Phase 2 and probe in preview/export phase. | |

**Auto choice:** Rust-owned import with ffprobe metadata
**Notes:** This satisfies MAT-01 through MAT-03 and keeps media facts available for Phase 3/4/5 without UI-owned semantics.

---

## Missing Material Recovery

| Option | Description | Selected |
|--------|-------------|----------|
| Preserve material and mark recoverable missing state | Loading a draft with unavailable media preserves semantics and returns classified missing-material information. | yes |
| Fail entire draft load | Treat missing media as fatal. | |
| Drop missing entries | Remove missing materials during load. | |

**Auto choice:** Preserve material and mark recoverable missing state
**Notes:** Required by MAT-04 and protects `.veproj` integrity. Relink UI is deferred.

---

## the agent's Discretion

- Planner may choose exact Rust modules, fixture paths, and command names if they preserve Rust ownership and generated-contract gates.
- Planner may decide whether thumbnail generation is implemented in Phase 2 or deferred, but material metadata and missing-material recovery are mandatory.

## Deferred Ideas

- Rich material bin UI, relink UI, timeline edit commands, preview/export cache behavior, and adapter compatibility are deferred to later roadmap phases.
