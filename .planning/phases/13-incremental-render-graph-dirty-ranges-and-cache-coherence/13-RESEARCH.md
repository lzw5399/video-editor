# Phase 13: Incremental Render Graph, Dirty Ranges, And Cache Coherence - Research

**Researched:** 2026-06-18
**Domain:** Rust semantic command deltas, render graph identity, dirty range propagation, preview/cache coherence
**Confidence:** HIGH for local architecture findings, MEDIUM for recommended staging details

## User Constraints

- Write only `13-CONTEXT.md`, `13-RESEARCH.md`, and `13-DESIGN.md`. [VERIFIED: user request]
- Do not modify product source files. [VERIFIED: user request]
- Do not modify Phase 10.1 files. [VERIFIED: user request]
- Do not commit. [VERIFIED: user request]
- Focus research on `CommandDelta`, stable render graph node identity, dirty range propagation, undo/redo snapshot strategy, and staging before Phase 14 and Phase 16. [VERIFIED: user request]

## Summary

Phase 13 should add a semantic delta layer rather than replacing the existing draft, command, engine, or preview/export pipeline. Commands already mutate drafts in Rust and return `TimelineCommandResponse`; Phase 13 should extend that contract with a binding-safe `CommandDelta` carrying changed entities, domains, and integer-microsecond dirty ranges. [VERIFIED: crates/draft_model/src/lib.rs; crates/draft_commands/src/timeline.rs]

Render graph generation currently rebuilds from normalized draft and sampled range state, and preview cache keys currently include a fingerprint of the whole draft. This is correct but coarse: a localized edit invalidates everything that uses the draft fingerprint. Phase 13 should preserve the same semantic pipeline while adding stable node identities and per-node fingerprints so graph diffing and cache invalidation can become targeted. [VERIFIED: crates/render_graph/src/graph.rs; crates/preview_service/src/service.rs]

**Primary recommendation:** Implement `CommandDelta` in the command contract, add stable render graph node IDs plus separate fingerprints in `render_graph`, and route all cache invalidation through a domain-aware `DirtySet`/`DirtyRange` model that Phase 14 can persist and Phase 16 can schedule. [VERIFIED: .planning/REQUIREMENTS.md; .planning/notes/production-editor-architecture-decisions.md]

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|--------------|----------------|-----------|
| Command delta emission | Rust command core | Binding bridge | Accepted edits are validated and applied in `draft_commands`; bindings only transport responses. [VERIFIED: crates/draft_commands/src/timeline.rs; crates/bindings_node/src/lib.rs] |
| Dirty range derivation | Rust command/core services | Preview/export services | Dirty ranges are semantic consequences of accepted edits and must use Rust-owned integer time. [VERIFIED: AGENTS.md; .planning/REQUIREMENTS.md] |
| Render graph node identity | `render_graph` | `engine_core` | Graph roles are produced in `render_graph` from normalized semantic entities and range state. [VERIFIED: crates/render_graph/src/graph.rs; crates/engine_core/src/normalize.rs] |
| Node fingerprints | `render_graph` | Preview/export/artifact consumers | Fingerprints describe current graph node content and runtime/output inputs; consumers use them for reuse decisions. [VERIFIED: .planning/notes/production-editor-architecture-decisions.md] |
| Preview cache invalidation | `preview_service` | Binding bridge | Preview cache contracts already live in `preview_service`; binding command adapts payload entries. [VERIFIED: crates/preview_service/src/cache.rs; crates/bindings_node/src/preview_export_service.rs] |
| Export preparation invalidation | Rust export service path | FFmpeg compiler | Export path builds normalized range and graph before compiling; compiler should not decide editing behavior. [VERIFIED: crates/bindings_node/src/preview_export_service.rs; docs/runtime-boundaries.md] |
| Artifact persistence | Phase 14 artifact store | Preview service | Phase 13 should define keys and invalidation facts; SQLite/blob persistence is Phase 14. [VERIFIED: .planning/ROADMAP.md] |
| Work scheduling | Phase 16 scheduler | Preview/audio/artifact services | Phase 13 should expose dirty work units; priority/cancellation policy is later. [VERIFIED: .planning/notes/production-editor-architecture-decisions.md] |

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| INCR-01 | Stable graph node IDs separate from fingerprints | Current graph structs lack node IDs; add semantic role/entity IDs and per-node fingerprints. [VERIFIED: crates/render_graph/src/graph.rs] |
| INCR-02 | Accepted commands emit `CommandDelta` | Current command responses contain draft, command state, selection, and events only. [VERIFIED: crates/draft_model/src/lib.rs] |
| INCR-03 | Dirty propagation spans preview/export/audio/thumbnails/waveforms/proxies/cache | Current preview invalidation supports ranges/materials only; export/audio/proxy/waveform scopes need explicit domains. [VERIFIED: crates/preview_service/src/cache.rs; .planning/REQUIREMENTS.md] |
| INCR-04 | Undo/redo restores state and cache coherence | Current history snapshots store draft/selection/label only; no graph/cache metadata exists. [VERIFIED: crates/draft_model/src/lib.rs; crates/draft_commands/src/history.rs] |
| INCR-05 | Large-timeline graph diff and consistency tests | Existing tests cover snapshots/cache overlap, not large localized diff cost. [VERIFIED: crates/preview_service/tests/cache_invalidation.rs; crates/render_graph/tests/render_graph_snapshots.rs] |

## Local Architecture Findings

### Command And History Surface

`TimelineCommandResponse` currently contains `draft`, `command_state`, `selection`, and `events`; it does not contain delta data. [VERIFIED: crates/draft_model/src/lib.rs]

`CommandHistorySnapshot` is documented as session-only and stores `draft`, `selection`, and optional `label`; it is not persisted to `.veproj/project.json`. [VERIFIED: crates/draft_model/src/lib.rs]

Undo and redo pop a draft snapshot and return `undoCommitted` or `redoCommitted` events; they do not calculate changed ranges between restored and current drafts. [VERIFIED: crates/draft_commands/src/history.rs]

Timeline edit commands clone the draft, mutate a segment/track/canvas field, validate, push an undo snapshot, and return events; this is the right hook for building deltas from pre/post state. [VERIFIED: crates/draft_commands/src/timeline.rs; crates/draft_commands/src/audio.rs; crates/draft_commands/src/text.rs; crates/draft_commands/src/visual.rs; crates/draft_commands/src/keyframe.rs; crates/draft_commands/src/canvas.rs]

Selection-only commands change selection without mutating draft semantics, so they should emit `CommandDelta::none()` or no dirty domains. [VERIFIED: crates/draft_commands/src/timeline.rs]

### Time And Range Model

`Microseconds` is a `u64` newtype, and `TargetTimerange` stores `start` and `duration` as `Microseconds`. [VERIFIED: crates/draft_model/src/time.rs; crates/draft_model/src/timeline.rs]

`resolve_render_range` samples frames using rational frame-rate conversion through integer arithmetic; dirty range code should reuse this model instead of introducing float time. [VERIFIED: crates/engine_core/src/frame_state.rs]

Timerange overlap exists in preview cache invalidation as half-open interval logic. [VERIFIED: crates/preview_service/src/cache.rs]

### Engine And Render Graph

`engine_core::normalize_draft` produces `NormalizedDraft`, `NormalizedTrack`, and `NormalizedSegment` with draft ID, track ID, segment ID, material ID, source/target ranges, visual/text/audio state, keyframes, filters, and transition. [VERIFIED: crates/engine_core/src/normalize.rs]

`resolve_frame_state` resolves active visual layers, audio segments, and text overlays at an integer microsecond timeline position. [VERIFIED: crates/engine_core/src/frame_state.rs]

`build_render_graph` currently returns lists for materials, video layers, audio mixes, text overlays, sampled frames, sampled animation states, and diagnostics. These entries carry semantic IDs but not a first-class stable node ID or fingerprint. [VERIFIED: crates/render_graph/src/graph.rs]

`RenderGraphPlan` combines a `RenderGraph` with an output profile; output profile target ranges and dimensions are already typed and validated. [VERIFIED: crates/render_graph/src/profile.rs]

### Preview And Export Cache Surface

`PreviewCacheKey` currently contains `profile`, `target_timerange`, `semantic_fingerprint`, and `material_dependencies`. [VERIFIED: crates/preview_service/src/cache.rs]

Preview preparation normalizes the whole draft, resolves the requested range, builds a render graph, computes `semantic_fingerprint(draft)` by serializing the whole draft, and derives the cache key from that whole-draft fingerprint. [VERIFIED: crates/preview_service/src/service.rs]

`PreviewInvalidationRequest` currently supports `changed_ranges`, `changed_material_ids`, and a freeform reason. [VERIFIED: crates/preview_service/src/cache.rs]

The binding preview invalidation command reconstructs `PreviewCacheEntry` values from binding payloads and calls `invalidate_preview_cache`; it does not carry dirty domains. [VERIFIED: crates/bindings_node/src/preview_export_service.rs]

Export preparation in the binding service also normalizes the whole draft, resolves the full export range, builds the render graph, compiles an FFmpeg job, and writes sidecars. [VERIFIED: crates/bindings_node/src/preview_export_service.rs]

### Project Constraints

AGENTS.md requires UI command-only behavior, Rust-owned timeline semantics, integer/rational time, render graph isolation from FFmpeg, derived artifacts outside canonical `.veproj/project.json`, and executable gates per roadmap phase. [VERIFIED: AGENTS.md]

`docs/runtime-boundaries.md` requires pure semantic crates to avoid preview/runtime/filesystem/platform dependencies and places preview runtime boundaries in `preview_service`. [VERIFIED: docs/runtime-boundaries.md]

Production architecture decisions explicitly require `CommandDelta`, stable node identity separate from fingerprints, and dirty range propagation across preview, export prep, audio, thumbnails, waveforms, proxy, and cache invalidation. [VERIFIED: .planning/notes/production-editor-architecture-decisions.md]

## Design Options

### Option A: Whole-Draft Fingerprint Invalidation

Keep the existing whole-draft fingerprint and invalidate all artifacts after accepted edits. [VERIFIED: crates/preview_service/src/service.rs]

This is simple but fails the large-timeline intent because any localized edit changes the whole semantic fingerprint and prevents targeted reuse. [VERIFIED: .planning/ROADMAP.md]

**Disposition:** Reject as primary Phase 13 design; keep as explicit full-invalidate fallback for unknown commands or schema mismatches. [ASSUMED]

### Option B: Command-Specific Handwritten Invalidation Only

Each command emits handcrafted preview/cache invalidation ranges and skips graph-level node identity. [VERIFIED: crates/draft_commands/src/timeline.rs]

This would satisfy some preview invalidation needs but would not give render graph snapshots or future artifact store entries a stable way to identify unchanged semantic nodes. [VERIFIED: .planning/REQUIREMENTS.md]

**Disposition:** Reject as incomplete for INCR-01 and Phase 14 staging. [VERIFIED: .planning/REQUIREMENTS.md]

### Option C: Semantic Delta Plus Graph Identity And Fingerprints

Accepted commands emit `CommandDelta`; render graph entries carry stable node IDs and fingerprints; preview/export/artifact consumers use dirty domains and ranges to invalidate targeted artifacts. [VERIFIED: .planning/REQUIREMENTS.md; .planning/notes/production-editor-architecture-decisions.md]

This preserves Rust-owned semantics, keeps `.veproj/project.json` canonical, and lets Phase 14 persist the same keys without moving SQLite work into Phase 13. [VERIFIED: AGENTS.md; .planning/ROADMAP.md]

**Disposition:** Recommended. [VERIFIED: .planning/REQUIREMENTS.md]

## Recommendation

Use Option C with these implementation boundaries:

- Add binding-safe delta types in `draft_model` and populate them in `draft_commands`. [VERIFIED: crates/draft_model/src/lib.rs; crates/draft_commands/src/timeline.rs]
- Add reusable range helpers for union/intersection/merge using `TargetTimerange` and `Microseconds`. [VERIFIED: crates/draft_model/src/time.rs; crates/preview_service/src/cache.rs]
- Add graph node identity and fingerprint structs in `render_graph`, derived from draft ID, graph node role, and semantic entity IDs. [VERIFIED: crates/render_graph/src/graph.rs]
- Replace whole-draft-only preview cache keys with graph/content/runtime fingerprint components while keeping backward-compatible invalidation fallbacks. [VERIFIED: crates/preview_service/src/cache.rs; crates/preview_service/src/service.rs]
- Keep snapshots session-only; prefer deterministic invalidation on undo/redo unless a matching graph/cache snapshot has exact draft generation and fingerprint matches. [VERIFIED: crates/draft_model/src/lib.rs; crates/draft_commands/src/history.rs]

## Proposed Standard Stack

No new external packages are required. [VERIFIED: local Cargo manifests]

| Component | Existing Crate | Purpose | Phase 13 Use |
|-----------|----------------|---------|--------------|
| Command contracts | `draft_model` | Serde/schema/TypeScript command surface | Define `CommandDelta`, dirty domains, changed entities, and response fields. [VERIFIED: crates/draft_model/Cargo.toml] |
| Command execution | `draft_commands` | Rust-owned timeline edit semantics | Emit deltas after accepted commands and undo/redo. [VERIFIED: crates/draft_commands/src/timeline.rs] |
| Normalization/ranges | `engine_core` | Deterministic frame/range state | Reuse integer/rational time and frame sampling rules. [VERIFIED: crates/engine_core/src/frame_state.rs] |
| Graph metadata | `render_graph` | Renderer-neutral render intent graph | Add node IDs, fingerprints, and graph diff summaries. [VERIFIED: crates/render_graph/src/graph.rs] |
| Cache invalidation | `preview_service` | Preview artifacts/cache boundary | Consume dirty domains/ranges and node fingerprints. [VERIFIED: crates/preview_service/src/cache.rs] |
| Binding bridge | `bindings_node` | Electron command transport | Expose generated delta and invalidation contracts without UI-owned logic. [VERIFIED: crates/bindings_node/src/preview_export_service.rs] |

## Package Legitimacy Audit

No external packages should be installed for Phase 13. [VERIFIED: local Cargo manifests]

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| None | — | — | — | — | — | No install needed |

**Packages removed due to slopcheck [SLOP] verdict:** none.
**Packages flagged as suspicious [SUS]:** none.

## Architecture Patterns

### System Architecture Diagram

```text
Accepted UI command
  -> bindings_node command envelope
  -> draft_commands validates and mutates Draft
  -> TimelineCommandResponse { draft, command_state, selection, events, delta }
  -> DirtySet { entities, domains, target ranges, material/runtime/profile scope }
  -> engine_core normalize/resolve affected range
  -> render_graph builds/diffs nodes with stable identities
  -> fingerprints decide reuse
  -> consumers:
       preview cache invalidation
       export prep invalidation
       audio dirty spans
       thumbnail dirty spans
       waveform dirty spans
       proxy dirty spans
       graph snapshot invalidation
       Phase 14 artifact index keys
       Phase 16 scheduler work units
```

### Pattern 1: Delta Is A Semantic Command Result

`CommandDelta` should describe the accepted semantic change, not the UI action or renderer-side state. [VERIFIED: AGENTS.md; crates/draft_commands/src/timeline.rs]

Recommended fields:

```rust
pub struct CommandDelta {
    pub command: CommandName,
    pub changed_entities: Vec<ChangedEntity>,
    pub changed_domains: Vec<DirtyDomain>,
    pub changed_ranges: Vec<DirtyRange>,
    pub invalidation: InvalidationScope,
    pub reason: String,
}
```

### Pattern 2: Identity Is Not Fingerprint

Node identity should be deterministic from semantic role and stable IDs, for example `segment:video:{track_id}:{segment_id}` or `material:{material_id}`. Fingerprints should hash current semantic fields, source material fingerprint, output profile, runtime capability fingerprint, artifact schema version, and generator version where relevant. [VERIFIED: .planning/REQUIREMENTS.md; .planning/notes/production-editor-architecture-decisions.md]

### Pattern 3: Dirty Ranges Are Half-Open Integer Intervals

Preview cache overlap already behaves as `start < other_end && other_start < end`. Phase 13 should formalize this range behavior and share it across dirty propagation. [VERIFIED: crates/preview_service/src/cache.rs]

### Pattern 4: Deterministic Fallback Beats Stale Reuse

If a command cannot precisely compute dirty ranges, it should emit full draft/profile/material invalidation for the affected domains. This is less efficient but preserves correctness. [VERIFIED: .planning/REQUIREMENTS.md]

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Persisted time math | Float seconds or renderer-local time conversion | `Microseconds`, `TargetTimerange`, `RationalFrameRate` | Project constraints require integer/rational time. [VERIFIED: AGENTS.md] |
| UI cache decisions | Renderer-side graph/cache invalidation | Rust `CommandDelta` + preview/artifact invalidation APIs | UI must emit commands only. [VERIFIED: AGENTS.md] |
| Cache as canonical state | Store semantic truth in cache/artifact metadata | `.veproj/project.json` plus rebuildable derived artifacts | Project format constraint forbids derived artifacts as canonical semantics. [VERIFIED: AGENTS.md] |
| Node identity by hash | Content hash as graph node ID | Semantic graph node ID + content/runtime fingerprints | INCR-01 requires stable semantic identities separate from fingerprints. [VERIFIED: .planning/REQUIREMENTS.md] |
| Scheduler in Phase 13 | Priority queues, starvation control, cancellation policy | Dirty work-unit contracts only | Phase 16 owns scheduler. [VERIFIED: .planning/notes/production-editor-architecture-decisions.md] |
| SQLite artifact store in Phase 13 | On-disk artifact index and blob GC | Phase 14 artifact-store contracts | Phase 14 owns SQLite artifact persistence. [VERIFIED: .planning/ROADMAP.md] |

## Common Pitfalls

### Pitfall 1: Content Hash Masquerading As Identity

**What goes wrong:** A changed visual or text setting changes a hash and appears as a removed node plus added node. [VERIFIED: .planning/REQUIREMENTS.md]
**How to avoid:** Keep `RenderGraphNodeId` stable and compare fingerprints separately. [VERIFIED: .planning/REQUIREMENTS.md]

### Pitfall 2: Old Range Omitted On Move/Trim

**What goes wrong:** Moving or trimming a segment invalidates the new position but leaves stale cached output at the old position. [ASSUMED]
**How to avoid:** Delta helpers must include both previous and next target ranges for timing edits. [ASSUMED]

### Pitfall 3: Selection Commands Mark Timeline Dirty

**What goes wrong:** Pure selection changes churn preview/cache state. [VERIFIED: crates/draft_commands/src/timeline.rs]
**How to avoid:** Selection-only commands should return no semantic dirty domains. [VERIFIED: crates/draft_commands/src/timeline.rs]

### Pitfall 4: Undo/Redo Reuses Artifacts Without Fingerprint Match

**What goes wrong:** A restored draft state may look similar but have different material/runtime/profile inputs. [ASSUMED]
**How to avoid:** Snapshot reuse requires exact draft generation and node/artifact fingerprint match; otherwise deterministic invalidation wins. [VERIFIED: .planning/REQUIREMENTS.md]

### Pitfall 5: Domain Collapse To "Everything"

**What goes wrong:** Every edit becomes a full cache purge, so large-timeline requirements are not met. [VERIFIED: .planning/ROADMAP.md]
**How to avoid:** Emit dirty domains for timing, visual, text, audio, material, effect/filter, transition, canvas/profile, and runtime capability. [VERIFIED: .planning/notes/production-editor-architecture-decisions.md]

## Dirty Domain Matrix

| Edit Type | Entities | Domains | Ranges |
|-----------|----------|---------|--------|
| Add segment | track, segment, material | timing plus media kind domain | new target range. [VERIFIED: crates/draft_commands/src/timeline.rs] |
| Move segment | source/target track, segment | timing, preview/export, media kind domain | old range union new range. [VERIFIED: crates/draft_commands/src/timeline.rs] |
| Split segment | track, left/right segments | timing, media kind domain | original range. [VERIFIED: crates/draft_commands/src/timeline.rs] |
| Trim segment | track, segment | timing, media kind domain | old range union new range. [VERIFIED: crates/draft_commands/src/timeline.rs] |
| Delete segment | track, segment | timing, media kind domain | old range. [VERIFIED: crates/draft_commands/src/timeline.rs] |
| Edit text | segment, material | text, visual, preview/export | segment target range. [VERIFIED: crates/draft_commands/src/text.rs] |
| Volume/keyframe volume | segment | audio | segment range or keyframe influence span. [VERIFIED: crates/draft_commands/src/audio.rs; crates/draft_commands/src/keyframe.rs] |
| Track mute | track and contained segments | audio or visual depending track kind | all contained segment ranges. [VERIFIED: crates/draft_commands/src/audio.rs] |
| Segment visual | segment | visual | segment target range. [VERIFIED: crates/draft_commands/src/visual.rs] |
| Canvas/profile | draft | canvas/profile, preview/export, graph snapshots | full draft duration. [VERIFIED: crates/draft_commands/src/canvas.rs; crates/engine_core/src/normalize.rs] |
| Material relink/status | material and dependent segments | material, proxies, thumbnails, waveform, preview/export | all dependent segment ranges or material-wide. [VERIFIED: crates/engine_core/src/normalize.rs] |

## State Of The Art

| Old Approach | Current Phase 13 Approach | When Changed | Impact |
|--------------|---------------------------|--------------|--------|
| Whole-draft semantic fingerprint for preview cache | Per-node semantic identity plus content/input/runtime fingerprints | Phase 13 | Enables targeted reuse after localized edits. [VERIFIED: crates/preview_service/src/service.rs; .planning/REQUIREMENTS.md] |
| Command events only | Command events plus `CommandDelta` | Phase 13 | Lets preview/export/cache consumers react deterministically. [VERIFIED: crates/draft_model/src/lib.rs] |
| Range/material preview invalidation only | Domain-aware dirty set with range/material/entity/runtime/profile scopes | Phase 13 | Stages Phase 14 artifact keys and Phase 16 work units. [VERIFIED: crates/preview_service/src/cache.rs; .planning/notes/production-editor-architecture-decisions.md] |
| Draft-only undo snapshots | Draft snapshots plus deterministic delta/invalidation replay, optional graph snapshot refs | Phase 13 | Avoids stale artifact reuse across undo/redo. [VERIFIED: crates/draft_commands/src/history.rs; .planning/REQUIREMENTS.md] |

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Rust framework | Cargo test with workspace Rust 1.95.0. [VERIFIED: Cargo.toml; package.json] |
| Schema/contracts | `draft_model` schema export tests plus generated TypeScript drift checks. [VERIFIED: package.json] |
| Desktop | Playwright Electron tests. [VERIFIED: apps/desktop-electron/package.json] |
| Quick run command | `cargo test -p draft_commands delta -- --nocapture && cargo test -p render_graph incremental -- --nocapture && cargo test -p preview_service dirty -- --nocapture` [ASSUMED] |
| Full suite command | `pnpm run test:phase13 && pnpm run test:contracts` [ASSUMED] |

### Phase Requirements → Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| INCR-01 | Stable node IDs remain stable while fingerprints change | unit/snapshot | `cargo test -p render_graph node_identity -- --nocapture` | ❌ Wave 0 |
| INCR-02 | Commands emit changed entities/domains/ranges | unit/contract | `cargo test -p draft_commands delta -- --nocapture` | ❌ Wave 0 |
| INCR-03 | Dirty domains route to preview/export/audio/thumb/waveform/proxy/snapshot scopes | unit | `cargo test -p preview_service dirty -- --nocapture` | ❌ Wave 0 |
| INCR-04 | Undo/redo emits deterministic invalidation or exact snapshot reuse metadata | unit | `cargo test -p draft_commands undo_redo_delta -- --nocapture` | ❌ Wave 0 |
| INCR-05 | Large localized edit diff avoids full graph regeneration and preserves preview/export consistency | integration/perf guard | `cargo test -p render_graph large_timeline_incremental -- --nocapture && cargo test -p testkit preview_export_parity -- --nocapture` | ❌ Wave 0 / ✅ parity baseline |

### Wave 0 Gaps

- [ ] `crates/draft_commands/tests/command_delta.rs` covers INCR-02 and INCR-04. [ASSUMED]
- [ ] `crates/render_graph/tests/node_identity.rs` covers INCR-01 and graph diff behavior. [ASSUMED]
- [ ] `crates/preview_service/tests/dirty_propagation.rs` covers INCR-03. [ASSUMED]
- [ ] `crates/testkit/tests/large_timeline_incremental.rs` or render_graph integration test covers INCR-05. [ASSUMED]
- [ ] `scripts/phase13-source-guards.sh` blocks renderer-owned dirty/cache/graph logic and float time. [ASSUMED]
- [ ] `package.json` gains `test:phase13-rust`, `test:phase13-source-guards`, and `test:phase13`. [ASSUMED]

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | no | Phase 13 has no authentication surface. [VERIFIED: .planning/ROADMAP.md] |
| V3 Session Management | no | Command history is editor session state, not user auth session management. [VERIFIED: crates/draft_model/src/lib.rs] |
| V4 Access Control | no | No multi-user authorization scope in this phase. [VERIFIED: .planning/PROJECT.md] |
| V5 Input Validation | yes | Validate command delta contracts, range overflow, non-empty IDs, known dirty domains, and schema-generated payloads. [VERIFIED: crates/draft_model/src/validation.rs; crates/draft_commands/src/timeline.rs] |
| V6 Cryptography | no | Fingerprints are cache/version identifiers, not security hashes or signatures. [ASSUMED] |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malformed ranges causing overflow | Tampering/DoS | Use checked `u64` arithmetic and existing timerange validation patterns. [VERIFIED: crates/draft_commands/src/timeline.rs; crates/engine_core/src/frame_state.rs] |
| Cache path confusion | Tampering | Keep cache/artifact paths service-owned; UI transports entries but does not compute filesystem paths. [VERIFIED: crates/preview_service/src/service.rs; AGENTS.md] |
| Stale work overwriting current state | Tampering | Carry generation/snapshot identifiers for later scheduler/runtime rejection. [VERIFIED: .planning/notes/production-editor-architecture-decisions.md] |

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| Rust/Cargo | Rust tests/build | ✓ | Workspace requires Rust 1.95.0. [VERIFIED: Cargo.toml] | Install matching toolchain |
| pnpm | Contract and desktop tests | Required by scripts. [VERIFIED: package.json] | Run Rust-only gates if desktop unavailable |
| Playwright/Electron | Desktop command-only guards | Configured in desktop package. [VERIFIED: apps/desktop-electron/package.json] | Rust/source-guard validation for non-UI Phase 13 core |
| FFmpeg | Preview/export parity smoke | Existing preview/export pipeline uses FFmpeg executor. [VERIFIED: crates/preview_service/src/service.rs] | Use non-runtime unit tests for graph/delta correctness |

**Missing dependencies with no fallback:** none identified for research. [VERIFIED: local files]

**Missing dependencies with fallback:** FFmpeg availability may affect parity smoke, but core delta/graph/cache tests can run without executing FFmpeg. [VERIFIED: crates/preview_service/src/service.rs]

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Full-invalidate fallback should remain available for unknown commands or schema mismatches. | Design Options | If wrong, planner may under-spec precise deltas for rare edits. |
| A2 | Move/trim must include old and new ranges to avoid stale old-position cache. | Common Pitfalls | If wrong, invalidation could be overbroad but still correct. |
| A3 | Exact snapshot reuse on undo/redo should require fingerprint match; otherwise invalidate. | Common Pitfalls | If too strict, fewer cache hits but safer behavior. |
| A4 | Proposed test file names and package scripts do not exist yet. | Validation Architecture | Planner must create or adjust names during planning. |
| A5 | Fingerprints are cache identifiers, not cryptographic security controls. | Security Domain | If later used for trust/security, stronger hashing/signing review is needed. |

## Resolved Questions

1. **[RESOLVED] `CommandDelta` lives directly on `TimelineCommandResponse`.**
   - Decision: Phase 13 exits with accepted command responses carrying `TimelineCommandResponse.delta` as the structured semantic change contract. [VERIFIED: crates/draft_model/src/lib.rs]
   - Implementation bridge: serde/default or a migration-compatible optional field may be used while updating call sites and generated contracts, but the completed Phase 13 behavior requires delta data to be present for every accepted command response. [ASSUMED]
   - Rejected alternative: `CommandEvent` metadata is not the primary carrier because events are UI/reporting facts, while `CommandDelta` is the Rust-owned semantic invalidation contract. [VERIFIED: AGENTS.md; crates/draft_model/src/lib.rs]

2. **[RESOLVED] Fingerprinting is centralized behind deterministic non-security fingerprint APIs.**
   - Decision: Phase 13 defines deterministic fingerprint helper APIs with graph schema version and generator version fields; callers compare fingerprint values but do not depend on a specific algorithm. [VERIFIED: .planning/REQUIREMENTS.md; .planning/notes/production-editor-architecture-decisions.md]
   - Implementation bridge: the current whole-draft FNV-style helper can inform migration tests, but the algorithm remains an implementation detail and must not be presented as a security hash, signature, or trust boundary. [VERIFIED: crates/preview_service/src/service.rs]
   - Required wording for implementers: fingerprints are cache/version identifiers for semantic/input/output/runtime reuse decisions; V6 cryptography does not apply unless a later phase uses them for security. [ASSUMED]

3. **[RESOLVED] Undo history stores semantic draft snapshots plus optional lightweight graph/cache refs only.**
   - Decision: Undo/redo correctness is based on restoring semantic draft snapshots and then emitting deterministic invalidation for changed ranges/domains. [VERIFIED: crates/draft_model/src/lib.rs; crates/draft_commands/src/history.rs]
   - Optional metadata: history may carry lightweight session-only graph snapshot refs or fingerprint maps when bounded and exact-matchable, but it must not persist large render graph snapshots, sampled frame states, cache entries, or derived artifact metadata. [VERIFIED: AGENTS.md; .planning/ROADMAP.md]
   - Required fallback: deterministic invalidation is mandatory whenever exact graph/cache fingerprint evidence is absent or mismatched; no large graph snapshots are persisted into `.veproj/project.json`. [VERIFIED: .planning/REQUIREMENTS.md]

## Sources

### Primary (HIGH confidence)

- `AGENTS.md` - project architecture, terminology, time model, derived artifact, and testing constraints.
- `.planning/PROJECT.md` - active project constraints and product scope.
- `.planning/ROADMAP.md` - Phase 13 goal, dependencies, success criteria, and Phase 14/16 staging.
- `.planning/REQUIREMENTS.md` - INCR-01 through INCR-05.
- `.planning/notes/production-editor-architecture-decisions.md` - confirmed post-10.1 architecture decisions.
- `docs/runtime-boundaries.md` - crate/runtime boundary constraints.
- `crates/draft_model/src/lib.rs`, `time.rs`, `timeline.rs` - command response/history/time contracts.
- `crates/draft_commands/src/*.rs` - accepted edit command patterns and undo/redo behavior.
- `crates/engine_core/src/normalize.rs`, `frame_state.rs` - normalized semantic and range evaluation paths.
- `crates/render_graph/src/graph.rs`, `profile.rs` - current graph and output profile contracts.
- `crates/preview_service/src/cache.rs`, `service.rs` - current preview cache key/invalidation and preview preparation.
- `crates/bindings_node/src/preview_export_service.rs` - binding preview invalidation and export preparation paths.
- `package.json`, `justfile` - executable validation commands.

### Secondary (MEDIUM confidence)

- None. No external technical sources were needed for this codebase-local research.

### Tertiary (LOW confidence)

- Assumptions listed in the Assumptions Log.

## Metadata

**Confidence breakdown:**

- Command delta shape: MEDIUM - current command locations are clear, exact response compatibility needs implementation confirmation.
- Render graph identity: HIGH - requirements explicitly require identity separate from fingerprints and current graph lacks first-class node IDs.
- Dirty propagation: MEDIUM - preview cache path is clear, but thumbnails/waveforms/proxies are mostly future Phase 14/16 consumers.
- Undo/redo strategy: MEDIUM - current history behavior is clear, snapshot memory tradeoffs need implementation measurement.
- Validation: MEDIUM - existing test infrastructure is clear, new test names are proposed.

**Research date:** 2026-06-18
**Valid until:** 2026-07-18
