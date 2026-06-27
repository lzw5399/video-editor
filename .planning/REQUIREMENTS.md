# Requirements: Video Editor v1.1 Usability & Export

**Defined:** 2026-06-27
**Core Value:** Users can reliably import media, edit clips on a familiar Jianying-style timeline, preview the result, save the draft, and export a video through one consistent editing and rendering model.

## v1.1 Requirements

### Real Editing UAT

- [ ] **UAT11-01**: User can complete a packaged product E2E session that imports mixed media, edits a long timeline, previews through the production compositor, saves, reopens, exports, and continues editing.
- [ ] **UAT11-02**: User can repeat edit, save, reopen, and export cycles without semantic drift, stale preview/export state, or derived artifact pollution of `.veproj/project.json`.

### Long Timeline

- [ ] **LONG11-01**: User can work on a long multi-track timeline with selection, scroll, zoom, scrub, move, trim, split, undo, redo, and preview within documented responsiveness budgets.
- [ ] **LONG11-02**: Export, artifact generation, probing, and cache work do not block playhead scrub, inspector edits, preview delivery, or interaction-session commit and cancel paths.

### Interaction And Shortcuts

- [ ] **INT11-01**: User-facing high-frequency controls use Rust-owned interaction sessions where updates do not save, increment revision, or push undo, and commit creates one canonical mutation.
- [ ] **SHORT11-01**: User can use common desktop editing shortcuts that are focus-safe, discoverable, and routed through Rust-owned intents or interaction sessions.

### Crop And Export

- [ ] **CROP11-01**: User-visible crop export validates, clamps, or rejects crop rectangles against decoded source dimensions before FFmpeg runtime execution and reports typed diagnostics.
- [ ] **CROP11-02**: Supported crop behavior has preview/export parity for video, image, imported template, and small-source fixtures.
- [ ] **EXP11-01**: User sees export progress, cancel, success, blocked, degraded, unsupported, and failed states in product language with actionable diagnostics.

### Phase 19 Parity

- [ ] **FX11-01**: Existing Phase 19 retime, effect, filter, transition, mask, and blend support has a preview/export parity matrix backed by product evidence.
- [ ] **FX11-02**: Every visible supported effect or timeline capability is backed by Rust capability facts, while unsupported or degraded paths cannot report product success.

### Diagnostics And Boundaries

- [ ] **DIAG11-01**: Export, effect, and crop diagnostics expose typed codes, product-safe copy, affected draft targets where possible, and opt-in developer details.
- [ ] **ADAPT11-01**: External adapter provider IDs and raw provider payloads remain limited to adapter, report, or provenance boundaries and do not become first-party render semantics.
- [ ] **GATE11-01**: Product success cannot be satisfied by fallback, mock, artifact, CPU probe, DOM overlay, native single-video proof, first-frame snapshot, or file-exists-only export evidence.

### UI Polish

- [ ] **UI11-01**: UI cleanup changes only expose backed behavior or explicit gated states, and every visible changed control has product click-through evidence.
- [ ] **UI11-02**: The editor has no overlapping, clipped, debug, or raw backend copy at 1120x720, 1280x800, and a crowded long-timeline state.

## Future Requirements

### Broader Effects And Providers

- **FX12-01**: User can access a broader first-party effect, filter, transition, and preset library after the current Phase 19 support set is preview/export reliable.
- **ADAPT12-01**: User can import broader Jianying, CapCut, or Kaipai draft subsets through adapters with compatibility reports, without making proprietary semantics first-party render semantics.
- **PROV12-01**: User can use live provider integrations after offline adapter boundaries and first-party rendering reliability are stable.

### Platform Expansion

- **MOB12-01**: User can use mobile product surfaces backed by the same draft, preview, and export semantics.
- **CLOUD12-01**: User can use cloud or server rendering product UX backed by the same render graph and runtime contracts.

### AI Workflows

- **AI12-01**: User can use AI-assisted editing workflows only after the general-purpose editor core remains reliable under v1.1 product UAT.

## Out of Scope

| Feature | Reason |
|---------|--------|
| Broad new effect/filter/transition library | v1.1 closes preview/export reliability for the existing Phase 19 capability set before expanding breadth. |
| Full proprietary Jianying/CapCut/Kaipai parity | Proprietary IDs and private presets are external adapter/report facts, not first-party render semantics. |
| Live provider integrations | v1.1 is a desktop usability/export closure milestone, not an external service integration milestone. |
| Mobile app UI and cloud rendering product UX | Portable runtime boundaries exist, but v1.1 focuses on the Electron desktop product. |
| AI oral-video, ASR, auto-highlight, or digital-human workflows | These are outside the current general-purpose editor product identity. |
| Renderer-owned FFmpeg, render graph, crop, retime, effect, cache, or timeline semantics | Violates the production ownership boundary and would create preview/export drift. |
| Product success from derived artifacts or fallback evidence | v1.1 must prove the production chain, not a fallback, mock, DOM, artifact, CPU, or file-exists-only path. |

## Traceability

Traceability is filled during roadmap creation. Each v1.1 requirement must map to exactly one Phase 20+ phase.

| Requirement | Phase | Status |
|-------------|-------|--------|
| UAT11-01 | Phase 20 | Pending |
| UAT11-02 | Phase 20 | Pending |
| LONG11-01 | Phase 20 | Pending |
| LONG11-02 | Phase 20 | Pending |
| INT11-01 | Phase 21 | Pending |
| SHORT11-01 | Phase 21 | Pending |
| CROP11-01 | Phase 22 | Pending |
| CROP11-02 | Phase 22 | Pending |
| EXP11-01 | Phase 22 | Pending |
| FX11-01 | Phase 23 | Pending |
| FX11-02 | Phase 23 | Pending |
| DIAG11-01 | Phase 23 | Pending |
| ADAPT11-01 | Phase 23 | Pending |
| GATE11-01 | Phase 20 | Pending |
| UI11-01 | Phase 24 | Pending |
| UI11-02 | Phase 24 | Pending |

**Coverage:**
- v1.1 requirements: 16 total
- Mapped to phases: 16
- Unmapped: 0

---
*Requirements defined: 2026-06-27*
*Last updated: 2026-06-27 after v1.1 research and user confirmation*
