# Archived Roadmap Phases 11-13

**Archived:** 2026-06-18
**Quick task:** 260618-o2v
**Reason:** Temporarily remove future retiming, effects, and transitions work from active GSD planning while Phase 10.1 continues.

This file preserves the original active GSD content for later restoration. To restore, copy the requirement rows back into `.planning/REQUIREMENTS.md`, copy the phase blocks back into `.planning/ROADMAP.md` after Phase 10.1, recreate any needed `.planning/phases/<phase>-.../` directories, and update `.planning/STATE.md` progress totals.

## Requirements

- **SPEED-01**: Segment speed/变速 is represented as typed semantics that define source/target time mapping after retiming.
- **SPEED-02**: Reverse playback and curve speed have explicit deferred/degraded capability boundaries until implemented.
- **SPEED-03**: Audio follow-speed behavior is explicit and renderable/degradable through the shared preview/export path.

- **FX-01**: Draft distinguishes filter, adjustment, effect, and transition concepts with Jianying-aligned names and typed parameter schemas.
- **FX-02**: First-party filter/adjustment/effect parameters report supported, degraded, or unsupported capabilities through render_graph/compiler outputs.
- **FX-03**: Jianying/Kaipai private native effect IDs remain external compatibility references and are not treated as internal render semantics.

- **TRN-01**: Transitions attach to the correct adjacent or overlapping segment relationship with type, duration, and typed parameters.
- **TRN-02**: Timeline validation defines transition effects on overlap, trim, snapping, and main-track magnet behavior.
- **TRN-03**: render_graph represents transition windows deterministically for preview/export, with unsupported proprietary transitions classified.

## Phase List Entries

- [ ] **Phase 11: Retiming And Speed System** - Segment speed, source/target time mapping, audio follow-speed policy, and deferred reverse/curve-speed boundaries
- [ ] **Phase 12: Filter Adjustment And Effect Semantics** - First-party filter/adjustment/effect parameter schemas plus supported/degraded/unsupported capability boundaries
- [ ] **Phase 13: Transition Semantics And Timeline Integration** - Transition attachment, duration/type/params, overlap/trim/snapping effects, and render graph representation

## Phase Details

### Phase 11: Retiming And Speed System

**Goal**: Add segment speed semantics so templates can express rhythm changes beyond trim/split/move.
**Depends on**: Phase 10.1
**Requirements**: SPEED-01, SPEED-02, SPEED-03
**UI hint**: yes
**Success Criteria** (what must be TRUE):

  1. Segment speed changes are represented as typed semantics that define source/target time mapping after retiming.
  2. Audio follow-speed policy is explicit and renderable/degradable through the shared preview/export path.
  3. Reverse playback and curve speed have explicit deferred or degraded capability boundaries until implemented.
  4. Timeline trim/split/move validation understands retimed segments and remains atomic/undoable.

**Plans**: TBD

### Phase 12: Filter Adjustment And Effect Semantics

**Goal**: Define first-party effect semantics and capability reporting instead of stuffing native effects into opaque strings.
**Depends on**: Phase 11
**Requirements**: FX-01, FX-02, FX-03
**UI hint**: yes
**Success Criteria** (what must be TRUE):

  1. Draft schema distinguishes filter, adjustment, effect, and transition concepts using Jianying-aligned names and typed parameter schemas.
  2. render_graph and ffmpeg_compiler classify each effect parameter as supported, degraded, or unsupported.
  3. Jianying/Kaipai private native effect IDs are external references with compatibility reports, not internal render semantics.
  4. Desktop UI can show deferred/unsupported states in Chinese without pretending the effect is fully renderable.

**Plans**: TBD

### Phase 13: Transition Semantics And Timeline Integration

**Goal**: Implement transition semantics as first-class timeline relationships with deterministic edit and render behavior.
**Depends on**: Phase 12
**Requirements**: TRN-01, TRN-02, TRN-03
**UI hint**: yes
**Success Criteria** (what must be TRUE):

  1. Transitions attach to the correct adjacent or overlapping segment relationship with type, duration, and parameters.
  2. Timeline validation defines how transitions affect overlap, trim, snapping, and main-track magnet behavior.
  3. render_graph represents transition windows deterministically for preview/export compilation.
  4. Unsupported proprietary transitions degrade or report incompatibility rather than becoming opaque supported strings.

**Plans**: TBD

## Traceability Rows

| Requirement | Phase | Status |
|-------------|-------|--------|
| SPEED-01 | Phase 11 | Planned |
| SPEED-02 | Phase 11 | Planned |
| SPEED-03 | Phase 11 | Planned |
| FX-01 | Phase 12 | Planned |
| FX-02 | Phase 12 | Planned |
| FX-03 | Phase 12 | Planned |
| TRN-01 | Phase 13 | Planned |
| TRN-02 | Phase 13 | Planned |
| TRN-03 | Phase 13 | Planned |
