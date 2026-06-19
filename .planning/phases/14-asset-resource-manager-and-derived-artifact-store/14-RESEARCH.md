# Phase 14: Asset Resource Manager And Derived Artifact Store - Research

**Researched:** 2026-06-19
**Domain:** Rust desktop resource management, SQLite artifact index, derived media cache
**Confidence:** HIGH for codebase boundaries and SQLite/BLAKE3 package choice; MEDIUM for future sync manifest shape because remote rendering is later scope.

## User Constraints

- Phase 14 must add a production resource layer backed by a project-local SQLite artifact index and derived blob store for materials, fonts, effects, proxies, thumbnails, waveforms, graph snapshots, and preview artifacts. [VERIFIED: prompt]
- It must address ASSET-01, ASSET-02, ASSET-03, ASSET-04, and ASSET-05. [VERIFIED: .planning/REQUIREMENTS.md]
- The renderer must not construct FFmpeg commands, render graphs, cache keys, artifact fingerprints, or derived invalidation decisions. [VERIFIED: AGENTS.md]
- `.veproj/project.json` remains the canonical semantic truth; artifact indexes, thumbnails, waveforms, proxies, preview caches, graph snapshots, FFmpeg scripts, and exports are derived. [VERIFIED: AGENTS.md]
- Time/range/fingerprint contracts must use integer microseconds, frame indices, stable IDs, or rational rates; persisted semantics must not use naked floating point time. [VERIFIED: AGENTS.md]
- Do not read, modify, move, or rely on the untracked `reference/` directory. [VERIFIED: prompt]
- No Phase 14 `CONTEXT.md` exists, so there are no locked discussion decisions to copy. [VERIFIED: codebase grep]

## Project Constraints (from AGENTS.md)

- UI emits commands; Rust core owns project and timeline semantics; no UI code may directly construct FFmpeg commands. [VERIFIED: AGENTS.md]
- `.veproj/project.json` is canonical, while render graphs, FFmpeg scripts, thumbnails, waveform data, proxy files, and preview caches are derived artifacts. [VERIFIED: AGENTS.md]
- Product language, desktop code, Rust types, IPC commands, docs, schema, and tests should follow Jianying concepts such as draft/material/track/segment/keyframe/filter/transition. [VERIFIED: AGENTS.md]
- Core time math must use integer microseconds, frame indices, or rational frame rates. [VERIFIED: AGENTS.md]
- Render Graph isolates editing semantics from FFmpeg; FFmpeg Runtime executes jobs and reports progress/errors, but does not decide editing behavior. [VERIFIED: AGENTS.md]
- Kdenlive and MLT are conceptual references only; do not copy GPL code, assets, XML definitions, presets, or UI implementation. [VERIFIED: AGENTS.md]
- External drafts go through adapters and compatibility reports; proprietary IDs are external references, not internal render semantics. [VERIFIED: AGENTS.md]
- Each roadmap phase must define executable gates before implementation is complete. [VERIFIED: AGENTS.md]
- FFmpeg distribution must be reviewed for LGPL/GPL/nonfree options, notices, and commercial obligations. [VERIFIED: AGENTS.md]
- Before file-changing work, use GSD workflow entry points unless the user explicitly bypasses them; this research was started through `init.phase-op` fallback because `gsd-tools` was unavailable on PATH but available through the bundled Node script. [VERIFIED: shell probe]

## Summary

Phase 14 should add a Rust-owned derived-artifact subsystem without changing the canonical draft schema. Existing `project_store` writes and opens `.veproj/project.json`, validates material URIs, and tests that derived fields such as preview caches are rejected from the draft. [VERIFIED: crates/project_store/src/bundle.rs; crates/project_store/tests/project_bundle.rs] Existing Phase 13 code already emits `CommandDelta`, `DirtyRange`, `DirtyDomain`, `RenderGraphSnapshot`, graph node fingerprints, `PreviewInvalidationRequest`, and `ExportPrepDirtyFacts`; Phase 14 should persist and query those facts rather than recomputing them in Electron. [VERIFIED: crates/draft_model/src/delta.rs; crates/render_graph/src/fingerprint.rs; crates/preview_service/src/cache.rs]

The standard implementation should use a new Rust workspace crate, `artifact_store`, with internal modules for SQLite schema/migrations, blob paths, source-resource indexing, source fingerprinting, actual proxy/thumbnail/waveform generation, job state, invalidation, GC, quota, and manifests. A separate `asset_resource_manager` crate is deferred unless a later phase proves a clean acyclic boundary is needed. [RESOLVED: 14-CONTEXT.md; VERIFIED: current preview_service stores artifacts only by filesystem path; crates/preview_service/src/service.rs] `preview_service` should become a consumer of the store for preview artifacts, while `bindings_node` exposes generated transport commands for status and maintenance only. [VERIFIED: crates/preview_service/src/service.rs]

**Primary recommendation:** Use `rusqlite` for `.veproj/derived/artifact-store.sqlite`, `blake3` for source/blob fingerprints, and project-relative blob paths under `.veproj/derived/blobs`, with SQLite rows as the source of truth for derived artifact validity. [VERIFIED: crates.io/docs.rs + slopcheck; CITED: docs.rs/rusqlite; CITED: docs.rs/blake3]

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|--------------|----------------|-----------|
| Resource/source indexing | API / Backend (Rust core) | Database / Storage | Material/resource IDs and source fingerprints affect artifact validity and must be Rust-owned. [VERIFIED: AGENTS.md] |
| SQLite artifact index | Database / Storage | API / Backend | SQLite stores derived artifact rows; Rust owns schema, migrations, and transactions. [CITED: sqlite.org/lang_transaction.html] |
| Blob store paths | API / Backend (Rust core) | Database / Storage | Rust must produce project-relative derived blob paths and reject traversal/symlink escape. [VERIFIED: crates/project_store/src/paths.rs] |
| Preview/thumbnail/waveform/proxy generation | API / Backend (Rust core) | Media Runtime | Generation uses FFmpeg/native runtime traits, but editing semantics and artifact keys stay outside runtime. [VERIFIED: AGENTS.md; crates/media_runtime/src/job.rs] |
| Artifact status display | Browser / Client | Frontend Server/Main IPC | UI may display status, progress, and errors from generated contracts, but must not compute keys or invalidation. [VERIFIED: scripts/phase13-source-guards.sh] |
| GC/quota/sync manifest | API / Backend (Rust core) | Database / Storage | Deletion and sync eligibility must follow dependency rows and fingerprints, not renderer heuristics. [ASSUMED] |

## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| ASSET-01 | Asset manager indexes materials, proxies, thumbnails, waveforms, fonts, and supported effect resources with stable IDs and project-relative references. | `draft_model::Material` already has stable `MaterialId`; add derived resource rows keyed by resource ID/material ID and project-relative artifact URIs, not draft fields. [VERIFIED: crates/draft_model/src/material.rs; crates/project_store/src/paths.rs] |
| ASSET-02 | Derived artifacts are tracked in `.veproj/derived/artifact-store.sqlite` with schema version, runtime capability fingerprint, source material fingerprint, graph fingerprint, generation parameters, dependency rows, dirty state, and generation status. | Use SQLite metadata/migration tables, artifact rows, dependency rows, and job/chunk rows. SQLite supports transactions and PRAGMAs needed for foreign keys/WAL. [CITED: sqlite.org/foreignkeys.html; CITED: sqlite.org/wal.html] |
| ASSET-03 | Replacing, relinking, renaming, or deleting source media invalidates or regenerates exactly affected artifacts. | Reuse Phase 13 `CommandDelta`, changed material IDs, graph node keys, and dirty domains to mark dependent rows dirty; compute source fingerprints from files when source URIs resolve. [VERIFIED: crates/preview_service/src/cache.rs; crates/project_store/src/paths.rs] |
| ASSET-04 | Proxy, thumbnail, and waveform generation is chunked, resumable, cancellable, and isolated from interactive preview responsiveness. | Use artifact job/chunk rows and existing `media_runtime::CancelToken`; defer priority scheduler to Phase 16 but keep artifact generation on a dedicated worker boundary. [VERIFIED: crates/media_runtime/src/job.rs; .planning/ROADMAP.md] |
| ASSET-05 | Cache garbage collection, storage quotas, and optional cloud/server synchronization manifests are defined before remote rendering depends on them. | Add quota policy rows, mark-and-sweep GC, tombstones, and a sync manifest table that records relative blob paths and fingerprints without uploading in Phase 14. [ASSUMED] |

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `rusqlite` [VERIFIED: crates.io/docs.rs + slopcheck] | 0.40.1 | SQLite access for `.veproj/derived/artifact-store.sqlite` | Official docs describe it as an ergonomic Rust wrapper for SQLite, and it exposes transactions/prepared statements needed for a local artifact index. [CITED: docs.rs/rusqlite] |
| `blake3` [VERIFIED: crates.io/docs.rs + slopcheck] | 1.8.5 | Source file, generation-parameter, and blob content fingerprints | Official docs describe it as the official Rust implementation of the BLAKE3 cryptographic hash function with incremental/file-reader hashing APIs. [CITED: docs.rs/blake3] |
| SQLite WAL mode [CITED: sqlite.org/wal.html] | SQLite 3.51.0 available locally | Concurrent local artifact index reads while generation updates rows | SQLite docs state WAL is activated via `PRAGMA journal_mode=WAL` and has checkpoint tradeoffs that must be managed. [CITED: sqlite.org/wal.html] |

### Supporting

| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `fs2` [VERIFIED: crates.io/docs.rs + slopcheck] | 0.4.3 | Advisory project-level lock for blob-store maintenance | Use for optional `.veproj/derived/.artifact-store.lock` during GC/compaction; do not rely on it as the only correctness mechanism because file locks are advisory. [CITED: docs.rs/fs2] |
| Existing `serde`/`serde_json` [VERIFIED: Cargo.toml] | 1.0.228 / 1.0.150 | Serialize generation parameters, graph snapshots, and sync manifest payloads | Already used across Rust contracts and deterministic graph fingerprints. [VERIFIED: Cargo.toml; crates/render_graph/src/fingerprint.rs] |
| Existing `tempfile` [VERIFIED: Cargo.toml] | 3.27.0 | Crash/partial-write tests for bundle and artifact store behavior | Already used in project-store, preview, binding, and testkit tests. [VERIFIED: Cargo.toml] |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `rusqlite` | `sqlx` | `sqlx` is stronger for async/server DBs, but Phase 14 is a local embedded SQLite index and the current Rust services are synchronous. [ASSUMED] |
| `blake3` | `sha2` | SHA-256 is more conventional for interchange, but BLAKE3 is faster for large media and has official Rust file-reader APIs; sync manifests can still include an algorithm field. [CITED: docs.rs/blake3] |
| `fs2` lock | SQLite-only locking | SQLite protects DB writes, but blob GC/compaction touches files outside SQLite; advisory locks reduce multi-process maintenance races. [ASSUMED] |

**Installation:**

```bash
cargo add --package artifact_store rusqlite blake3 fs2
```

**Version verification:** `cargo search` and `cargo info` confirmed `rusqlite 0.40.1`, `blake3 1.8.5`, and `fs2 0.4.3` from crates.io on 2026-06-19. [VERIFIED: crates.io]

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| `rusqlite` | crates.io | Created 2014-11-21 | 73,088,131 total / 21,166,829 recent | github.com/rusqlite/rusqlite | OK | Approved |
| `blake3` | crates.io | Created 2019-09-17 | 138,617,758 total / 30,126,469 recent | github.com/BLAKE3-team/BLAKE3 | OK | Approved |
| `fs2` | crates.io | Created 2015-09-02 | 66,377,058 total / 11,983,541 recent | github.com/danburkert/fs2-rs | OK | Approved as optional advisory lock |

**Packages removed due to slopcheck [SLOP] verdict:** none.
**Packages flagged as suspicious [SUS]:** none.

Note: `slopcheck install --ecosystem crates.io rusqlite blake3 fs2` reported all three packages `[OK]`, then failed at `cargo add` because the workspace has multiple packages and no target package was specified. No dependency was added during research. [VERIFIED: shell output]

## Architecture Patterns

### System Architecture Diagram

```text
Accepted Rust command / material relink / runtime capability change
  -> CommandDelta + DirtyRange + DirtyDomain + changed material IDs
  -> Resource Manager resolves source URIs against .veproj
  -> Source fingerprint probe (BLAKE3 + size + mtime + probe facts)
  -> RenderGraphSnapshot / graph node fingerprints when graph-dependent
  -> ArtifactStore transaction
       -> resource rows
       -> artifact rows
       -> dependency rows
       -> generation job/chunk rows
       -> dirty/ready/failed/cancelled state
  -> BlobStore writes temp file under .veproj/derived/blobs/tmp
       -> fsync + atomic rename to content-addressed/project-relative path
  -> SQLite commit records ready artifact path + blob fingerprint
  -> Preview/audio/export/UI read generated status and project-relative artifact refs
  -> GC/quota scans rows and deletes only unreferenced blobs
```

### Recommended Project Structure

```text
crates/
├── artifact_store/          # SQLite schema, migrations, blob paths, resource indexing, generation, invalidation, GC, quota, manifest
├── preview_service/         # consume artifact_store for preview cache rows instead of path-only cache entries
├── project_store/           # bundle root, project.json, path classification; no SQLite or generation semantics
├── draft_model/             # generated transport/status contracts only; Draft remains semantic-only
└── bindings_node/           # command dispatch and JSON transport; no artifact key/fingerprint decisions
```

### Pattern 1: SQLite Index Is A Derived Store, Not Draft State

**What:** Store artifact state in `.veproj/derived/artifact-store.sqlite`, with `project.json` remaining semantic-only. [VERIFIED: AGENTS.md; crates/project_store/tests/project_bundle.rs]

**When to use:** Every proxy, thumbnail, waveform, graph snapshot, preview artifact, sidecar, and generated manifest row. [VERIFIED: .planning/ROADMAP.md]

**Example:**

```sql
-- Source: SQLite transaction/foreign-key docs, adapted for Phase 14.
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;
PRAGMA user_version = 1;

CREATE TABLE artifact (
  artifact_id TEXT PRIMARY KEY,
  artifact_kind TEXT NOT NULL,
  stable_key TEXT NOT NULL UNIQUE,
  blob_relative_path TEXT,
  blob_fingerprint TEXT,
  source_fingerprint TEXT,
  graph_fingerprint TEXT,
  runtime_capability_fingerprint TEXT,
  generation_parameters_json TEXT NOT NULL,
  status TEXT NOT NULL,
  dirty INTEGER NOT NULL DEFAULT 0,
  bytes INTEGER NOT NULL DEFAULT 0,
  created_at_unix_ms INTEGER NOT NULL,
  updated_at_unix_ms INTEGER NOT NULL
);
```

### Pattern 2: Atomic Blob Write Then SQLite Commit

**What:** Write generated bytes to a temp path under `.veproj/derived/blobs/tmp`, fsync the file, rename into the final content-addressed path, then commit the SQLite row in a transaction. [VERIFIED: project_store already uses temp write + sync + replace for project.json; crates/project_store/src/lib.rs]

**When to use:** Any preview frame, proxy segment, waveform chunk, thumbnail tile, graph snapshot JSON, or FFmpeg sidecar. [VERIFIED: .planning/ROADMAP.md]

**Example:**

```rust
// Source: project_store atomic-write pattern + rusqlite Transaction docs.
let tx = conn.transaction()?;
write_blob_temp_then_rename(&blob_root, &artifact_id, bytes)?;
tx.execute(
    "UPDATE artifact SET status = ?1, blob_relative_path = ?2, blob_fingerprint = ?3 WHERE artifact_id = ?4",
    (&"ready", &relative_path, &blob_fingerprint, &artifact_id),
)?;
tx.commit()?;
```

### Pattern 3: Dependency Rows Drive Invalidation

**What:** Persist dependencies by resource/material ID, graph node key, dirty domain, and optional integer target/source ranges. [VERIFIED: Phase 13 dirty facts exist in crates/preview_service/src/cache.rs]

**When to use:** Replacing a source material, changing runtime capabilities, changing graph fingerprints, or editing a timeline range. [VERIFIED: crates/draft_model/src/delta.rs]

**Example:**

```sql
CREATE TABLE artifact_dependency (
  artifact_id TEXT NOT NULL REFERENCES artifact(artifact_id) ON DELETE CASCADE,
  dependency_kind TEXT NOT NULL,
  dependency_key TEXT NOT NULL,
  target_start_us INTEGER,
  target_duration_us INTEGER,
  source_start_us INTEGER,
  source_duration_us INTEGER,
  dirty_domain TEXT,
  PRIMARY KEY (artifact_id, dependency_kind, dependency_key, target_start_us, target_duration_us)
);
```

### Anti-Patterns To Avoid

- **Putting derived rows in `Draft` or `project.json`:** Project-store tests already reject derived fields; keep artifact metadata under `.veproj/derived`. [VERIFIED: crates/project_store/tests/project_bundle.rs]
- **Renderer-supplied cache roots as durable behavior:** Current UI passes `/tmp/video-editor-preview-cache`; Phase 14 should move project-local artifact root resolution into Rust. [VERIFIED: apps/desktop-electron/src/renderer/App.tsx]
- **mtime-only invalidation:** mtime/size can be an optimization, but source validity needs content fingerprinting for replacement/relink correctness. [ASSUMED]
- **Deleting blobs by path prefix only:** GC must consult artifact/dependency rows and tombstones before deletion. [ASSUMED]
- **Using graph fingerprint as node identity:** Phase 13 separates stable graph node keys from fingerprints; preserve that split in artifact keys and dependencies. [VERIFIED: crates/render_graph/src/incremental.rs; crates/render_graph/src/fingerprint.rs]

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Embedded artifact index | Ad hoc JSON index files | SQLite through `rusqlite` | Transactions, constraints, indexes, and migrations are required for crash recovery and dependency queries. [CITED: docs.rs/rusqlite; CITED: sqlite.org/lang_transaction.html] |
| Source/blob fingerprints | FNV helper or mtime-only checks | `blake3::Hasher` | Existing FNV helper is deterministic but not appropriate as a collision-resistant media fingerprint; BLAKE3 docs provide incremental/file-reader hashing. [VERIFIED: crates/render_graph/src/fingerprint.rs; CITED: docs.rs/blake3] |
| Cross-table dependency cleanup | Manual cascades in Rust loops only | SQLite foreign keys plus explicit tests | SQLite requires `PRAGMA foreign_keys = ON` per connection; use it and test orphan rejection. [CITED: sqlite.org/foreignkeys.html] |
| Chunked job state | In-memory only job maps | SQLite job/chunk rows plus `CancelToken` | Resumability after crash requires persisted chunk state; cancellation can reuse existing runtime token semantics. [VERIFIED: crates/media_runtime/src/job.rs] |
| Quota/GC | Recursive delete by age | Mark-and-sweep from artifact rows with byte accounting | Untracked path deletion risks removing ready artifacts or user files if traversal/symlink mistakes exist. [ASSUMED] |

**Key insight:** Phase 14 is not a cache-key phase; Phase 13 already defined dirty and fingerprint facts. Phase 14 is a persistence, lifecycle, and safety phase for derived artifacts. [VERIFIED: .planning/phases/13-incremental-render-graph-dirty-ranges-and-cache-coherence/13-VERIFICATION.md]

## Common Pitfalls

### Pitfall 1: Renderer-Owned Artifact Semantics

**What goes wrong:** React/TypeScript starts choosing artifact paths, cache roots, dirty domains, graph fingerprints, or invalidation scope. [VERIFIED: current guards reject this for Phase 13; scripts/phase13-source-guards.sh]
**Why it happens:** Current preview commands accept `cacheRoot` from the renderer. [VERIFIED: apps/desktop-electron/src/renderer/commandHelpers.ts]
**How to avoid:** Replace user-visible preview commands with bundle/session-scoped Rust artifact APIs or make `cacheRoot` optional/deprecated and Rust-resolved. [ASSUMED]
**Warning signs:** New renderer code mentions `artifactStore`, `cacheKey`, `fingerprint`, `graphNode`, `dirtyRange`, or `.veproj/derived`. [VERIFIED: scripts/phase13-source-guards.sh]

### Pitfall 2: SQLite Migrations Without Connection PRAGMAs

**What goes wrong:** Foreign keys are declared but not enforced, or WAL checkpointing grows without bound. [CITED: sqlite.org/foreignkeys.html; CITED: sqlite.org/wal.html]
**Why it happens:** SQLite foreign keys must be enabled per connection, and WAL checkpoint policy has workload tradeoffs. [CITED: sqlite.org/foreignkeys.html; CITED: sqlite.org/wal.html]
**How to avoid:** Centralize connection opening and always set `PRAGMA foreign_keys=ON`, `PRAGMA journal_mode=WAL`, `PRAGMA busy_timeout`, and `PRAGMA user_version` checks before use. [ASSUMED]
**Warning signs:** Tests pass with orphan dependency rows or WAL files grow during repeated generation tests. [ASSUMED]

### Pitfall 3: Partial Blob And Row Mismatch

**What goes wrong:** A crash leaves a ready row pointing to a missing/partial blob, or an orphan blob never gets collected. [ASSUMED]
**Why it happens:** Blob writes and SQLite commits are separate filesystem operations. [ASSUMED]
**How to avoid:** Use temp files, content fingerprint verification, atomic rename, transaction commit, and startup repair that demotes missing blobs to dirty/failed and sweeps temp files. [VERIFIED: project_store atomic pattern; crates/project_store/src/lib.rs]
**Warning signs:** Ready artifacts with zero-byte files, rows with absolute paths, or blobs outside `.veproj/derived/blobs`. [ASSUMED]

### Pitfall 4: Source Fingerprint Weakness

**What goes wrong:** A replaced media file reuses stale thumbnails/proxies/waveforms because path, size, or mtime did not change enough. [ASSUMED]
**Why it happens:** Filesystems have timestamp granularity and users can overwrite in place. [ASSUMED]
**How to avoid:** Store `source_fingerprint_algorithm`, BLAKE3 content hash, byte length, mtime, and probe metadata; use mtime/size only as a fast path before content hash confirmation. [CITED: docs.rs/blake3]
**Warning signs:** Tests only touch mtimes rather than modifying bytes under the same material URI. [ASSUMED]

### Pitfall 5: Phase 16 Scheduler Leakage

**What goes wrong:** Phase 14 invents full priority queues/backpressure scheduling that conflicts with Phase 16. [VERIFIED: Phase 16 is later; .planning/ROADMAP.md]
**Why it happens:** ASSET-04 requires isolation and cancellation now, but full scheduler requirements are later. [VERIFIED: .planning/REQUIREMENTS.md]
**How to avoid:** Persist job/chunk/cancel state and use a minimal dedicated artifact worker trait; leave priority queues and telemetry budgets to Phase 16. [ASSUMED]
**Warning signs:** New `JobScheduler`, priority queues, starvation, or backpressure code appears in Phase 14 source guards. [VERIFIED: scripts/phase13-source-guards.sh showed these were later-scope terms]

## Code Examples

### Streaming Source Fingerprint

```rust
// Source: blake3 Hasher docs.
fn fingerprint_file(path: &std::path::Path) -> std::io::Result<String> {
    let file = std::fs::File::open(path)?;
    let mut hasher = blake3::Hasher::new();
    hasher.update_reader(file)?;
    Ok(format!("blake3:v1:{}", hasher.finalize().to_hex()))
}
```

### SQLite Transaction With Rollback-On-Drop

```rust
// Source: rusqlite Transaction docs.
fn mark_artifact_dirty(conn: &mut rusqlite::Connection, artifact_id: &str) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    tx.execute(
        "UPDATE artifact SET dirty = 1, status = 'dirty' WHERE artifact_id = ?1",
        [artifact_id],
    )?;
    tx.commit()
}
```

### Phase 14 Invalidation Flow

```rust
// Source: existing PreviewInvalidationRequest::from_command_delta pattern.
let invalidation = PreviewInvalidationRequest::from_command_delta(&timeline_response.delta);
artifact_store.mark_dirty_by_dependencies(
    &invalidation.changed_material_ids,
    &invalidation.changed_graph_node_keys,
    &invalidation.dirty_ranges,
    &invalidation.changed_domains,
)?;
```

## State Of The Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Renderer-provided `/tmp/video-editor-preview-cache` | Project-local `.veproj/derived` store resolved by Rust | Phase 14 target | Keeps artifacts portable with project bundles and out of renderer semantics. [VERIFIED: current renderer constant; ASSUMED target] |
| In-memory/path-only preview cache entries | SQLite artifact rows with dependencies and statuses | Phase 14 target | Enables exact invalidation, GC, resumability, and future server sync. [VERIFIED: current preview_service path-only cache; ASSUMED target] |
| FNV graph fingerprint helper for deterministic graph tests | BLAKE3 content/source/blob fingerprint for files | Phase 14 target | Reduces stale reuse risk for replaced media. [VERIFIED: crates/render_graph/src/fingerprint.rs; CITED: docs.rs/blake3] |
| Phase 13 dirty facts as transport data | Phase 14 dirty facts persisted as artifact dependency state | Phase 14 target | Converts validated dirty ranges into concrete derived artifact lifecycle. [VERIFIED: Phase 13 verification] |

**Deprecated/outdated:**
- Renderer `cacheRoot` as a required preview command field should be treated as transitional after Phase 14. [VERIFIED: apps/desktop-electron/src/renderer/commandHelpers.ts]
- Preview cache entries without graph/runtime/source fingerprint facts should be invalidated when v2/v3 artifact-store facts are present. [VERIFIED: crates/preview_service/src/cache.rs]

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Use one new `artifact_store` crate with internal resource/generation/invalidation modules; defer a separate `asset_resource_manager` crate unless a later phase proves the boundary is needed. | Summary / Architecture Patterns / Resolved Questions | Planner and executor should not create a second crate in Phase 14 unless the plan is explicitly revised. |
| A2 | Use BLAKE3 rather than SHA-256 for source/blob fingerprints. | Standard Stack | Remote/server sync might require SHA-256 for third-party interoperability. |
| A3 | Use `fs2` as optional advisory lock for GC/compaction. | Standard Stack | If multi-process project opening is out of scope, this dependency may be unnecessary. |
| A4 | Implement actual Rust-owned proxy, thumbnail, and waveform generation facades/workers in Phase 14, while deferring full priority scheduling/backpressure to Phase 16. | Common Pitfalls / Plan checker resolution | If Phase 14 only persists job rows, ASSET-04 is not actually satisfied. |
| A5 | Define sync manifests in SQLite/JSON without implementing upload/download. | Phase Requirements / Architecture Patterns | Later server rendering may need a richer manifest contract. |

## Open Questions (RESOLVED)

1. **Should the artifact store be one new crate or split into `artifact_store` plus `asset_resource_manager`?**
   - What we know: the current workspace has clear service crates, and no artifact-store crate exists. [VERIFIED: Cargo.toml]
   - RESOLVED: Phase 14 starts with one `artifact_store` crate and modules for schema, paths, blob store, resource index, dependencies, invalidation, generation, jobs, GC, quota, and manifest. A separate `asset_resource_manager` crate is deferred unless a later phase proves a clear acyclic boundary is needed. [VERIFIED: 14-CONTEXT.md D-01..D-10; 14-PATTERNS.md]

2. **Should source fingerprints hash full media bytes immediately or use lazy/fast-path hashing?**
   - What we know: BLAKE3 supports streaming readers and file-oriented APIs. [CITED: docs.rs/blake3]
   - RESOLVED: Artifact validity depends on full BLAKE3 source/blob fingerprints when generation touches the source or writes a blob. Size/mtime may be stored as diagnostic or fast pre-check metadata, but must not be the sole validity proof for reusing proxy, thumbnail, waveform, preview, graph, or sync-manifest artifacts. [CITED: docs.rs/blake3; VERIFIED: 14-CONTEXT.md D-03, D-08]

3. **How much UI should Phase 14 expose?**
   - What we know: Phase 14 has `UI hint: yes`, and the current UI already has material/preview status surfaces. [VERIFIED: .planning/ROADMAP.md; apps/desktop-electron/src/renderer/App.tsx]
   - RESOLVED: Phase 14 exposes only the production resource UI defined in `14-UI-SPEC.md`: compact material resource status chips, a `资源任务` strip, preview/timeline safe status, and one quiet `资源维护` section with quota/GC summary and `清理缓存`. Debug-heavy panels, SQLite paths, absolute cache roots, raw fingerprints, graph keys, dirty ranges, FFmpeg/probe internals, blob folders, tombstones, and advanced cache/file browsing UI are out of scope. [VERIFIED: 14-UI-SPEC.md; 14-CONTEXT.md D-09, D-10]

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| Rust/Cargo | New Rust crates and tests | yes | cargo 1.95.0 / rustc 1.95.0 | none |
| Node | Contract generation and package scripts | yes | v24.12.0 | none |
| pnpm | Desktop/package scripts | yes | 10.32.1 | none |
| SQLite CLI | Manual DB inspection/debug | yes | 3.51.0 | Use `rusqlite` tests |
| FFmpeg/ffprobe | Thumbnail/proxy/waveform generation and material probing | yes | 8.1 | Mock generator traits for unit tests |
| `ctx7` | Documentation lookup | no | - | Used docs.rs / sqlite.org |
| `slopcheck` | Package legitimacy gate | yes | 0.6.1 | none |

**Missing dependencies with no fallback:** none.

**Missing dependencies with fallback:**
- `ctx7` is unavailable; official docs.rs and sqlite.org pages were used instead. [VERIFIED: shell probe]

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust `cargo test`, Node/pnpm scripts, Playwright Electron tests. [VERIFIED: package.json] |
| Config file | Root `Cargo.toml`, root `package.json`, `apps/desktop-electron/playwright.config.ts`. [VERIFIED: codebase grep] |
| Quick run command | `cargo test -p artifact_store -- --nocapture` after Wave 0 creates the crate. [ASSUMED] |
| Full suite command | `pnpm run test:phase14 && pnpm run test:contracts`. [ASSUMED] |

### Phase Requirements -> Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| ASSET-01 | Material/font/effect/proxy/thumb/waveform resources get stable IDs and project-relative refs | Rust unit/integration | `cargo test -p artifact_store resource_index -- --nocapture` | No, Wave 0 |
| ASSET-02 | SQLite schema tracks versions, fingerprints, dependencies, dirty state, status | Rust integration | `cargo test -p artifact_store sqlite_schema -- --nocapture` | No, Wave 0 |
| ASSET-03 | Replace/relink/rename/delete invalidates exactly affected artifact rows | Rust integration | `cargo test -p artifact_store invalidation -- --nocapture` | No, Wave 0 |
| ASSET-04 | Proxy, thumbnail, and waveform generation is chunked, resumable, cancellable, isolated, and produces BlobStore-backed derived artifacts | Rust integration | `cargo test -p artifact_store artifact_jobs -- --nocapture && cargo test -p artifact_store artifact_generation -- --nocapture` | No, Wave 0 |
| ASSET-05 | GC/quota/sync manifest definitions preserve live artifacts and delete only safe blobs | Rust integration | `cargo test -p artifact_store gc_quota_manifest -- --nocapture` | No, Wave 0 |

### Sampling Rate

- **Per task commit:** focused crate test plus `pnpm run test:contracts` when generated contracts change. [VERIFIED: existing contract workflow in package.json]
- **Per wave merge:** `pnpm run test:phase14`. [ASSUMED]
- **Phase gate:** `pnpm run test:phase14 && pnpm run test:contracts && pnpm run test:phase13`. [ASSUMED]

### Wave 0 Gaps

- [ ] `crates/artifact_store/` - SQLite schema, connection PRAGMAs, blob paths, source/blob fingerprint helpers, resource indexing, invalidation, job state, actual proxy/thumbnail/waveform generation facades, GC/quota/manifest modules.
- [ ] `scripts/phase14-source-guards.sh` - reject renderer-owned artifact store, fingerprints, cache keys, `.veproj/derived`, SQLite, FFmpeg args, and generated drift.
- [ ] `package.json` scripts `test:phase14-rust`, `test:phase14-source-guards`, `test:phase14`.
- [ ] Generated contract tests if new binding-visible artifact status/maintenance commands are added.

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | no | Local desktop project files only in Phase 14. [VERIFIED: project scope] |
| V3 Session Management | partial | Future artifact handles should include owner session/generation in Phase 17; Phase 14 can expose IDs/status only. [VERIFIED: BIND-02 later in .planning/REQUIREMENTS.md] |
| V4 Access Control | yes | Restrict artifact paths to `.veproj/derived`; reject traversal and absolute derived blob paths. [VERIFIED: project_store URI traversal pattern] |
| V5 Input Validation | yes | `serde(deny_unknown_fields)`, generated schemas, SQLite constraints, path validation, bounded JSON parameters. [VERIFIED: draft_model patterns] |
| V6 Cryptography | yes | Use BLAKE3 via `blake3`, never custom file hashing for source/blob fingerprints. [CITED: docs.rs/blake3] |
| V8 Data Protection | yes | Avoid leaking absolute external paths into sync manifests when project-relative paths are sufficient. [ASSUMED] |
| V12 File and Resources | yes | Atomic writes, path containment checks, symlink-aware deletion, quotas, and temp cleanup. [VERIFIED: project_store atomic-write pattern; ASSUMED expanded controls] |

### Known Threat Patterns For Phase 14

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Path traversal into or out of `.veproj/derived` | Tampering / Information Disclosure | Canonicalize root, validate relative paths, reject `..`, absolute paths, and symlink escape before write/delete. [VERIFIED: crates/project_store/src/paths.rs] |
| Stale artifact reuse after source replacement | Tampering | Store BLAKE3 source fingerprint, byte length, mtime, probe facts, and dependency rows; invalidate by material ID and fingerprint mismatch. [CITED: docs.rs/blake3; VERIFIED: Phase 13 dirty facts] |
| SQLite orphan rows or disabled constraints | Tampering | Enable `PRAGMA foreign_keys=ON` for every connection and test cascade/orphan behavior. [CITED: sqlite.org/foreignkeys.html] |
| WAL/checkpoint growth | DoS | Configure WAL intentionally, monitor checkpoint behavior, and add maintenance tests. [CITED: sqlite.org/wal.html] |
| Partial blob writes after crash/cancel | Tampering / DoS | Temp files, fsync, atomic rename, verified blob fingerprint, startup repair, and cancellable job states. [VERIFIED: project_store atomic-write pattern; VERIFIED: media_runtime CancelToken] |
| GC deleting live artifacts | Tampering / DoS | Mark-and-sweep using DB references, tombstones, and dry-run tests before deletion. [ASSUMED] |
| Renderer inventing artifact semantics | Tampering | Source guards and generated contracts; renderer displays status only. [VERIFIED: scripts/phase13-source-guards.sh] |

## Sources

### Primary (HIGH confidence)

- `AGENTS.md` - project constraints, architecture, terminology, time model, testing, licensing.
- `.planning/PROJECT.md`, `.planning/STATE.md`, `.planning/ROADMAP.md`, `.planning/REQUIREMENTS.md` - Phase 14 scope and later consumers.
- `.planning/phases/13-incremental-render-graph-dirty-ranges-and-cache-coherence/13-DESIGN.md`, `13-VERIFICATION.md`, `13-SECURITY.md` - upstream dirty-range and graph fingerprint contracts.
- `crates/project_store`, `crates/draft_model`, `crates/preview_service`, `crates/render_graph`, `crates/media_runtime`, `crates/bindings_node`, `apps/desktop-electron` - current implementation boundaries.
- https://docs.rs/rusqlite/latest/rusqlite/ - Rust SQLite wrapper API.
- https://docs.rs/blake3/latest/blake3/ - BLAKE3 Rust API.
- https://sqlite.org/foreignkeys.html - SQLite foreign key behavior.
- https://sqlite.org/wal.html - SQLite WAL and checkpoint behavior.
- https://sqlite.org/lang_transaction.html - SQLite transaction behavior.

### Secondary (MEDIUM confidence)

- https://docs.rs/fs2/latest/fs2/ - advisory file locking and filesystem utilities.
- crates.io API and `cargo info` - package versions, repository URLs, publish dates, and download counts.

### Tertiary (LOW confidence)

- None used for recommendations.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - current package versions and docs were verified through crates.io/docs.rs and slopcheck.
- Architecture: HIGH for boundaries - codebase and Phase 13 artifacts define the ownership model; MEDIUM for exact crate split because no Phase 14 implementation exists yet.
- Pitfalls: HIGH for renderer/draft leakage and SQLite PRAGMA risks; MEDIUM for sync manifest/GC details because remote rendering is later scope.

**Research date:** 2026-06-19
**Valid until:** 2026-07-19 for crate/schema choices; revisit earlier if Phase 16 scheduler or Phase 17 binding work changes artifact handle requirements.
