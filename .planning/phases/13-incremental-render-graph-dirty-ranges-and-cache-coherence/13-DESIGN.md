# Phase 13: Incremental Render Graph, Dirty Ranges, And Cache Coherence - Design

**Designed:** 2026-06-18
**Status:** Ready for planning
**Scope:** Types, identity scheme, delta model, cache invalidation model, tests, rollout waves

## Design Summary

Phase 13 adds a semantic change layer between accepted commands and render/cache consumers:

```text
accepted command
  -> CommandDelta
  -> DirtySet
  -> render graph node diff/fingerprints
  -> preview/export/audio/thumb/waveform/proxy/snapshot invalidation
  -> Phase 14 artifact rows
  -> Phase 16 scheduler work units
```

The canonical draft remains `.veproj/project.json`. All render graph snapshots, preview artifacts, thumbnails, waveforms, proxies, and artifact indexes remain derived and rebuildable.

## Proposed Types

### CommandDelta

Define binding-safe types in `draft_model` so Rust commands, Node-API, generated schemas, and desktop code share one contract.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandDelta {
    pub command: CommandName,
    pub changed_entities: Vec<ChangedEntity>,
    pub changed_domains: Vec<DirtyDomain>,
    pub changed_ranges: Vec<DirtyRange>,
    pub invalidation: InvalidationScope,
    pub reason: String,
}

impl CommandDelta {
    pub fn none(command: CommandName, reason: impl Into<String>) -> Self { /* ... */ }
}
```

Add to `TimelineCommandResponse`:

```rust
pub struct TimelineCommandResponse {
    pub draft: Draft,
    pub command_state: CommandState,
    pub selection: TimelineSelection,
    pub events: Vec<CommandEvent>,
    #[serde(default)]
    pub delta: CommandDelta,
}
```

If a default is awkward because `CommandName` has no obvious default, use `Option<CommandDelta>` during migration and require `Some` before Phase 13 exits.

### ChangedEntity

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ChangedEntity {
    Draft { draft_id: DraftId },
    Material { material_id: MaterialId },
    Track { track_id: TrackId },
    Segment { track_id: TrackId, segment_id: SegmentId },
    Keyframe { track_id: TrackId, segment_id: SegmentId, property: KeyframeProperty, at: Microseconds },
    Canvas { draft_id: DraftId },
    RuntimeCapabilities { capability_fingerprint: String },
}
```

Do not put graph node IDs here as the primary command fact. Commands change semantic entities; graph nodes are derived from those entities.

### DirtyDomain

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum DirtyDomain {
    Timing,
    Visual,
    Text,
    Audio,
    Material,
    Effect,
    Filter,
    Transition,
    Canvas,
    OutputProfile,
    RuntimeCapabilities,
    Preview,
    ExportPrep,
    Thumbnail,
    Waveform,
    Proxy,
    GraphSnapshot,
    PreviewCache,
}
```

The first group describes semantic domains; the second group describes derived consumer domains. Keeping both in one enum is acceptable for generated contracts, but implementation helpers should separate "semantic cause" from "consumer effect" internally.

### DirtyRange

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DirtyRange {
    pub target_timerange: TargetTimerange,
    pub source: DirtyRangeSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum DirtyRangeSource {
    Previous,
    Current,
    PreviousAndCurrent,
    FullDraft,
    MaterialWide,
}
```

Ranges are half-open intervals: `[start, start + duration)`. Add shared helpers for checked end, overlap, union, and sorted merge.

### InvalidationScope

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InvalidationScope {
    pub full_draft: bool,
    pub material_ids: Vec<MaterialId>,
    pub graph_node_ids: Vec<String>,
    pub consumer_domains: Vec<DirtyDomain>,
}
```

`full_draft` is the correctness fallback. It should appear in tests for unknown/unsupported precise delta cases, but normal segment edits should avoid it.

## Render Graph Node Identity Scheme

### Node ID Type

Use a structured type in Rust and serialize as fields, not only a freeform string. A derived `stable_key` string can be used for maps and cache keys.

```rust
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderGraphNodeId {
    pub role: RenderGraphNodeRole,
    pub draft_id: DraftId,
    pub track_id: Option<TrackId>,
    pub segment_id: Option<SegmentId>,
    pub material_id: Option<MaterialId>,
    pub local_id: Option<String>,
}
```

```rust
pub enum RenderGraphNodeRole {
    Canvas,
    Material,
    VideoSegment,
    AudioSegment,
    TextOverlay,
    SegmentFilter,
    SegmentTransition,
    AudioMix,
    VideoComposite,
    SampledFrame,
    Output,
}
```

### Stable Keys

Recommended stable key examples:

| Node | Stable Key |
|------|------------|
| Canvas | `draft:{draft_id}:canvas` |
| Material | `draft:{draft_id}:material:{material_id}` |
| Video segment | `draft:{draft_id}:track:{track_id}:segment:{segment_id}:video` |
| Audio segment | `draft:{draft_id}:track:{track_id}:segment:{segment_id}:audio` |
| Text overlay | `draft:{draft_id}:track:{track_id}:segment:{segment_id}:text` |
| Segment filter | `draft:{draft_id}:track:{track_id}:segment:{segment_id}:filter:{index_or_filter_id}` |
| Transition | `draft:{draft_id}:track:{track_id}:segment:{segment_id}:transition` |
| Audio mix | `draft:{draft_id}:audio-mix:{track_id_or_master}` |
| Video composite | `draft:{draft_id}:video-composite:{target_range_or_output_role}` |
| Sampled frame | `draft:{draft_id}:frame:{frame_index}:at:{microseconds}` |
| Output | `draft:{draft_id}:output:{profile_id}` |

Avoid including content hashes in stable keys.

### Fingerprints

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderGraphNodeFingerprint {
    pub node_id: RenderGraphNodeId,
    pub semantic_fingerprint: String,
    pub input_fingerprint: String,
    pub output_profile_fingerprint: String,
    pub runtime_capability_fingerprint: String,
    pub graph_schema_version: u32,
    pub generator_version: String,
}
```

Fingerprint responsibilities:

- `semantic_fingerprint`: fields on the semantic entity that affect rendering.
- `input_fingerprint`: source material fingerprint, relink path/status, decoded capability inputs, font/effect resources when applicable.
- `output_profile_fingerprint`: preview frame, preview segment, export preset, dimensions, frame rate, range, codec/container settings.
- `runtime_capability_fingerprint`: compiler/runtime support matrix, hardware/software fallback capability, relevant FFmpeg/GPU feature set.
- `graph_schema_version` and `generator_version`: force invalidation after graph/fingerprint algorithm changes.

Phase 13 can initially compute deterministic serialized fingerprints in Rust. It should centralize the helper so Phase 14 can reuse the same algorithm.

## Graph Diff Model

```rust
pub struct RenderGraphDiff {
    pub added: Vec<RenderGraphNodeId>,
    pub removed: Vec<RenderGraphNodeId>,
    pub changed: Vec<RenderGraphNodeChange>,
    pub unchanged: Vec<RenderGraphNodeId>,
    pub dirty_ranges: Vec<DirtyRange>,
}

pub struct RenderGraphNodeChange {
    pub node_id: RenderGraphNodeId,
    pub previous_fingerprint: RenderGraphNodeFingerprint,
    pub current_fingerprint: RenderGraphNodeFingerprint,
    pub domains: Vec<DirtyDomain>,
}
```

Diff rule:

1. Same node ID and same fingerprint: unchanged.
2. Same node ID and different fingerprint: changed.
3. Previous ID missing from current graph: removed.
4. Current ID missing from previous graph: added.

This makes move/trim behavior explicit: a segment keeps identity but timing fingerprint and dirty ranges change.

## Delta Rules By Command

| Command | Delta Rule |
|---------|------------|
| `SelectTimelineSegments` | `CommandDelta::none`, no dirty domains. |
| `AddSegment` / `AddAudioSegment` / `AddTextSegment` | entity: segment, track, material; domains: timing plus visual/audio/text/material as applicable; range: new target range. |
| `MoveSegment` | entity: segment, old/new tracks; domains: timing and media domain; ranges: previous range and current range merged. |
| `SplitSegment` | entities: original segment and new right segment; domains: timing and media domain; range: previous original range. |
| `TrimSegment` | entity: segment; domains: timing and media domain; ranges: previous and current ranges merged. |
| `DeleteSegment` | entity: segment, track; domains: timing and media domain; range: previous range. |
| `EditTextSegment` | entity: segment/material; domains: text, visual, preview/export, thumbnail if text thumbnail depends on timeline; range: current target range. |
| `SetSegmentVolume` | entity: segment; domains: audio, waveform preview if timeline mix waveform exists; range: segment target range. |
| `SetTrackMute` | entity: track; domains: audio for audio track, visual for visual track if later exposed; ranges: all contained segment ranges. |
| `UpdateSegmentVisual` | entity: segment; domains: visual, preview, export prep, thumbnail; range: segment target range. |
| `SetSegmentKeyframe` / `RemoveSegmentKeyframe` | entity: keyframe and segment; domain from property; range: segment range initially, narrowed to influence span only after helpers are tested. |
| `UpdateDraftCanvasConfig` | entity: draft/canvas; domains: canvas, output profile, preview, export prep, graph snapshot; range: full draft duration. |
| `UndoTimelineEdit` / `RedoTimelineEdit` | compare previous draft and restored draft or use stored inverse delta; emit deterministic dirty ranges. Snapshot reuse only with exact fingerprint match. |

## Dirty Range Propagation Model

### DirtySet

```rust
pub struct DirtySet {
    pub entities: Vec<ChangedEntity>,
    pub semantic_domains: Vec<DirtyDomain>,
    pub consumer_domains: Vec<DirtyDomain>,
    pub ranges: Vec<DirtyRange>,
    pub material_ids: Vec<MaterialId>,
    pub full_draft: bool,
}
```

Processing steps:

1. Normalize command-specific facts into `DirtySet`.
2. Validate range ends using checked integer arithmetic.
3. Sort and merge overlapping or adjacent target ranges.
4. Expand semantic domains into consumer domains.
5. Emit preview/export/artifact invalidation requests.

### Consumer Expansion

| Semantic Cause | Consumers |
|----------------|-----------|
| Timing | preview, export prep, audio if audio segment, thumbnails, proxies, graph snapshots, preview cache |
| Visual | preview, export prep, thumbnails, proxies, graph snapshots, preview cache |
| Text | preview, export prep, thumbnails, graph snapshots, preview cache |
| Audio | preview segment cache, export prep, audio engine, waveform/mix waveform, graph snapshots |
| Material | preview/export, thumbnails, waveforms, proxies, graph snapshots, preview cache |
| Canvas/output profile | preview/export, thumbnails, proxies, graph snapshots, preview cache |
| Runtime capabilities | all fingerprints/artifacts that include runtime capability fingerprint |

Waveforms need two categories in later implementation:

- Source material waveform: dirty on material relink/replacement or audio decode capability changes.
- Timeline mix waveform: dirty on segment timing, volume, mute, audio effects, and keyframed volume.

## Cache Invalidation Model

### Preview Cache Key v2

```rust
pub struct PreviewCacheKey {
    pub key_id: String,
    pub profile: PreviewCacheProfile,
    pub target_timerange: TargetTimerange,
    pub graph_node_ids: Vec<RenderGraphNodeId>,
    pub semantic_fingerprint: String,
    pub input_fingerprint: String,
    pub output_profile_fingerprint: String,
    pub runtime_capability_fingerprint: String,
    pub material_dependencies: Vec<MaterialId>,
    pub artifact_schema_version: u32,
    pub generator_version: String,
}
```

Keep the old shape only as a migration bridge if needed. New tests should assert that localized unrelated changes do not change the key for unaffected ranges/nodes.

### PreviewInvalidationRequest v2

```rust
pub struct PreviewInvalidationRequest {
    pub dirty_ranges: Vec<DirtyRange>,
    pub changed_material_ids: Vec<MaterialId>,
    pub changed_graph_node_ids: Vec<RenderGraphNodeId>,
    pub changed_domains: Vec<DirtyDomain>,
    pub runtime_capability_fingerprint: Option<String>,
    pub full_draft: bool,
    pub reason: String,
}
```

Invalidation predicate:

1. If `full_draft`, invalidate all entries.
2. If range overlaps and domain/profile applies, invalidate.
3. If material dependency intersects, invalidate.
4. If graph node dependency intersects, invalidate.
5. If runtime/output/profile fingerprint mismatch, invalidate.
6. Otherwise retain.

## Undo/Redo Strategy

### Recommended Default: Deterministic Invalidation

When undo/redo restores a draft snapshot, compute a delta between the current draft and restored draft, or store the inverse delta alongside the history snapshot. Then invalidate affected ranges/domains just like a normal accepted command.

This is the required correctness path.

### Optional Snapshot Reuse

Add optional session-only cache metadata to history snapshots only if memory is bounded:

```rust
pub struct CommandHistorySnapshot {
    pub draft: Draft,
    pub selection: TimelineSelection,
    pub label: Option<String>,
    pub graph_snapshot: Option<GraphSnapshotRef>,
}

pub struct GraphSnapshotRef {
    pub draft_generation: u64,
    pub graph_fingerprint: String,
    pub node_fingerprints: Vec<RenderGraphNodeFingerprint>,
}
```

Reuse rule:

- Reuse graph/cache snapshot only when restored draft generation/fingerprint and every relevant node/artifact fingerprint match.
- Otherwise run deterministic invalidation.

Do not persist these refs into `.veproj/project.json`.

## Staging Before Phase 14 And Phase 16

### Phase 13 Must Produce

- Binding-safe delta and dirty domain contracts.
- Render graph node IDs and fingerprints.
- In-memory graph diff summaries.
- Preview cache invalidation using dirty ranges, material IDs, graph node IDs, and fingerprints.
- Export-prep invalidation contract even if export still rebuilds full jobs.
- Tests proving localized edits do not force unrelated range invalidation.

### Phase 14 Will Add

- `.veproj/derived/artifact-store.sqlite`.
- Artifact dependency rows keyed by node IDs/fingerprints, ranges, output profile, runtime capability, schema version, and generator version.
- Blob paths, generation status, replacement/relink invalidation, GC, and quotas.

### Phase 16 Will Add

- Dirty work units from Phase 13 invalidation facts.
- Priority queues, cancellation, stale-generation rejection, starvation control, queue telemetry, and resource budgets.
- Background scheduling for proxy/waveform/thumbnail/export/cache jobs without starving interactive preview.

## Testing Strategy

### Unit Tests

- `draft_model`: schema exports include `CommandDelta`, `ChangedEntity`, `DirtyDomain`, `DirtyRange`, `InvalidationScope`, graph node ID references if exposed.
- `draft_commands`: every accepted mutating command emits expected entities/domains/ranges.
- `draft_commands`: selection-only command emits no dirty domains.
- `draft_commands`: undo/redo emits deterministic invalidation.
- `engine_core`: dirty full-draft range helper uses normalized duration and checked arithmetic.
- `render_graph`: stable node IDs remain stable across content edits; fingerprints change.
- `render_graph`: added/removed/changed/unchanged diff buckets are deterministic.
- `preview_service`: invalidation by range, material, graph node ID, runtime fingerprint, and full draft.

### Integration Tests

- Large timeline with hundreds or thousands of segments: moving one segment changes only old/new ranges and one segment node fingerprint.
- Text edit changes text overlay node and affected preview/export ranges, not unrelated audio/material nodes.
- Volume edit changes audio node/fingerprint and preview segment/export audio dirty range, not frame PNG thumbnails unless policy says mixed audio preview affects them.
- Canvas/profile change invalidates full draft preview/export/graph snapshots.
- Undo/redo after localized move returns to previous graph fingerprint or invalidates exactly the union range.

### Source Guards

Create `scripts/phase13-source-guards.sh`:

- Reject `f32`/`f64` time in `draft_model`, `draft_commands`, command schema, generated TypeScript, and render graph dirty contracts.
- Reject renderer direct mutation of `draft.tracks`, `track.segments`, graph node IDs, cache keys, dirty ranges, or preview invalidation decisions.
- Reject FFmpeg command construction in renderer.
- Require generated contracts contain `CommandDelta`, `DirtyDomain`, and `RenderGraphNodeId`.
- Require `.veproj/project.json` schema stays free of preview caches, artifact rows, graph snapshots, FFmpeg scripts, waveform/proxy paths, and cache metadata.

### Recommended Commands

```bash
cargo test -p draft_model delta -- --nocapture
cargo test -p draft_commands delta -- --nocapture
cargo test -p render_graph node_identity -- --nocapture
cargo test -p preview_service dirty -- --nocapture
cargo test -p testkit large_timeline_incremental -- --nocapture
pnpm run test:contracts
```

Add:

```json
{
  "test:phase13-rust": "cargo test -p draft_model delta -- --nocapture && cargo test -p draft_commands delta -- --nocapture && cargo test -p render_graph node_identity -- --nocapture && cargo test -p preview_service dirty -- --nocapture",
  "test:phase13-source-guards": "bash scripts/phase13-source-guards.sh",
  "test:phase13": "pnpm run test:phase13-rust && pnpm run test:phase13-source-guards && pnpm run test:contracts"
}
```

## Rollout Waves

### Wave 0: Test And Contract Harness

- Add schema/contract tests for new delta/dirty/node ID types.
- Add source guard script.
- Add package scripts.
- Add large-timeline fixtures/helpers.

### Wave 1: CommandDelta Core

- Add `CommandDelta` types.
- Add range helper functions.
- Populate deltas for simple commands: add, delete, move, split, trim, selection no-op.
- Add undo/redo deterministic invalidation fallback.

### Wave 2: Domain Coverage

- Populate deltas for text, audio, visual, keyframe, canvas/profile, track mute.
- Add material dependency dirty expansion helpers.
- Add consumer domain expansion.

### Wave 3: Render Graph Identity

- Add `RenderGraphNodeId` and node fingerprint types.
- Attach IDs/fingerprints to graph materials, video layers, audio mixes, text overlays, sampled frames, and output/composite roles.
- Add graph diff helper and snapshots.

### Wave 4: Preview/Export Cache Coherence

- Upgrade preview cache key and invalidation request.
- Ensure preview cache retention works for unrelated localized edits.
- Add export-prep invalidation contract; export may still rebuild full job but must receive correct dirty facts.

### Wave 5: Large-Timeline And Parity Gates

- Add large localized edit tests.
- Add preview/export consistency checks after edit and undo/redo.
- Run full phase gates and contract drift checks.

## Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Too much graph snapshot data in undo history | Memory growth | Store lightweight fingerprint maps only; deterministic invalidation remains default. |
| Dirty ranges too narrow | Stale preview/export artifacts | Start conservative with segment/full draft ranges, then narrow with tests. |
| Fingerprint algorithm churn | Cache misses and migration pain | Include generator/schema version and centralize fingerprint helper. |
| Renderer starts computing cache decisions | Architecture drift | Source guards and command-only tests. |
| Phase 14 store assumptions leak into Phase 13 | Overbuild | Keep persistence out; expose in-memory keys and dependency facts only. |

## Acceptance Criteria

- Every accepted mutating command emits a non-empty `CommandDelta`; selection-only commands emit a no-op delta.
- `CommandDelta` includes changed entities, dirty domains, and integer-microsecond dirty ranges or an explicit full-draft fallback.
- Render graph nodes have stable semantic IDs distinct from fingerprints.
- Node fingerprints change when relevant semantic/input/output/runtime data changes.
- Preview cache invalidation can retain unrelated entries after localized edits.
- Undo/redo invalidates deterministically or reuses snapshots only with exact fingerprint match.
- Large-timeline tests prove localized edit cost and invalidation scope are bounded.
- Contract/schema/generated files are updated and drift checks pass.
