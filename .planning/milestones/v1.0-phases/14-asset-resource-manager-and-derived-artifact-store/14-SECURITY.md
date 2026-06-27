---
phase: 14
slug: asset-resource-manager-and-derived-artifact-store
status: verified
threats_open: 0
asvs_level: 1
created: 2026-06-19
---

# Phase 14 - Security

Per-phase security contract for the asset/resource manager and derived
artifact store.

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| Project bundle path -> SQLite/blob filesystem | User-controlled project paths cross into `.veproj/derived` file creation and deletion. | Bundle paths, derived relative blob paths, SQLite file path |
| SQLite metadata -> blob filesystem | DB rows decide which blobs are live, dirty, repairable, collectible, and manifest-visible. | Artifact rows, dependencies, tombstones, generation jobs |
| Generation worker -> BlobStore | Generated proxy, thumbnail, and waveform bytes cross into project-contained derived storage. | Bytes, MIME type, fingerprints, chunk/job state |
| Renderer command payload -> Rust artifact service | UI payloads request status, generation actions, quota, and GC, but Rust owns decisions. | Generated command envelopes and safe result summaries |
| Rust status -> default production UI | Resource state becomes visible to users and must not expose internals. | Safe labels, progress, counts, action flags |

## Threat Register

| Threat ID | Category | Component | Disposition | Mitigation | Status |
|-----------|----------|-----------|-------------|------------|--------|
| T-14-01 | Tampering / Information Disclosure | `paths.rs`, `BlobStore` | mitigate | `validate_derived_relative_path` rejects empty, absolute, Windows absolute, parent/current traversal, non-UTF8 display paths, and existing symlink escape. Blob operations validate derived-relative paths before reads/writes/deletes; tests cover invalid paths and symlink escape. | closed |
| T-14-02 | Tampering | SQLite migrations | mitigate | `open_artifact_store` applies foreign keys, WAL, and busy timeout, then sets schema version. Schema tests verify pragmas, idempotent migration, and orphan dependency/chunk rejection. | closed |
| T-14-03 | Tampering / DoS | Atomic blob writes | mitigate | `BlobStore::write_blob_atomic` fingerprints bytes, writes temp file, syncs, renames atomically, syncs parent when possible, verifies fingerprint/byte count, and only then commits ready rows. Repair demotes missing/empty ready blobs and clears temp files. | closed |
| T-14-04 | Information Disclosure | Default desktop UI/renderer | mitigate | `scripts/phase14-source-guards.sh` rejects renderer artifact roots, SQLite internals, fingerprints, dirty ranges, graph/cache internals, and FFmpeg command construction; workspace tests assert forbidden production copy is hidden. | closed |
| T-14-05 | Tampering | `resource_index.rs` | mitigate | Resource IDs and stable keys are derived in Rust from semantic material/text/effect references; renderer helpers only send generated command envelopes. | closed |
| T-14-06 | Tampering | `dependencies.rs` | mitigate | Dependencies use typed kinds, checked integer microsecond ranges, foreign keys, and transaction-scoped replacement. Tests reject overflow and assert no partial dependency insert. | closed |
| T-14-07 | Information Disclosure | External resource refs | mitigate | Resource indexing records `project_relative_ref` only for in-bundle refs; external absolute paths/URIs are not surfaced as project-relative refs. UI/source guards block default exposure of absolute refs and source fingerprints. | closed |
| T-14-08 | Tampering | Source-change invalidation | mitigate | `mark_dirty_for_source_change` matches material/resource/source-fingerprint dependency rows; invalidation tests prove unrelated artifacts remain ready. | closed |
| T-14-09 | DoS | Dirty range overlap/merge | mitigate | Dirty range normalization uses checked integer microseconds and records full-draft fallback on overflow. Tests cover overflow fallback. | closed |
| T-14-10 | Repudiation | Invalidation outcomes | mitigate | Dirty updates persist `dirty_reason`, `dirty_source_change_kind`, and fallback reason while result rows expose only safe status/reason summaries. | closed |
| T-14-11 | Tampering | Job/chunk status transitions | mitigate | Job transitions guard terminal states; stale late failures cannot overwrite completed jobs. Tests cover terminal preservation and restart behavior. | closed |
| T-14-12 | DoS | Long-running generation | mitigate | Generation jobs persist cancel requests, acknowledgements, resumable state, and restart state. Scheduler/backpressure policy remains out of Phase 14 and is source-guarded for Phase 16. | closed |
| T-14-13 | Tampering / Information Disclosure | Generation to BlobStore | mitigate | Generated artifacts pass cancellation checks, non-empty validation, MIME validation, BlobStore fingerprint verification, and dependency persistence for material/resource/source refs. Failure paths persist failed/cancelled terminal state. | closed |
| T-14-14 | Information Disclosure / Tampering / DoS | Status summaries and GC | mitigate | Binding summaries expose display labels/progress/action flags only. GC plans from DB rows, preserves live artifacts and active jobs/chunks, validates blob paths under `blobs/`, excludes temp paths, and tombstones deleted rows. | closed |
| T-14-15 | Information Disclosure | Quota state | mitigate | `compute_quota_state` returns aggregate counts/byte labels/severity only, with no absolute paths, fingerprints, graph keys, dirty ranges, or SQLite internals. | closed |
| T-14-16 | Tampering / Repudiation | Sync manifest | mitigate | Manifest generation is deterministically ordered, versioned, path-validated, fingerprinted, includes tombstones, and has no remote side effects. | closed |
| T-14-17 | Tampering | Generated command contracts | mitigate | Rust schema export builds root command/payload pairing constraints. Tests assert every `CommandName` appears exactly once and artifact commands have the expected payload shape. | closed |
| T-14-18 | Elevation of Privilege / Tampering | `artifact_store_service.rs` | mitigate | Bindings validate session IDs, `.veproj` bundle path shape, non-empty job IDs, and delegate store/GC/retry decisions to Rust services without SQL or cache-key computation in renderer code. | closed |
| T-14-19 | Information Disclosure | Status responses | mitigate | Artifact status/quota/maintenance responses contain safe labels, severities, counts, and action flags only. Contract tests reject internal fields such as fingerprints, SQLite, dirty ranges, and FFmpeg args. | closed |
| T-14-20 | Tampering | Preview artifact root | mitigate | Preview artifact roots resolve in Rust from the project bundle. Renderer-owned cache root remains optional/deprecated and is blocked from default production UI/source ownership. | closed |
| T-14-21 | Tampering | Renderer command helpers | mitigate | Renderer helpers build generated command envelopes only. Source guards reject artifact-root/key/fingerprint/dirty/GC/SQLite/FFmpeg decisions outside approved helper/view-model surfaces. | closed |
| T-14-22 | Information Disclosure | Resource status UI | mitigate | UI tests and source guards block absolute paths, SQLite names, raw fingerprints, graph keys, dirty ranges, logs, and FFmpeg internals in production-facing resource UI. | closed |
| T-14-23 | Tampering / DoS | Cache cleanup UI | mitigate | Cleanup UI requires confirmation copy and sends only generated GC commands; Rust computes candidates and returns safe labels/results. | closed |
| T-14-24 | Repudiation | Final gates | mitigate | `test:phase14` wires Rust behavior tests, source guards, workspace UI coverage, and contract drift checks; Phase 13 regression gate was also kept green during verification. | closed |
| T-14-SC | Tampering | Package installs | accept | Plan 14-01 used only research-approved crates `rusqlite`, `blake3`, and `fs2`; later plans installed no new external packages. | closed |

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-14-SC | T-14-SC | Supply-chain risk is bounded to research-approved crates in Plan 14-01; later Phase 14 plans added no external packages. | Phase 14 plan threat model | 2026-06-19 |

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-06-19 | 25 | 25 | 0 | Codex inline secure-phase audit |

## Evidence Summary

- Path and blob safety: `crates/artifact_store/src/paths.rs`, `crates/artifact_store/src/blob_store.rs`, and `crates/artifact_store/tests/blob_store.rs`.
- SQLite/schema safety: `crates/artifact_store/src/schema.rs` and `crates/artifact_store/tests/sqlite_schema.rs`.
- Resource/dependency/invalidation safety: `crates/artifact_store/src/resource_index.rs`, `crates/artifact_store/src/dependencies.rs`, `crates/artifact_store/src/invalidation.rs`, and related tests.
- Generation lifecycle safety: `crates/artifact_store/src/jobs.rs`, `crates/artifact_store/src/generation.rs`, `crates/artifact_store/tests/artifact_jobs.rs`, and `crates/artifact_store/tests/artifact_generation.rs`.
- GC/quota/manifest safety: `crates/artifact_store/src/gc.rs`, `crates/artifact_store/src/quota.rs`, `crates/artifact_store/src/manifest.rs`, and `crates/artifact_store/tests/gc_quota_manifest.rs`.
- Binding and renderer boundaries: `crates/bindings_node/src/artifact_store_service.rs`, `crates/bindings_node/tests/artifact_store_commands.rs`, `apps/desktop-electron/src/renderer/commandHelpers.ts`, `apps/desktop-electron/tests/workspace.spec.ts`, and `scripts/phase14-source-guards.sh`.
- Contract drift and command pairing: `crates/draft_model/tests/schema_exports.rs` and `schemas/command.schema.json`.

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-06-19
