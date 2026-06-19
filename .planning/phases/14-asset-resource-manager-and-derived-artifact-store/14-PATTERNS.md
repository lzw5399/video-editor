# Phase 14: asset-resource-manager-and-derived-artifact-store - Pattern Map

**Mapped:** 2026-06-19
**Files analyzed:** 18 file/module areas
**Analogs found:** 17 / 18

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/artifact_store/Cargo.toml` | config | dependency wiring | `crates/project_store/Cargo.toml` | role-match |
| `Cargo.toml` | config | workspace membership | root `Cargo.toml` | exact |
| `crates/artifact_store/src/lib.rs` | service | CRUD + file-I/O | `crates/project_store/src/bundle.rs` + `crates/preview_service/src/cache.rs` | role-match |
| `crates/artifact_store/src/paths.rs` | utility | file-I/O | `crates/project_store/src/paths.rs` | exact |
| `crates/artifact_store/src/blob_store.rs` | service | file-I/O | `crates/project_store/src/bundle.rs` | partial |
| `crates/artifact_store/src/fingerprint.rs` | utility | streaming/file-I/O | `crates/render_graph/src/fingerprint.rs` | role-match |
| `crates/artifact_store/src/schema.rs` or `migrations.rs` | migration | CRUD | no local SQLite analog | none |
| `crates/artifact_store/src/resource_index.rs` | service | CRUD + transform | `crates/project_store/src/bundle.rs` + `crates/preview_service/src/cache.rs` | role-match |
| `crates/artifact_store/src/invalidation.rs` | service | event-driven + CRUD | `crates/preview_service/src/cache.rs` | exact |
| `crates/artifact_store/src/jobs.rs` | service | event-driven + batch | `crates/media_runtime/src/job.rs` | role-match |
| `crates/artifact_store/src/gc.rs` | service | batch + file-I/O | `crates/project_store/src/paths.rs` | partial |
| `crates/artifact_store/src/manifest.rs` | service | transform | `crates/render_graph/src/fingerprint.rs` | role-match |
| `crates/artifact_store/tests/*.rs` | test | CRUD + file-I/O + event-driven | `crates/preview_service/tests/cache_invalidation.rs`, `crates/media_runtime/tests/export_job.rs` | role-match |
| `crates/project_store/src/paths.rs` | utility | file-I/O | existing same file | exact |
| `crates/preview_service/src/service.rs` | service | request-response + file-I/O | existing same file | exact |
| `crates/draft_model/src/*.rs` command/result contracts | model | request-response | `crates/draft_model/src/delta.rs` + `tests/schema_exports.rs` | exact |
| `crates/bindings_node/src/*artifact*.rs` | provider/service | request-response | `crates/bindings_node/src/realtime_preview_service.rs` | role-match |
| `scripts/phase14-source-guards.sh` and `package.json` scripts | config/test | batch | `scripts/phase13-source-guards.sh` + `package.json` | exact |
| `apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx` + CSS/view model | component | request-response presentation | `FeaturePanel.tsx`, `WorkspaceShell.tsx`, `styles.css` | exact |

## Pattern Assignments

### `crates/artifact_store/Cargo.toml` and Root `Cargo.toml` (config, dependency wiring)

**Analog:** `Cargo.toml` and `crates/project_store/Cargo.toml`

**Workspace member pattern** (`Cargo.toml` lines 1-16):
```toml
[workspace]
members = [
  "crates/draft_model",
  "crates/draft_commands",
  ...
  "crates/project_store",
  "crates/preview_service",
  "crates/testkit",
  "crates/bindings_node",
]
resolver = "3"
```

**Crate manifest pattern** (`crates/project_store/Cargo.toml` lines 1-16):
```toml
[package]
name = "project_store"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish = false

[lib]
path = "src/lib.rs"

[dependencies]
draft_model = { path = "../draft_model" }
serde_json = "1.0.150"
thiserror = "2.0.18"
```

**Apply:** Add `crates/artifact_store` to the workspace members. Mirror package metadata and `publish = false`. Add `rusqlite`, `blake3`, optional `fs2`, `serde`, `serde_json`, `thiserror`, and path deps needed for `draft_model`, `render_graph`, and `project_store` facts.

---

### `crates/artifact_store/src/paths.rs` (utility, file-I/O)

**Analog:** `crates/project_store/src/paths.rs`

**Imports and constants pattern** (lines 1-6):
```rust
use std::path::{Component, Path, PathBuf};

use crate::ProjectStoreError;

pub const PROJECT_JSON_FILE_NAME: &str = "project.json";
```

**Project-contained relative path validation** (lines 88-108):
```rust
fn validate_bundle_relative_path(uri: &str) -> Result<(), ProjectStoreError> {
    let path = Path::new(uri);
    if path.components().next().is_none() {
        return invalid_uri(uri, "relative URI must contain a path");
    }

    for component in path.components() {
        match component {
            Component::Normal(_) => {}
            Component::CurDir => {}
            Component::ParentDir => {
                return invalid_uri(uri, "parent directory traversal is not allowed");
            }
            Component::RootDir | Component::Prefix(_) => {
                return invalid_uri(uri, "absolute paths are not bundle-relative URIs");
            }
        }
    }

    Ok(())
}
```

**Path-to-display URI pattern** (lines 137-145):
```rust
fn path_to_uri(path: &Path) -> Result<String, ProjectStoreError> {
    let value = path
        .to_str()
        .ok_or_else(|| ProjectStoreError::InvalidMaterialUri {
            uri: path.to_string_lossy().into_owned(),
            reason: "path must be valid UTF-8".to_owned(),
        })?
        .replace('\\', "/");
    Ok(value)
}
```

**Apply:** Add `derived_root_path(bundle_path) -> bundle/.veproj?` equivalent only if the bundle path itself is `.veproj`; for this codebase the bundle path is already `*.veproj`, so helpers should resolve `bundle_path.join("derived")`, `artifact-store.sqlite`, `blobs`, `tmp`, and displayable paths relative to `derived`. Reuse traversal rejection logic and return typed artifact-store errors.

---

### `crates/artifact_store/src/blob_store.rs` (service, file-I/O)

**Analog:** `crates/project_store/src/bundle.rs`

**Validate before write pattern** (lines 30-50):
```rust
pub fn save_project_bundle(
    fs: &impl PlatformFileSystem,
    bundle_path: impl AsRef<Path>,
    draft: &Draft,
) -> Result<ProjectBundle, ProjectStoreError> {
    let bundle_path = bundle_path.as_ref();
    let project_json_path = project_json_path(bundle_path);

    validate_draft(draft).map_err(|source| semantic_error(&project_json_path, source))?;
    validate_material_uris(bundle_path, draft)?;
    let contents = serde_json::to_string_pretty(draft).map_err(|error| {
        ProjectStoreError::InvalidProjectJson {
            path: project_json_path.clone(),
            message: error.to_string(),
        }
    })?;
    fs.write_string(&project_json_path, &format!("{contents}\n"))
        .map_err(|source| ProjectStoreError::Io {
            path: project_json_path.clone(),
            source,
        })?;
```

**Apply:** Blob writes should validate artifact row intent and project-contained destination before any file write. For Phase 14, strengthen this pattern with temp file under `.veproj/derived/blobs/tmp`, file sync, atomic rename, BLAKE3 verification, and SQLite update in a transaction.

**No exact local atomic blob analog:** `project_store` abstracts writes through `PlatformFileSystem`; no current local helper shows temp-write-plus-rename-plus-fsync. Planner should specify this explicitly rather than pretending a repo pattern exists.

---

### `crates/artifact_store/src/fingerprint.rs` and `manifest.rs` (utility/service, streaming + transform)

**Analog:** `crates/render_graph/src/fingerprint.rs`

**Versioned fingerprint contract pattern** (lines 12-25):
```rust
pub const GRAPH_SCHEMA_VERSION: u32 = 1;
pub const GRAPH_GENERATOR_VERSION: &str = "render-graph-generator-v1";

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

**Deterministic ordering before snapshot output** (lines 54-70):
```rust
let mut node_fingerprints = node_fingerprints(
    graph,
    &output_profile_fingerprint,
    &runtime_capability_fingerprint,
);
node_fingerprints.sort_by_key(|fingerprint| fingerprint.node_id.stable_key());

Self {
    draft_id: graph.draft_id.clone(),
    target_timerange: graph.target_timerange.clone(),
    frame_rate: graph.frame_rate.clone(),
    graph_schema_version: GRAPH_SCHEMA_VERSION,
    generator_version: GRAPH_GENERATOR_VERSION.to_owned(),
    output_profile_fingerprint,
    runtime_capability_fingerprint,
    node_fingerprints,
}
```

**Apply:** Artifact fingerprints and sync manifests should include explicit algorithm/schema/generator versions and deterministic ordering. Use BLAKE3 for file/blob/source hashes, but keep the same style: typed structs, `serde(rename_all = "camelCase", deny_unknown_fields)`, stable sorting before fingerprint/manifest serialization.

---

### `crates/artifact_store/src/resource_index.rs` (service, CRUD + transform)

**Analog:** `crates/project_store/src/bundle.rs` and `crates/project_store/src/paths.rs`

**Material URI classification and warning collection pattern** (`bundle.rs` lines 106-127):
```rust
fn collect_warnings(
    fs: &impl PlatformFileSystem,
    bundle_path: &Path,
    draft: &Draft,
) -> Result<Vec<ProjectStoreWarning>, ProjectStoreError> {
    let mut warnings = Vec::new();

    for material in &draft.materials {
        let classified = classify_material_uri(bundle_path, &material.uri)?;
        if let Some(resolved_path) = classified.resolved_path {
            if !fs.exists(&resolved_path) {
                warnings.push(ProjectStoreWarning::MissingMaterial {
                    material_id: material.material_id.as_str().to_owned(),
                    uri: material.uri.clone(),
                    resolved_path: Some(resolved_path),
                });
            }
        }
    }

    Ok(warnings)
}
```

**Apply:** Index material resources from existing `Draft.materials`, text fonts, filters, transitions, and generated artifact roles. Do not add cache-only font/effect variants to canonical `Material`; persist resource rows under artifact store tables keyed by stable semantic IDs and project-relative/source refs.

---

### `crates/artifact_store/src/invalidation.rs` (service, event-driven + CRUD)

**Analog:** `crates/preview_service/src/cache.rs`

**Dirty fact request pattern** (lines 162-173):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewInvalidationRequest {
    pub dirty_ranges: Vec<DirtyRange>,
    pub changed_material_ids: Vec<MaterialId>,
    pub changed_graph_node_keys: Vec<String>,
    pub changed_domains: Vec<DirtyDomain>,
    pub runtime_capability_fingerprint: Option<String>,
    pub output_profile_fingerprint: Option<String>,
    pub full_draft: bool,
    pub reason: String,
}
```

**Command delta conversion pattern** (lines 197-233):
```rust
pub fn from_command_delta(delta: &CommandDelta) -> Self {
    let changed_domains = if delta.invalidation.consumer_domains.is_empty() {
        consumer_domains_for_dirty_domains(delta.changed_domains.iter().copied())
    } else {
        consumer_domains_for_dirty_domains(
            delta
                .changed_domains
                .iter()
                .chain(delta.invalidation.consumer_domains.iter())
                .copied(),
        )
    };
    let mut material_ids = delta.invalidation.material_ids.clone();
    material_ids.extend(
        delta
            .changed_entities
            .iter()
            .filter_map(|entity| match entity {
                draft_model::ChangedEntity::Material { material_id } => {
                    Some(material_id.clone())
                }
                _ => None,
            }),
    );

    let mut request = Self {
        dirty_ranges: delta.changed_ranges.clone(),
        changed_material_ids: material_ids,
        changed_graph_node_keys: delta.invalidation.graph_node_ids.clone(),
        changed_domains,
        runtime_capability_fingerprint: None,
        output_profile_fingerprint: None,
        full_draft: delta.invalidation.full_draft,
        reason: delta.reason.clone(),
    };
    request.normalize();
    request
}
```

**Exact invalidation matching pattern** (lines 493-549):
```rust
pub fn invalidate_preview_cache(
    entries: &[PreviewCacheEntry],
    request: &PreviewInvalidationRequest,
) -> PreviewInvalidationResult {
    let mut retained = Vec::new();
    let mut invalidated = Vec::new();

    for entry in entries {
        if should_invalidate(entry, request) {
            invalidated.push(entry.clone());
        } else {
            retained.push(entry.clone());
        }
    }

    PreviewInvalidationResult {
        retained,
        invalidated,
    }
}
```

**Overflow fallback pattern** (lines 589-615):
```rust
fn merge_dirty_ranges(mut ranges: Vec<DirtyRange>) -> Option<Vec<DirtyRange>> {
    ranges.sort_by_key(|range| {
        (
            range.target_timerange.start,
            range.target_timerange.duration,
            range.source as u8,
        )
    });

    let mut merged: Vec<DirtyRange> = Vec::new();
    for range in ranges {
        range.target_timerange.checked_end()?;
        ...
    }

    Some(merged)
}
```

**Apply:** Persist dependency rows and mark artifact rows dirty by the same dimensions: material IDs, graph node stable keys, dirty domains, integer ranges, runtime capability fingerprint, and output profile fingerprint. Full-draft invalidation should be only for explicit `full_draft`, overflow, or unknown dependency cases.

---

### `crates/artifact_store/src/jobs.rs` (service, event-driven + batch)

**Analog:** `crates/media_runtime/src/job.rs`

**Cancelable job state and progress pattern** (lines 64-81):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FfmpegJobState {
    Started,
    Running,
    Completed,
    Cancelled,
    TimedOut,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FfmpegProgress {
    pub out_time_microseconds: u64,
    pub expected_duration_microseconds: Option<u64>,
    pub progress_per_mille: Option<u16>,
}
```

**Cancel token pattern** (lines 179-196):
```rust
#[derive(Debug, Clone, Default)]
pub struct CancelToken {
    cancelled: Arc<AtomicBool>,
}

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}
```

**Apply:** Artifact generation jobs should persist job/chunk rows, but cancellation semantics can mirror `CancelToken`. Use `progress_per_mille: Option<u16>` and integer microseconds where duration applies. Avoid Phase 16 scheduler terms like priority/backpressure beyond a scheduler-compatible contract.

---

### `crates/artifact_store/src/schema.rs` or `migrations.rs` (migration, CRUD)

**Analog:** No local SQLite analog.

**Closest local patterns:** crate manifest layout from `project_store`; typed serialization/versioning from `render_graph::fingerprint`; tests from `preview_service/tests/cache_invalidation.rs`.

**Planner instruction:** Create a local SQLite boundary with one connection-opening function that always applies PRAGMAs and schema version checks. Required Phase 14 tables should cover resources, artifacts, artifact dependencies, generation jobs, generation chunks, quota/GC metadata, tombstones, and sync manifest rows. Because no local DB pattern exists, require explicit tests for:

```rust
// Shape only; no local excerpt exists.
conn.pragma_update(None, "foreign_keys", "ON")?;
conn.pragma_update(None, "journal_mode", "WAL")?;
conn.pragma_update(None, "busy_timeout", 5_000)?;
```

**No Analog Found reason:** The codebase currently has no `rusqlite`, migration framework, SQL schema file, or SQLite connection wrapper.

---

### `crates/artifact_store/tests/*.rs` (test, CRUD + file-I/O + event-driven)

**Analog:** `crates/preview_service/tests/cache_invalidation.rs` and `crates/media_runtime/tests/export_job.rs`

**Snapshot/serialization test style** (`cache_invalidation.rs` lines 16-42):
```rust
#[test]
fn cache_entry_snapshot_includes_range_profile_fingerprint_materials_and_artifact() {
    let entry = entry(
        "frame",
        0,
        100_000,
        &["video-material"],
        PreviewCacheProfile::FramePng,
    );

    assert_eq!(
        serde_json::to_value(&entry).expect("entry should serialize"),
        serde_json::json!({
            "key": {
                "keyId": "frame",
                "profile": "framePng",
                "targetTimerange": { "start": 0, "duration": 100000 },
                "semanticFingerprint": "fingerprint-frame",
                "materialDependencies": ["video-material"]
            },
            "artifact": {
                "profile": "framePng",
                "path": "/cache/frame.png",
                "mimeType": "image/png"
            }
        })
    );
}
```

**Cancellation test style** (`export_job.rs` lines 30-68):
```rust
#[test]
fn export_job_cancel_returns_cancelled_state_and_bounded_logs() {
    let sandbox = Sandbox::new("cancel");
    ...
    let cancel = CancelToken::new();
    let cancel_clone = cancel.clone();

    thread::spawn(move || {
        ...
        cancel_clone.cancel();
    });

    let mut events = Vec::new();
    let result = run_export_job(&job, &cancel, |event| events.push(event))
        .expect("cancel should produce a classified job result");

    assert_eq!(result.state, FfmpegJobState::Cancelled);
    assert!(events.iter().any(|event| matches!(event, FfmpegJobEvent::Progress { .. })));
}
```

**Apply:** Add focused tests named by validation: `sqlite_schema`, `blob_store`, `resource_index`, `invalidation`, `artifact_jobs`, `gc_quota_manifest`. Prefer `tempfile` for filesystem isolation; use JSON equality for deterministic manifest/status contracts.

---

### `crates/project_store/src/paths.rs` (modified utility, file-I/O)

**Analog:** existing same file.

**Existing public helper style** (lines 21-23, 59-64):
```rust
pub fn project_json_path(bundle_path: impl AsRef<Path>) -> PathBuf {
    bundle_path.as_ref().join(PROJECT_JSON_FILE_NAME)
}

pub fn resolve_material_uri(
    bundle_path: impl AsRef<Path>,
    uri: &str,
) -> Result<Option<PathBuf>, ProjectStoreError> {
    Ok(classify_material_uri(bundle_path, uri)?.resolved_path)
}
```

**Apply:** If Phase 14 adds derived path helpers to `project_store`, keep them pure path helpers with validation. Do not put SQLite operations, generation state, cache keys, or invalidation decisions in `project_store`.

---

### `crates/preview_service/src/service.rs` (modified service, request-response + file-I/O)

**Analog:** existing same file.

**Current transitional cache root pattern to replace** (lines 20-38):
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewServiceConfig {
    pub cache_root: PathBuf,
    pub ffmpeg_path: PathBuf,
    pub compiler_capabilities: CompilerCapabilities,
    pub preview_frame_max_dimensions: OutputDimensions,
    pub preview_segment_max_dimensions: OutputDimensions,
}
```

**Graph/fingerprint/cache-key flow** (lines 223-245):
```rust
let runtime_capability_fingerprint = deterministic_fingerprint(
    "preview-runtime-capabilities",
    &config.compiler_capabilities,
);
let snapshot =
    RenderGraphSnapshot::from_graph(&graph, &output_profile, &runtime_capability_fingerprint);
let material_dependencies = graph
    .materials
    .iter()
    .map(|material| material.material_id.clone())
    .collect::<Vec<_>>();
let key = preview_cache_key(
    profile,
    snapshot.target_timerange.clone(),
    &snapshot,
    material_dependencies,
);
let artifact_path = artifact_path(&config.cache_root, &key);
let artifact = PreviewArtifact {
    profile,
    path: path_to_string(&artifact_path),
    mime_type: profile.mime_type().to_owned(),
};
```

**Apply:** Route preview artifacts through Rust-owned project-local artifact store APIs. Preserve render graph/fingerprint ownership in Rust. Deprecate renderer-supplied `cacheRoot` by resolving artifact roots from bundle/project APIs.

---

### `crates/draft_model/src/*.rs` and Generated Contracts (model, request-response)

**Analog:** `crates/draft_model/src/delta.rs` and `crates/draft_model/tests/schema_exports.rs`

**Contract derive pattern** (`delta.rs` lines 10-20):
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
```

**Generated TypeScript export pattern** (`schema_exports.rs` lines 67-88):
```rust
#[test]
fn schema_exports_generated_contract_artifacts_from_rust() {
    let root = project_root();
    let schema_path = root.join("schemas/command.schema.json");
    let draft_schema_path = root.join("schemas/draft.schema.json");
    let generated_dir = root.join("apps/desktop-electron/src/generated");

    let schema_json = command_schema_json();
    ...
    assert_or_update_contract_file(&schema_path, &format!("{schema_json}\n"));
```

**Command envelope export list pattern** (`schema_exports.rs` lines 88-139):
```rust
let command_envelope_ts = ts_contract_with_prelude(
    "import type { Draft, DraftCanvasConfig, ... } from \"./Draft\";\n\n",
    &[
        export_decl::<CommandName>(),
        export_decl::<PingCommandPayload>(),
        ...
        export_decl::<CommandPayload>(),
        export_decl::<CommandEnvelope>(),
    ],
);
```

**Apply:** Add artifact status/progress/quota/GC command payloads and result types in Rust with `JsonSchema` + `TS`; update schema export lists. Generated TS remains a checked artifact and must not be hand-edited.

---

### `crates/bindings_node/src/*artifact*.rs` (provider/service, request-response)

**Analog:** `crates/bindings_node/src/realtime_preview_service.rs`

**Registry wrapper pattern** (lines 17-31):
```rust
#[derive(Debug, Default)]
pub struct RealtimePreviewBindingRegistry {
    runtime: RealtimePreviewRuntime,
    next_binding_id: u64,
    sessions: BTreeMap<String, PreviewSessionId>,
}

impl RealtimePreviewBindingRegistry {
    pub fn new() -> Self {
        Self {
            runtime: RealtimePreviewRuntime::new(),
            next_binding_id: 1,
            sessions: BTreeMap::new(),
        }
    }
```

**Validate-then-delegate pattern** (lines 74-87):
```rust
pub fn close_session(
    &mut self,
    session_id: &str,
) -> Result<RealtimePreviewClosedBindingResponse, RealtimePreviewBindingError> {
    validate_binding_session_id(session_id)?;
    let runtime_id = self
        .sessions
        .remove(session_id)
        .ok_or_else(|| RealtimePreviewBindingError::unknown_session(session_id))?;
    let closed = self.runtime.close_session(runtime_id);
    Ok(RealtimePreviewClosedBindingResponse {
        session_id: session_id.to_owned(),
        closed,
    })
}
```

**Apply:** Bindings should validate command/session IDs, delegate to Rust artifact store/resource manager, and return user-safe typed responses. Do not compute roots, cache keys, fingerprints, SQLite queries, GC candidates, or invalidation in TypeScript or Electron main.

---

### `scripts/phase14-source-guards.sh` and `package.json` Scripts (config/test, batch)

**Analog:** `scripts/phase13-source-guards.sh` and `package.json`

**Guard structure pattern** (`phase13-source-guards.sh` lines 1-7, 33-60):
```bash
#!/usr/bin/env bash
set -euo pipefail

if ! command -v rg >/dev/null 2>&1; then
  echo "phase13-source-guards: rg is required" >&2
  exit 1
fi

fail() {
  echo "phase13-source-guards: $1" >&2
  exit 1
}

fail_matches() {
  local message="$1"
  local pattern="$2"
  shift 2
  local matches
  matches="$(matches_for_pattern "$pattern" "$@" || true)"
  if [ -n "$matches" ]; then
    printf '%s\n' "$matches" >&2
    fail "$message"
  fi
}
```

**Renderer forbidden patterns** (`phase13-source-guards.sh` lines 89-93):
```bash
RENDERER_GRAPH_DIRTY_CACHE_PATTERN='\b(?:RenderGraphNodeId|renderGraphNodeId|RenderGraphDiff|renderGraphDiff|graphDiff|dirtyRanges?|DirtyRange|dirtyRangePropagation|changedGraphNodeIds|previewCacheKey|cacheKey|cacheFingerprint|semanticFingerprint|nodeFingerprint|invalidationDecision|invalidateDirtyRange|artifactSchemaVersion|generatorVersion)\b'
RENDERER_FFMPEG_PATTERN='\b(?:FfmpegJob|FfmpegExecutor|ffmpegArgs|ffprobeArgs|filter_complex|filterComplex|ffmpegScripts|exportScript|AssSidecar|child_process|execFile|exec\s*\(|spawn\s*\()\b'
FLOAT_TIME_PATTERN='\b(?:targetTimeSeconds|target_time_seconds|timelineSeconds|timeline_seconds|durationSeconds|duration_seconds|sourceTimeSeconds|source_time_seconds|targetTimerangeSeconds|target_timerange_seconds|sourceTimerangeSeconds|source_timerange_seconds|seconds\s*:\s*f32|seconds\s*:\s*f64)\b'
DERIVED_ARTIFACT_PATTERN='\b(?:previewCaches?|previewArtifacts?|renderGraph|graphSnapshots?|ffmpegScripts?|proxyFiles?|thumbnailPath|waveformPath|artifactStore|artifact-store|artifact_store|derivedArtifacts?)\b'
PHASE14_OR_16_SCOPE_PATTERN='\b(?:artifact-store\.sqlite|artifactStoreSqlite|rusqlite|sqlx|CREATE TABLE|JobScheduler|priorityQueue|starvation|backpressure)\b'
```

**Package script pattern** (`package.json` lines 65-72):
```json
"test:phase13-rust": "cargo test -p draft_model contract -- --nocapture && cargo test -p draft_commands --test command_delta -- --nocapture && cargo test -p render_graph --test node_identity -- --nocapture && cargo test -p preview_service --test dirty_propagation -- --nocapture && cargo test -p testkit large_timeline -- --nocapture && cargo test -p testkit large_timeline_incremental -- --nocapture && cargo test -p testkit preview_export_parity -- --nocapture && cargo test -p draft_model draft_fixtures -- --nocapture",
"test:phase13-source-guards": "bash scripts/phase13-source-guards.sh",
"test:phase13": "pnpm run test:phase13-rust && pnpm run test:phase13-source-guards && pnpm run test:contracts",
"test:contracts": "git diff --exit-code schemas apps/desktop-electron/src/generated"
```

**Apply:** Add `test:phase14-rust`, `test:phase14-source-guards`, and `test:phase14`. Guard renderer/default UI against `.veproj/derived`, SQLite names, absolute artifact roots, cache keys, fingerprints, graph node IDs, dirty ranges, FFmpeg command construction, and Phase 16 scheduler terms.

---

### Desktop UI Resource Status (component, request-response presentation)

**Analog:** `apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx`, `WorkspaceShell.tsx`, `styles.css`

**Five-zone shell pattern** (`WorkspaceShell.tsx` lines 114-163):
```tsx
return (
  <main className="workspace" aria-label="剪映风格编辑工作区">
    <header className="top-feature-bar" aria-label="顶部功能区">
      ...
    </header>

    <section className="material-panel" aria-label="素材面板">
      <FeaturePanel ... />
    </section>

    <section className="preview-monitor" aria-label="预览窗口">
      <PreviewMonitor ... />
    </section>
```

**Material panel local UI state pattern** (`FeaturePanel.tsx` lines 74-93):
```tsx
const [search, setSearch] = useState("");
const [filter, setFilter] = useState<MaterialFilter>("全部");
const filteredMaterials = useMemo(
  () =>
    workspace.materials.filter((material) => {
      const matchesSearch =
        search.trim().length === 0 ||
        material.displayName.toLocaleLowerCase().includes(search.trim().toLocaleLowerCase()) ||
        material.uri.toLocaleLowerCase().includes(search.trim().toLocaleLowerCase());
      ...
      return matchesSearch && matchesFilter;
    }),
  [filter, search, workspace.materials]
);
```

**Manual CSS layout tokens** (`styles.css` lines 60-71, 237-249):
```css
.workspace {
  display: grid;
  grid-template-columns: 336px minmax(420px, 1fr) 288px;
  grid-template-rows: 48px minmax(0, 1fr) 252px;
  width: 100vw;
  height: 100vh;
  min-width: 1120px;
  min-height: 720px;
  gap: 1px;
  overflow: hidden;
  background: #343431;
}

.primary-action {
  height: 32px;
  min-width: 88px;
  padding: 0 12px;
  border: 1px solid #20c7d9;
  border-radius: 6px;
  color: #061314;
  background: #20c7d9;
```

**Apply:** Extend the existing material panel with compact `资源任务`, material-row chips, and `资源维护` controls. Keep UI local state limited to filters, expanded/dismissed controls, and pending button states. Use Rust-returned status/progress/display strings; default production UI must not show SQLite, `.veproj/derived`, cache roots, fingerprints, graph keys, dirty ranges, or raw logs.

## Shared Patterns

### Rust-Owned Semantics
**Source:** `crates/draft_model/src/delta.rs` lines 10-20 and `crates/preview_service/src/cache.rs` lines 197-233  
**Apply to:** artifact invalidation, resource indexing, bindings, UI commands

All semantic facts cross from Rust via typed structs using `Serialize`, `Deserialize`, `JsonSchema`, `TS`, camelCase, and `deny_unknown_fields`. Renderer displays and submits command envelopes only.

### Project-Contained Paths
**Source:** `crates/project_store/src/paths.rs` lines 88-108  
**Apply to:** derived root helpers, blob relative paths, sync manifest paths, diagnostics display refs

Reject parent traversal, roots, and platform prefixes for project-relative derived paths. Convert stored/display refs to slash-separated UTF-8.

### Dirty/Dependency Invalidation
**Source:** `crates/preview_service/src/cache.rs` lines 493-560  
**Apply to:** artifact dependency rows and replacement/relink/delete invalidation

Invalidate by full-draft flag, overlapping integer ranges, material dependencies, graph node stable keys, and fingerprint/profile mismatch. Preserve exact invalidation as normal path; use full draft only for explicit or overflow/unknown cases.

### Generation Cancellation And Progress
**Source:** `crates/media_runtime/src/job.rs` lines 64-81 and 179-196  
**Apply to:** artifact job/chunk rows, cancel/resume commands, UI task rows

Expose persisted job state plus optional per-mille progress. Reuse `CancelToken` semantics for cancellation and store cancellation/resume state in SQLite rows.

### Generated Contracts
**Source:** `crates/draft_model/tests/schema_exports.rs` lines 67-144 and 146-214  
**Apply to:** artifact status, generation retry/resume/cancel, quota/GC, sync manifest command/result contracts

Add Rust contract types to schema/TS export lists and keep `pnpm run test:contracts` as the generated artifact freshness gate.

### Source Guards
**Source:** `scripts/phase13-source-guards.sh` lines 89-93 and 141-173  
**Apply to:** Phase 14 ownership boundaries

Use comment-filtered `rg` guards and injected negative checks. Phase 14 should allow artifact-store terms in Rust artifact-store code while forbidding them in renderer/default UI and canonical draft/schema surfaces.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `crates/artifact_store/src/schema.rs` or `migrations.rs` | migration | CRUD | No local SQLite, SQL migration, `rusqlite`, or DB connection wrapper exists. Use research-approved `rusqlite` patterns and require focused tests for PRAGMAs, foreign keys, schema versioning, migration idempotence, and orphan rejection. |

## Metadata

**Analog search scope:** `crates/`, `apps/desktop-electron/src/`, `scripts/`, root `Cargo.toml`, root `package.json`; excluded `reference/` as requested.  
**Files scanned:** repo file list via `rg --files`, targeted grep for cache/fingerprint/cancel/contract/guard terms, and focused reads of 13 analog files.  
**Pattern extraction date:** 2026-06-19
