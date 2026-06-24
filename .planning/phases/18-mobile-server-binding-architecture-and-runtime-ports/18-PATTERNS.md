# Phase 18: Mobile/Server Binding Architecture And Runtime Ports - Pattern Map

**Mapped:** 2026-06-25
**Files analyzed:** 29 new/modified files
**Analogs found:** 27 / 29

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `Cargo.toml` | config | batch | `Cargo.toml` | exact |
| `crates/editor_runtime/Cargo.toml` | config | batch | `crates/bindings_node/Cargo.toml` | role-match |
| `crates/editor_runtime/src/lib.rs` | service/provider | request-response | `crates/bindings_node/src/lib.rs` + `crates/project_store/src/bundle.rs` | role-match |
| `crates/editor_runtime/src/session.rs` | service/model | request-response | `crates/bindings_node/src/project_session_service.rs` | role-match |
| `crates/editor_runtime/src/handles.rs` | model/service | request-response | `crates/media_runtime/src/frame.rs` + `crates/media_runtime/src/texture.rs` | exact |
| `crates/editor_runtime/src/project_session.rs` | service | CRUD + file-I/O | `crates/bindings_node/src/project_session_service.rs` + `crates/project_store/src/bundle.rs` | role-match |
| `crates/editor_runtime/src/export.rs` | service | batch + event-driven | `crates/bindings_node/src/preview_export_service.rs` + `crates/task_runtime/src/scheduler.rs` | role-match |
| `crates/editor_runtime/src/error.rs` | utility/model | transform | `crates/bindings_node/src/preview_export_service.rs` | role-match |
| `crates/bindings_node/Cargo.toml` | config | batch | `crates/bindings_node/Cargo.toml` | exact |
| `crates/bindings_node/src/lib.rs` | adapter/controller | request-response | `crates/bindings_node/src/lib.rs` | exact |
| `crates/bindings_node/src/project_session_service.rs` | adapter/service | request-response | `crates/bindings_node/src/project_session_service.rs` | exact |
| `crates/bindings_node/src/preview_export_service.rs` | adapter/service | batch + event-driven | `crates/bindings_node/src/preview_export_service.rs` | exact |
| `apps/desktop-electron/src/main/nativeBinding.ts` | adapter | request-response | `apps/desktop-electron/src/main/nativeBinding.ts` | exact |
| `apps/desktop-electron/src/main/index.ts` | controller | request-response | `apps/desktop-electron/src/main/index.ts` | exact |
| `apps/desktop-electron/src/preload/index.ts` | middleware/adapter | request-response | `apps/desktop-electron/src/preload/index.ts` | exact |
| `crates/bindings_c/Cargo.toml` | config | batch | `crates/bindings_node/Cargo.toml` | role-match |
| `crates/bindings_c/src/lib.rs` | adapter | request-response + FFI | `crates/bindings_node/src/lib.rs` + `crates/media_runtime/src/frame.rs` | partial |
| `crates/bindings_c/cbindgen.toml` | config | batch | none | no analog |
| `crates/bindings_c/include/video_editor_runtime.h` | generated interface | request-response + FFI | none | no analog |
| `crates/bindings_c/tests/abi_smoke.rs` | test | request-response + FFI | `crates/bindings_node/tests/binding_smoke.rs` + `crates/media_runtime/tests/frame_pool.rs` | role-match |
| `crates/server_runtime/Cargo.toml` | config | batch | `crates/bindings_node/Cargo.toml` | role-match |
| `crates/server_runtime/src/lib.rs` | service | batch + file-I/O | `crates/bindings_node/src/preview_export_service.rs` + `crates/project_store/src/bundle.rs` | role-match |
| `crates/server_runtime/src/main.rs` | route/controller | batch + file-I/O | `crates/testkit/tests/render_smoke.rs` | partial |
| `crates/server_runtime/tests/server_export_smoke.rs` | test | batch + file-I/O | `crates/bindings_node/tests/export_commands.rs` + `crates/bindings_node/tests/scheduler_export.rs` | role-match |
| `docs/mobile-runtime-contracts.md` | docs | event-driven lifecycle | `docs/runtime-boundaries.md` | role-match |
| `docs/runtime-boundaries.md` | docs | transform | `docs/runtime-boundaries.md` | exact |
| `scripts/phase18-source-guards.sh` | test/guard | batch | `scripts/phase17-1-source-guards.sh` + `scripts/phase16-source-guards.sh` | exact |
| `scripts/phase18-abi-drift.sh` | test/guard | batch + file-I/O | `scripts/phase17-1-source-guards.sh` + research cbindgen command | partial |
| `package.json` | config | batch | `package.json` | exact |

## Pattern Assignments

### `Cargo.toml` and new crate manifests (config, batch)

**Analog:** `Cargo.toml` and `crates/bindings_node/Cargo.toml`

**Workspace member pattern** (`Cargo.toml` lines 1-21):
```toml
[workspace]
members = [
  "crates/draft_model",
  "crates/draft_import",
  "crates/adapter_kaipai",
  "crates/draft_commands",
  "crates/engine_core",
  "crates/render_graph",
  "crates/task_runtime",
  "crates/realtime_preview_runtime",
  "crates/audio_engine",
  "crates/audio_output_desktop",
  "crates/ffmpeg_compiler",
  "crates/media_runtime",
  "crates/media_runtime_desktop",
  "crates/artifact_store",
  "crates/project_store",
  "crates/preview_service",
  "crates/testkit",
  "crates/bindings_node",
]
resolver = "3"
```

**Crate manifest pattern** (`crates/bindings_node/Cargo.toml` lines 1-10, 13-40):
```toml
[package]
name = "bindings_node"
version = "0.1.0"
edition.workspace = true
rust-version.workspace = true
license.workspace = true
publish = false

[lib]
crate-type = ["cdylib", "rlib"]
path = "src/lib.rs"

[dependencies]
adapter_kaipai = { path = "../adapter_kaipai" }
artifact_store = { path = "../artifact_store" }
audio_engine = { path = "../audio_engine" }
audio_output_desktop = { path = "../audio_output_desktop" }
blake3 = "1.8.5"
draft_commands = { path = "../draft_commands" }
draft_import = { path = "../draft_import" }
draft_model = { path = "../draft_model" }
engine_core = { path = "../engine_core" }
ffmpeg_compiler = { path = "../ffmpeg_compiler" }
media_runtime = { path = "../media_runtime" }
media_runtime_desktop = { path = "../media_runtime_desktop" }
image = { version = "0.25.10", default-features = false, features = ["jpeg", "png", "webp"] }
napi = { version = "3.9.2", features = ["serde-json"] }
napi-derive = "3.5.6"
preview_service = { path = "../preview_service" }
project_store = { path = "../project_store" }
realtime_preview_runtime = { path = "../realtime_preview_runtime" }
render_graph = { path = "../render_graph" }
rusqlite = "0.40.1"
serde = { version = "1.0.228", features = ["derive"] }
serde_json = "1.0.150"
task_runtime = { path = "../task_runtime" }
wgpu = "29.0.3"
```

**Apply to:** `crates/editor_runtime/Cargo.toml`, `crates/bindings_c/Cargo.toml`, `crates/server_runtime/Cargo.toml`.
`bindings_c` should use `crate-type = ["cdylib", "staticlib", "rlib"]`; `server_runtime` can use a normal `rlib` plus a binary target.

---

### `crates/editor_runtime/src/lib.rs` (service/provider, request-response)

**Analog:** `crates/bindings_node/src/lib.rs` and `crates/project_store/src/bundle.rs`

**Imports/module surface pattern** (`crates/bindings_node/src/lib.rs` lines 6-22, 44-53):
```rust
use draft_model::{
    ArtifactGenerationActionCommandPayload, AudioPreviewCommandPayload, CancelExportCommandPayload,
    CommandEnvelope, CommandError, CommandErrorKind, CommandName, CommandPayload,
    CommandResultEnvelope, DRAFT_MODEL_VERSION, ExportJobStatusResponse,
    GetArtifactQuotaStatusCommandPayload, GetArtifactStatusCommandPayload,
    GetExportJobStatusCommandPayload, MissingMaterialCommandDiagnostic,
    MissingMaterialCommandDiagnosticKind, PingResponse, RefreshArtifactStatusCommandPayload,
    RunArtifactGarbageCollectionCommandPayload, StartExportCommandPayload, VersionResponse,
};
use media_runtime::{DiscoveryError, discover_runtime_config};
use napi::Env;
use napi::bindgen_prelude::Result;
use napi::threadsafe_function::ThreadsafeFunction;
use napi_derive::napi;
use project_store::{ProjectStoreError, ProjectStoreWarning, resolve_material_uri};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

pub mod artifact_store_service;
pub mod audio_service;
pub mod material_service;
pub mod native_preview_presenter;
pub mod preview_export_service;
pub mod project_session_service;
pub mod realtime_preview_service;
pub mod runtime_capability_service;
pub mod task_runtime_service;
pub mod timeline_selection;
```

**Project-store boundary pattern** (`crates/project_store/src/bundle.rs` lines 22-65):
```rust
pub fn create_project_bundle(
    fs: &impl PlatformFileSystem,
    bundle_path: impl AsRef<Path>,
    draft: &Draft,
) -> Result<ProjectBundle, ProjectStoreError> {
    save_project_bundle(fs, bundle_path, draft)
}

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

    Ok(ProjectBundle {
        bundle_path: bundle_path.to_path_buf(),
        project_json_path,
        draft: draft.clone(),
    })
}
```

**Copy rule:** `editor_runtime` should expose shared Rust request/response structs and services. Do not copy `#[napi]`, `napi::Result`, or `serde_json::Value` into the semantic runtime layer.

---

### `crates/editor_runtime/src/session.rs` and `crates/editor_runtime/src/project_session.rs` (service/model, request-response + file-I/O)

**Analog:** `crates/bindings_node/src/project_session_service.rs`

**Current session state to move below adapters** (lines 892-915):
```rust
#[derive(Debug)]
struct ProjectSession {
    session_id: String,
    revision: u64,
    bundle_path: PathBuf,
    project_json_path: PathBuf,
    draft: Draft,
    command_state: CommandState,
    selection: TimelineSelection,
    playhead: Microseconds,
    active_interactions: HashMap<String, ActiveProjectInteraction>,
    next_interaction_id: u64,
    next_interaction_generation: u64,
}

#[derive(Debug, Clone)]
struct ActiveProjectInteraction {
    session: DraftProjectInteractionSession,
    latest_payload: Option<ProjectInteractionPayload>,
    provisional_view_model: Option<ProjectSessionViewModel>,
    provisional_delta: Option<CommandDelta>,
    provisional_draft: Option<Draft>,
    provisional_selection: Option<TimelineSelection>,
}
```

**Adapter parse pattern that should remain in adapters** (lines 944-1002):
```rust
pub fn create_project_session(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = match serde_json::from_value::<CreateProjectSessionRequest>(request) {
        Ok(request) => request,
        Err(error) => {
            return crate::to_js_value(crate::error_envelope(
                CommandErrorKind::InvalidPayload,
                format!("Invalid createProjectSession payload: {error}"),
                Some("createProjectSession".to_string()),
            ));
        }
    };

    with_project_session_registry(|registry| registry.create_session(request))
}

pub fn open_project_session(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = match serde_json::from_value::<OpenProjectSessionRequest>(request) {
        Ok(request) => request,
        Err(error) => {
            return crate::to_js_value(crate::error_envelope(
                CommandErrorKind::InvalidPayload,
                format!("Invalid openProjectSession payload: {error}"),
                Some("openProjectSession".to_string()),
            ));
        }
    };

    with_project_session_registry(|registry| registry.open_session(request))
}
```

**Interaction begin/update/commit lifecycle pattern** (lines 1755-1812, 1814-1893, 1896-1946):
```rust
fn begin_interaction(
    &mut self,
    request: BeginProjectInteractionRequest,
) -> Result<serde_json::Value> {
    let Some(session) = self.sessions.get_mut(&request.session_id) else {
        return project_interaction_error(
            "beginProjectInteraction",
            CommandErrorKind::InvalidProject,
            format!("Project session not found: {}", request.session_id),
        );
    };
    if request.expected_revision != session.revision {
        return stale_interaction_revision_error(
            "beginProjectInteraction",
            request.expected_revision,
            session.revision,
        );
    }

    let interaction_id = session.next_project_interaction_id();
    let generation = session.next_project_interaction_generation();
    let interaction = DraftProjectInteractionSession::new(
        interaction_id.clone(),
        request.kind,
        session.revision,
        generation,
    );
    session.active_interactions.insert(
        interaction_id,
        ActiveProjectInteraction {
            session: interaction,
            latest_payload: None,
            provisional_view_model: None,
            provisional_delta: None,
            provisional_draft: None,
            provisional_selection: None,
        },
    );

    crate::to_js_value(crate::ok_envelope(response))
}

fn update_interaction(
    &mut self,
    request: UpdateProjectInteractionRequest,
) -> Result<serde_json::Value> {
    let Some(active) = session.active_interactions.get(&request.interaction_id) else {
        return missing_interaction_error("updateProjectInteraction", &request.interaction_id);
    };
    if let Some(error) = validate_interaction_revision(
        "updateProjectInteraction",
        request.expected_revision,
        active.session.base_revision,
    ) {
        return error;
    }
    let mut accepted = active.session.clone();
    if let Some(error) =
        accept_interaction_sequence("updateProjectInteraction", &mut accepted, request.sequence)
    {
        return error;
    }

    let provisional = match session.provisional_interaction_payload(&request.payload) {
        Ok(response) => response,
        Err(message) => {
            return project_interaction_error(
                "updateProjectInteraction",
                CommandErrorKind::InvalidTimelineEdit,
                message,
            );
        }
    };
    active.session = accepted.clone();
    active.latest_payload = Some(request.payload);
    active.provisional_view_model = Some(provisional_view_model.clone());
    active.provisional_delta = Some(provisional_delta.clone());
    active.provisional_draft = Some(provisional.draft);
    active.provisional_selection = Some(provisional.selection);
}

fn commit_interaction(
    &mut self,
    request: CommitProjectInteractionRequest,
) -> Result<serde_json::Value> {
    let Some(active) = session.active_interactions.get(&request.interaction_id) else {
        return missing_interaction_error("commitProjectInteraction", &request.interaction_id);
    };
    let interaction = active.session.clone();
    let Some(payload) = active.latest_payload.clone() else {
        return project_interaction_error(
            "commitProjectInteraction",
            CommandErrorKind::InvalidPayload,
            format!(
                "Project interaction {} has no accepted update to commit",
                request.interaction_id
            ),
        );
    };

    let response = session.commit_interaction_payload(payload, &interaction)?;
    session.active_interactions.remove(&request.interaction_id);
    Ok(response)
}
```

**Commit persistence pattern** (lines 2191-2304):
```rust
fn commit_interaction_payload(
    &mut self,
    payload: ProjectInteractionPayload,
    interaction: &DraftProjectInteractionSession,
) -> Result<serde_json::Value> {
    let intent = match payload.into_project_intent() {
        Ok(intent) => intent,
        Err(message) => {
            return project_interaction_error(
                "commitProjectInteraction",
                CommandErrorKind::InvalidPayload,
                message,
            );
        }
    };
    match intent {
        ProjectIntent::SetSessionPlayhead { playhead } => {
            self.playhead = playhead;
            crate::to_js_value(crate::ok_envelope(ProjectInteractionCommitResponse {
                revision: self.revision,
                delta: CommandDelta::none(
                    CommandDeltaName::SeekAudioPreview,
                    "playhead scrub committed",
                ),
                // ...
            }))
        }
        intent => {
            let edit_payload = match self.intent_payload(intent) {
                Ok(payload) => payload,
                Err(message) => {
                    return project_interaction_error(
                        "commitProjectInteraction",
                        CommandErrorKind::InvalidTimelineEdit,
                        message,
                    );
                }
            };
            let response = match draft_commands::timeline::execute_timeline_edit(edit_payload) {
                Ok(response) => response,
                Err(error) => {
                    return project_interaction_error(
                        "commitProjectInteraction",
                        CommandErrorKind::InvalidTimelineEdit,
                        error.to_string(),
                    );
                }
            };
            self.apply_interaction_response(response, interaction)
        }
    }
}
```

**Copy rule:** Move the registry, session state, revision checks, interactions, `.veproj` save/open, and snapshots into `editor_runtime`. Leave only transport parsing/serialization in `bindings_node`.

---

### `crates/editor_runtime/src/handles.rs` (model/service, request-response)

**Analog:** `crates/media_runtime/src/frame.rs` and `crates/media_runtime/src/texture.rs`

**Opaque frame handle and lease structs** (`crates/media_runtime/src/frame.rs` lines 18-30, 53-95):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FrameHandleId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FrameLeaseId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FramePoolLimits {
    pub max_outstanding_leases: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuFrameHandle {
    pub handle_id: FrameHandleId,
    pub owner_session: MediaSessionId,
    pub generation: Option<u64>,
    pub dimensions: FrameDimensions,
    pub pixel_format: VideoPixelFormat,
    pub estimated_byte_len: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind", content = "handle")]
pub enum VideoFrameStorage {
    Cpu(CpuFrameHandle),
    Texture(TextureHandle),
    PlatformOpaque(PlatformFrameHandle),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DecodedVideoFrame {
    pub handle_id: FrameHandleId,
    pub owner_session: MediaSessionId,
    pub playback_generation: Option<u64>,
    pub source_time_us: u64,
    pub duration_us: Option<u64>,
    pub frame_index: Option<u64>,
    pub dimensions: FrameDimensions,
    pub pixel_format: VideoPixelFormat,
    pub color: VideoColorMetadata,
    pub storage: VideoFrameStorage,
    pub release: FrameLeaseId,
}
```

**Lease acquire/release/close pattern** (`crates/media_runtime/src/frame.rs` lines 163-291):
```rust
#[derive(Debug)]
pub struct FramePool {
    owner_session: MediaSessionId,
    limits: FramePoolLimits,
    next_id: u64,
    active: BTreeMap<FrameLeaseId, DecodedVideoFrame>,
}

impl FramePool {
    pub fn acquire_video_frame(
        &mut self,
        request: FrameLeaseRequest,
    ) -> Result<DecodedVideoFrame, FramePoolError> {
        if self.active.len() >= self.limits.max_outstanding_leases {
            return Err(FramePoolError::new(
                FramePoolErrorKind::LeaseLimitExceeded,
                "frame pool outstanding lease limit exceeded",
            ));
        }

        let handle_id = FrameHandleId(format!("frame-{}", self.next_id));
        let lease_id = FrameLeaseId(format!("lease-{}", self.next_id));
        self.next_id += 1;
        // storage owner validation happens before insertion
        self.active.insert(lease_id, frame.clone());
        Ok(frame)
    }

    pub fn release_for_session(
        &mut self,
        owner_session: &MediaSessionId,
        lease_id: FrameLeaseId,
    ) -> Result<FrameReleaseDiagnostic, FramePoolError> {
        if owner_session != &self.owner_session {
            return Err(FramePoolError::new(
                FramePoolErrorKind::OwnerSessionMismatch,
                "release owner session does not match frame pool session",
            ));
        }

        let frame = self.active.remove(&lease_id).ok_or_else(|| {
            FramePoolError::new(FramePoolErrorKind::LeaseNotFound, "frame lease not found")
        })?;

        Ok(release_diagnostic(lease_id, &frame, "frame lease released"))
    }

    pub fn close_session(&mut self) -> FramePoolCloseReport {
        let leak_diagnostics = std::mem::take(&mut self.active)
            .into_iter()
            .map(|(lease_id, frame)| {
                release_diagnostic(
                    lease_id,
                    &frame,
                    "unreleased frame lease closed with session",
                )
            })
            .collect();
        FramePoolCloseReport { owner_session: self.owner_session.clone(), leak_diagnostics }
    }
}
```

**Native texture identity and validation pattern** (`crates/media_runtime/src/texture.rs` lines 21-44, 95-151, 208-249):
```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TextureHandleId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeDeviceId {
    pub backend: TextureBackend,
    pub adapter_id: String,
    pub device_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextureHandle {
    pub handle_id: TextureHandleId,
    pub owner_session: MediaSessionId,
    pub generation: u64,
    pub backend: TextureBackend,
    pub device_id: RuntimeDeviceId,
    pub dimensions: FrameDimensions,
    pub pixel_format: VideoPixelFormat,
    pub color: VideoColorMetadata,
}

pub struct NativeTextureLeaseRegistry {
    leases: Rc<RefCell<BTreeMap<TextureHandleId, NativeTextureLease>>>,
}

pub fn resolve(
    &self,
    expected: &TextureHandle,
) -> Result<NativeTextureLease, NativeTextureLeaseError> {
    let leases = self.leases.borrow();
    let lease = leases.get(&expected.handle_id).ok_or_else(|| {
        NativeTextureLeaseError::new(
            NativeTextureLeaseErrorKind::NotRegistered,
            format!("native texture lease {} is not registered", expected.handle_id.0),
        )
    })?;
    validate_expected_handle(expected, &lease.handle)?;
    Ok(lease.clone())
}

fn validate_expected_handle(
    expected: &TextureHandle,
    registered: &TextureHandle,
) -> Result<(), NativeTextureLeaseError> {
    if expected.owner_session != registered.owner_session {
        return Err(NativeTextureLeaseError::new(
            NativeTextureLeaseErrorKind::OwnerSessionMismatch,
            "native texture lease owner session does not match the decoded frame",
        ));
    }
    if expected.generation != registered.generation {
        return Err(NativeTextureLeaseError::new(
            NativeTextureLeaseErrorKind::StaleGeneration,
            "native texture lease generation does not match the decoded frame",
        ));
    }
    if expected.device_id != registered.device_id {
        return Err(NativeTextureLeaseError::new(
            NativeTextureLeaseErrorKind::DeviceMismatch,
            "native texture lease device identity does not match the decoded frame",
        ));
    }
    Ok(())
}
```

**Copy rule:** Runtime/project/media/frame/texture/artifact handles must include owner session, generation, kind, explicit release, cascading close, double-release/unknown-handle errors, and leak diagnostics. Do not let JS/C/JNI/Swift fabricate metadata.

---

### `crates/editor_runtime/src/export.rs` and `crates/server_runtime/src/lib.rs` (service, batch + file-I/O)

**Analog:** `crates/bindings_node/src/preview_export_service.rs`, `crates/task_runtime/src/scheduler.rs`, `crates/project_store/src/bundle.rs`

**Export imports and tier dependencies** (`crates/bindings_node/src/preview_export_service.rs` lines 10-44):
```rust
use draft_model::{
    DirtyDomain, DirtyRange, Draft, ExportDiagnostic, ExportDiagnosticKind, ExportJobPhase,
    ExportJobStatusResponse, ExportPrepDirtyFacts, ExportPreset, ExportValidationReport,
    MaterialId, Microseconds, PreviewArtifactResponse, PreviewCacheEntryRef,
    PreviewCacheInvalidationResponse, PreviewDiagnostic, PreviewDiagnosticKind,
    PreviewOutputProfile, PreviewStatus, StartExportCommandPayload, TargetTimerange,
};
use engine_core::{EngineProfile, normalize_draft, resolve_render_range};
use ffmpeg_compiler::{
    CompileContext, CompilerCapabilities, FfmpegCompileError, FfmpegJob,
    OutputValidationExpectation as CompileValidation, TextRenderCapability, compile_ffmpeg_job,
};
use media_runtime::{
    CancelToken, FfmpegJobEvent, FfmpegJobResult, FfmpegJobState, FfmpegRuntimeError,
    FfmpegRuntimeJob, OutputValidationError, OutputValidationExpectation, RuntimeCapabilityReport,
    RuntimeConfig, validate_rendered_output,
};
use render_graph::{
    ExportMp4Preset, OutputDimensions, RenderAudioCodec, RenderContainer, RenderGraphPlan,
    RenderOutputProfile, RenderVideoCodec, build_render_graph,
};
use task_runtime::{
    CompletionFreshness, JobCompletion, JobDomain, JobEnvelope, JobId, JobPriority, JobResult,
    JobResultKind, ResourceClass, SchedulerTelemetrySnapshot, TaskCancellationToken,
    TaskRuntimeConfig,
};
```

**Scheduler-backed export service state** (lines 223-289, 371-448):
```rust
#[derive(Clone)]
struct SchedulerExportEntry {
    status: ExportJobStatusResponse,
    export_job_id: JobId,
    export_cancel_token: CancelToken,
    export_task_token: TaskCancellationToken,
    validation_job_id: Option<JobId>,
    validation_task_token: Option<TaskCancellationToken>,
}

#[derive(Clone, Default)]
pub struct SchedulerExportService {
    state: Arc<Mutex<SchedulerExportState>>,
}

struct SchedulerExportState {
    scheduler: task_runtime::JobScheduler,
    entries: BTreeMap<String, SchedulerExportEntry>,
    pending: BTreeMap<JobId, ScheduledExportWork>,
    started_at: Instant,
    next_token_id: u64,
}

impl SchedulerExportService {
    pub fn start_export(
        &self,
        runtime: RuntimeConfig,
        payload: StartExportCommandPayload,
    ) -> Result<SchedulerExportStatusResponse, ExportCommandError> {
        self.start_export_with_validation_executor(
            runtime,
            payload,
            DesktopFfmpegExecutor::default(),
        )
    }

    pub fn start_export_with_validation_executor<E>(
        &self,
        runtime: RuntimeConfig,
        payload: StartExportCommandPayload,
        validation_executor: E,
    ) -> Result<SchedulerExportStatusResponse, ExportCommandError>
    where
        E: FfmpegExecutor + Send + 'static,
    {
        let prepared = prepare_export_job(&runtime, payload)?;
        let export_job_id = JobId::new(prepared.job_id.clone());
        let export_cancel_token = CancelToken::new();
        let initial_status = ExportJobStatusResponse {
            job_id: prepared.job_id.clone(),
            phase: ExportJobPhase::Queued,
            output_path: prepared.output_path.display().to_string(),
            preset: prepared.preset,
            progress_per_mille: Some(0),
            out_time: Some(Microseconds::ZERO),
            log_summary: Some("导出任务已进入调度器队列".to_owned()),
            validation: None,
            diagnostic: None,
            dirty_facts: prepared.dirty_facts.clone(),
        };

        let mut state = self.state.lock().expect("scheduler export lock");
        let envelope = JobEnvelope::new(
            export_job_id.clone(),
            JobDomain::Export,
            JobPriority::UserVisible,
            ResourceClass::FfmpegProcess,
            export_task_token.clone(),
            submitted_at_us,
        );
        state.scheduler.submit(envelope).map_err(|error| {
            ExportCommandError::Scheduler(format!("scheduler export queue rejected: {error}"))
        })?;
        state.entries.insert(prepared.job_id.clone(), SchedulerExportEntry { /* ... */ });
        state.pending.insert(export_job_id, ScheduledExportWork::Export { /* ... */ });
        self.start_ready_jobs()?;
        self.status(&response_job_id)
    }
}
```

**Export graph/compiler/runtime path** (lines 985-1037):
```rust
fn prepare_export_job(
    runtime: &RuntimeConfig,
    payload: StartExportCommandPayload,
) -> Result<PreparedExportJob, ExportCommandError> {
    let output_path = validate_output_path(&payload.output_path)?;
    let sidecar_dir = export_sidecar_dir(&output_path);
    let dirty_facts = payload.dirty_facts.clone();
    let draft = payload.draft;
    let engine_profile = EngineProfile::from_draft_canvas(&draft).map_err(|error| {
        ExportCommandError::Engine(format!("export engine profile resolution failed: {error}"))
    })?;
    let normalized = normalize_draft(&draft, &engine_profile).map_err(|error| {
        ExportCommandError::Engine(format!("export engine normalization failed: {error}"))
    })?;
    let target_timerange = draft_export_timerange(&draft, normalized.duration)?;
    let range = resolve_render_range(&normalized, target_timerange.clone()).map_err(|error| {
        ExportCommandError::Engine(format!("export range resolution failed: {error}"))
    })?;
    let graph = build_render_graph(&normalized, &range).map_err(|error| {
        ExportCommandError::RenderGraph(format!("export render graph failed: {error}"))
    })?;
    let output_profile = RenderOutputProfile::export_mp4(
        OutputDimensions::new(engine_profile.canvas_width, engine_profile.canvas_height),
        range.frame_rate.clone(),
        target_timerange,
        export_preset(payload.preset),
    );
    let plan = RenderGraphPlan::new(graph, output_profile).map_err(|error| {
        ExportCommandError::RenderGraph(format!("export output profile failed: {error}"))
    })?;
    let compile_context = CompileContext::new(&output_path, &sidecar_dir)
        .with_capabilities(compiler_capabilities_from_runtime(runtime));
    let ffmpeg_job =
        compile_ffmpeg_job(&plan, &compile_context).map_err(ExportCommandError::Compile)?;
    write_export_sidecars(&ffmpeg_job)?;
    let runtime_job = FfmpegRuntimeJob::new(
        ffmpeg_job.job_id.clone(),
        runtime.ffmpeg.path.clone(),
        ffmpeg_job.args,
        output_path.clone(),
    )
    .with_expected_duration_microseconds(ffmpeg_job.validation.expected_duration.get());
    Ok(PreparedExportJob { /* ... */ })
}
```

**Scheduler cancellation/commit pattern** (`crates/task_runtime/src/scheduler.rs` lines 241-350):
```rust
pub fn cancel_at(
    &mut self,
    job_id: &JobId,
    cancelled_at_us: u64,
) -> Result<JobCancellationState, SchedulerRejected> {
    if let Some(index) = self.queue.iter().position(|queued| queued.envelope.job_id == *job_id) {
        let queued = self.queue.remove(index).expect("selected queued index must exist");
        queued.envelope.cancellation_token.cancel();
        self.terminal.insert(queued.envelope.job_id, TerminalJobState::Cancelled);
        self.telemetry.record_canceled_wait(wait_time_us);
        return Ok(JobCancellationState::Queued);
    }

    if let Some(running) = self.running.remove(job_id) {
        running.envelope.cancellation_token.cancel();
        self.release_resource(running.envelope.resource_class);
        self.terminal.insert(running.envelope.job_id, TerminalJobState::Cancelled);
        self.telemetry.record_canceled_running(run_time_us, job_duration_us);
        return Ok(JobCancellationState::Running);
    }

    Err(SchedulerRejected::UnknownJob { job_id: job_id.clone() })
}

pub fn complete_with_commit<F>(
    &mut self,
    job_id: &JobId,
    result: JobResult,
    completed_at_us: u64,
    current: CompletionFreshness,
    commit_visible_state: F,
) -> Result<JobCompletion, SchedulerRejected>
where
    F: FnOnce(&JobResult),
{
    if running.envelope.freshness.is_stale_for(current) {
        self.terminal.insert(running.envelope.job_id.clone(), TerminalJobState::StaleRejected);
        self.telemetry.record_stale_rejected();
        return Ok(JobCompletion::StaleRejected { job_id: running.envelope.job_id });
    }

    commit_visible_state(&result);
    self.telemetry.record_completed(run_time_us, job_duration_us, &result);
    Ok(JobCompletion::Accepted { job_id: running.envelope.job_id })
}
```

**Copy rule:** Server runtime must open `.veproj` through `project_store`, resolve bundle-relative materials, call the shared runtime export service, and report status/progress/cancel through the same scheduler-backed job registry. Do not reimplement render graph, compiler, FFmpeg process, or validation logic in a CLI.

---

### `crates/editor_runtime/src/error.rs` (utility/model, transform)

**Analog:** `crates/bindings_node/src/preview_export_service.rs`

**Typed error enum and diagnostics pattern** (lines 50-66, 117-144, 1249-1286):
```rust
#[derive(Debug)]
pub enum ExportCommandError {
    InvalidOutputPath(String),
    Engine(String),
    RenderGraph(String),
    Compile(FfmpegCompileError),
    Runtime(FfmpegRuntimeError),
    Validation(OutputValidationError),
    Scheduler(String),
    UnknownJob(String),
    Io(String),
}

impl fmt::Display for ExportCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOutputPath(message)
            | Self::Engine(message)
            | Self::RenderGraph(message)
            | Self::Scheduler(message)
            | Self::UnknownJob(message)
            | Self::Io(message) => write!(formatter, "{message}"),
            Self::Compile(error) => write!(formatter, "export compile failed: {}", error.message),
            Self::Runtime(error) => write!(formatter, "export runtime failed: {}", error.message),
            Self::Validation(error) => {
                write!(formatter, "export validation failed: {}", error.message)
            }
        }
    }
}

pub fn export_error_diagnostic(error: &ExportCommandError) -> ExportDiagnostic {
    match error {
        ExportCommandError::InvalidOutputPath(message) => ExportDiagnostic {
            kind: ExportDiagnosticKind::InvalidOutputPath,
            message: message.clone(),
            stdout_summary: None,
            stderr_summary: None,
        },
        ExportCommandError::Engine(message) => ExportDiagnostic {
            kind: ExportDiagnosticKind::EngineFailed,
            message: message.clone(),
            stdout_summary: None,
            stderr_summary: None,
        },
        ExportCommandError::Runtime(error) => export_runtime_diagnostic(error),
        ExportCommandError::Validation(error) => export_validation_diagnostic(error),
        ExportCommandError::Scheduler(message)
        | ExportCommandError::UnknownJob(message)
        | ExportCommandError::Io(message) => ExportDiagnostic {
            kind: ExportDiagnosticKind::RuntimeFailed,
            message: message.clone(),
            stdout_summary: None,
            stderr_summary: None,
        },
    }
}
```

**Copy rule:** `editor_runtime` errors should be typed and serializable. Adapters convert them to JSON, C status codes, or CLI output.

---

### `crates/bindings_node/src/lib.rs` (adapter/controller, request-response)

**Analog:** current `crates/bindings_node/src/lib.rs`

**N-API function wrapper pattern** (lines 188-246):
```rust
#[napi(js_name = "openProjectSession")]
pub fn open_project_session(request: serde_json::Value) -> Result<serde_json::Value> {
    project_session_service::open_project_session(request)
}

#[napi(js_name = "createProjectSession")]
pub fn create_project_session(request: serde_json::Value) -> Result<serde_json::Value> {
    project_session_service::create_project_session(request)
}

#[napi(js_name = "closeProjectSession")]
pub fn close_project_session(request: serde_json::Value) -> Result<serde_json::Value> {
    project_session_service::close_project_session(request)
}

#[napi(js_name = "executeProjectIntent")]
pub fn execute_project_intent(request: serde_json::Value) -> Result<serde_json::Value> {
    project_session_service::execute_project_intent(request)
}

#[napi(js_name = "beginProjectInteraction")]
pub fn begin_project_interaction(request: serde_json::Value) -> Result<serde_json::Value> {
    project_session_service::begin_project_interaction(request)
}

#[napi(js_name = "startProjectSessionExport")]
pub fn start_project_session_export(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = match serde_json::from_value::<StartProjectSessionExportRequest>(request) {
        Ok(request) => request,
        Err(error) => {
            return to_js_value(error_envelope(
                CommandErrorKind::InvalidPayload,
                format!("Invalid startProjectSessionExport payload: {error}"),
                Some("startProjectSessionExport".to_string()),
            ));
        }
    };
    start_project_session_export_command(request)
}
```

**Parse helper pattern** (lines 732-747):
```rust
fn parse_binding_payload<T>(
    command_label: &'static str,
    request: serde_json::Value,
) -> std::result::Result<T, CommandResultEnvelope<serde_json::Value>>
where
    T: serde::de::DeserializeOwned,
{
    match serde_json::from_value::<T>(request) {
        Ok(payload) => Ok(payload),
        Err(error) => Err(error_envelope(
            CommandErrorKind::InvalidPayload,
            format!("Invalid {command_label} payload: {error}"),
            Some(command_label.to_string()),
        )),
    }
}
```

**Copy rule:** After Phase 18, these functions should deserialize Node JSON, call `editor_runtime`, and serialize the shared response. They should not own registries, scheduler policy, export preparation, `.veproj` semantics, or handle metadata.

---

### `crates/bindings_node/src/project_session_service.rs` (adapter/service, request-response)

**Analog:** current file, but as a source for what to move out

**Imports showing current over-ownership** (lines 1-59):
```rust
use adapter_kaipai::{KaipaiFormulaBundle, KaipaiImportOptions, map_kaipai_bundle_to_import_plan};
use artifact_store::{
    ArtifactStoreError,
    resource_index::{
        ResourceKind, ResourceRef, index_draft_resources, index_draft_resources_with_extra_refs,
    },
};
use draft_commands::delta::material_dependency_delta;
use draft_import::{
    DraftImportApplicationInput, LocalizedResourceIndexKind, LocalizedResourceManifest,
    LocalizedResourceStatus, apply_import_plan_to_draft,
};
use draft_model::{ /* draft, command, timeline, interaction types */ };
use media_runtime::{discover_runtime_config, run_scheduled_material_probe};
use media_runtime_desktop::DesktopFfmpegExecutor;
use project_store::{
    ProjectStoreError, StdPlatformFileSystem, create_project_bundle, open_project_bundle,
    project_io_scheduler_envelope, save_project_bundle,
};
use task_runtime::{
    CompletionFreshness, JobDomain, JobEnvelope, JobFreshness, JobId, JobPriority, JobResult,
    PlaybackGeneration, ResourceClass, TaskCancellationToken, TaskRuntimeConfig,
};
```

**Planning interpretation:** These imports prove the current Node crate owns project IO, scheduled material probe, import, command execution, and interactions. Phase 18 should move these imports and service logic into `editor_runtime`, leaving `bindings_node` with request parsing and adapter calls.

---

### `crates/bindings_node/src/preview_export_service.rs` (adapter/service, batch)

**Analog:** current file, but export service should move below adapters

**Global registry to replace with shared runtime ownership** (lines 971-974):
```rust
pub fn global_export_registry() -> &'static SchedulerExportService {
    static REGISTRY: OnceLock<SchedulerExportService> = OnceLock::new();
    REGISTRY.get_or_init(SchedulerExportService::new)
}
```

**Output validation/error pattern to preserve** (lines 1110-1128, 1219-1247):
```rust
fn validate_output_path(value: &str) -> Result<PathBuf, ExportCommandError> {
    if value.trim().is_empty() {
        return Err(ExportCommandError::InvalidOutputPath(
            "export output path must not be empty".to_owned(),
        ));
    }
    let path = PathBuf::from(value);
    if path.extension().and_then(|extension| extension.to_str()) != Some("mp4") {
        return Err(ExportCommandError::InvalidOutputPath(
            "export output path must end with .mp4".to_owned(),
        ));
    }
    if path.parent().is_none() {
        return Err(ExportCommandError::InvalidOutputPath(
            "export output path must include a parent directory".to_owned(),
        ));
    }
    Ok(path)
}

fn export_runtime_diagnostic(error: &FfmpegRuntimeError) -> ExportDiagnostic {
    ExportDiagnostic {
        kind: match error.kind {
            media_runtime::FfmpegRuntimeErrorKind::RuntimeUnavailable
            | media_runtime::FfmpegRuntimeErrorKind::ProcessLaunchFailed => {
                ExportDiagnosticKind::RuntimeUnavailable
            }
            media_runtime::FfmpegRuntimeErrorKind::Timeout
            | media_runtime::FfmpegRuntimeErrorKind::NonZeroExit
            | media_runtime::FfmpegRuntimeErrorKind::MissingEncoder
            | media_runtime::FfmpegRuntimeErrorKind::MissingFilter
            | media_runtime::FfmpegRuntimeErrorKind::MalformedProgress => {
                ExportDiagnosticKind::RuntimeFailed
            }
        },
        message: error.message.clone(),
        stdout_summary: error.stdout_summary.clone(),
        stderr_summary: error.stderr_summary.clone(),
    }
}
```

**Copy rule:** Keep the export preparation, status, progress, cancellation, validation, and diagnostics behavior. Change ownership: `bindings_node` delegates to `editor_runtime::ExportService`.

---

### `apps/desktop-electron/src/main/nativeBinding.ts` (adapter, request-response)

**Analog:** current file

**Typed native binding shape** (lines 62-143):
```typescript
type NativeBinding = {
  ping: () => CommandResultEnvelope<PingResponse>;
  version: () => CommandResultEnvelope<VersionResponse>;
  configureBundledRuntimeDirectory: (directory: string) => void;
  probeMediaRuntime: () => CommandResultEnvelope<RuntimeConfigResponse>;
  probeRuntimeCapabilities: () => CommandResultEnvelope<RuntimeCapabilityReport>;
  createProjectSession: (request: CreateProjectSessionRequest) => CommandResultEnvelope<ProjectSessionOpenResponse>;
  openProjectSession: (request: OpenProjectSessionRequest) => CommandResultEnvelope<ProjectSessionOpenResponse>;
  closeProjectSession: (request: ProjectSessionRequest) => CommandResultEnvelope<ProjectSessionClosedResponse>;
  executeProjectIntent: (request: ExecuteProjectIntentRequest) => CommandResultEnvelope<ProjectSessionIntentResponse>;
  beginProjectInteraction: (
    request: BeginProjectInteractionRequest
  ) => CommandResultEnvelope<ProjectInteractionBeginResponse>;
  updateProjectInteraction: (
    request: UpdateProjectInteractionRequest
  ) => CommandResultEnvelope<ProjectInteractionUpdateResponse>;
  commitProjectInteraction: (
    request: CommitProjectInteractionRequest
  ) => CommandResultEnvelope<ProjectInteractionCommitResponse>;
  cancelProjectInteraction: (
    request: CancelProjectInteractionRequest
  ) => CommandResultEnvelope<ProjectInteractionCancelResponse>;
  startProjectSessionExport: (request: StartProjectSessionExportRequest) => CommandResultEnvelope<ExportJobStatusResponse>;
  getExportJobStatus: (request: ExportJobRequest) => CommandResultEnvelope<ExportJobStatusResponse>;
  cancelExport: (request: ExportJobRequest) => CommandResultEnvelope<ExportJobStatusResponse>;
};
```

**Required function loading pattern** (lines 1364-1497):
```typescript
function loadNativeBinding(): NativeBinding | null {
  if (cachedBinding !== undefined) {
    return cachedBinding;
  }

  const bindingPath = resolveNativeBindingPath();
  try {
    const loaded = requireNative(bindingPath) as Partial<NativeBinding>;
    if (
      typeof loaded.ping !== "function" ||
      typeof loaded.version !== "function" ||
      typeof loaded.configureBundledRuntimeDirectory !== "function" ||
      typeof loaded.probeMediaRuntime !== "function" ||
      typeof loaded.probeRuntimeCapabilities !== "function" ||
      typeof loaded.createProjectSession !== "function" ||
      typeof loaded.openProjectSession !== "function" ||
      typeof loaded.closeProjectSession !== "function" ||
      typeof loaded.executeProjectIntent !== "function" ||
      typeof loaded.beginProjectInteraction !== "function" ||
      typeof loaded.updateProjectInteraction !== "function" ||
      typeof loaded.commitProjectInteraction !== "function" ||
      typeof loaded.cancelProjectInteraction !== "function" ||
      typeof loaded.startProjectSessionExport !== "function" ||
      typeof loaded.getExportJobStatus !== "function" ||
      typeof loaded.cancelExport !== "function"
    ) {
      throw new Error("Native binding does not expose the required editor and realtime preview functions");
    }

    cachedBinding = {
      ping: loaded.ping,
      version: loaded.version,
      configureBundledRuntimeDirectory: loaded.configureBundledRuntimeDirectory,
      probeMediaRuntime: loaded.probeMediaRuntime,
      probeRuntimeCapabilities: loaded.probeRuntimeCapabilities,
      createProjectSession: loaded.createProjectSession,
      openProjectSession: loaded.openProjectSession,
      closeProjectSession: loaded.closeProjectSession,
      executeProjectIntent: loaded.executeProjectIntent,
      beginProjectInteraction: loaded.beginProjectInteraction,
      updateProjectInteraction: loaded.updateProjectInteraction,
      commitProjectInteraction: loaded.commitProjectInteraction,
      cancelProjectInteraction: loaded.cancelProjectInteraction,
      startProjectSessionExport: loaded.startProjectSessionExport,
      getExportJobStatus: loaded.getExportJobStatus,
      cancelExport: loaded.cancelExport,
    };
    cachedLoadError = null;
    return cachedBinding;
  } catch (error) {
    cachedBinding = null;
    cachedLoadError = boundErrorMessage(error);
    return null;
  }
}
```

**Copy rule:** Update expected functions when Node adapter changes, but keep this file as a desktop transport adapter. It must not construct render/export behavior.

---

### `apps/desktop-electron/src/main/index.ts` and `apps/desktop-electron/src/preload/index.ts` (controller/middleware, request-response)

**Analogs:** current main/preload files

**Main IPC route pattern** (`apps/desktop-electron/src/main/index.ts` lines 253-360):
```typescript
ipcMain.handle("core:createProjectSession", (event, request: CreateProjectSessionRequest) => {
  assertAllowedIpcSender(event);
  recordTestProjectSessionCall("createProjectSession", request);
  return createProjectSession(request);
});

ipcMain.handle("core:openProjectSession", (event, request: OpenProjectSessionRequest) => {
  assertAllowedIpcSender(event);
  recordTestProjectSessionCall("openProjectSession", request);
  return openProjectSession(request);
});

ipcMain.handle("core:startProjectSessionExport", (event, request: StartProjectSessionExportRequest) => {
  assertAllowedIpcSender(event);
  recordTestProjectSessionCall("startProjectSessionExport", request);
  const testExportResponse = maybeBuildTestProjectSessionExportResponse(request);
  if (testExportResponse !== null) {
    return testExportResponse;
  }
  return startProjectSessionExport(request);
});
```

**IPC sender guard pattern** (lines 837-858):
```typescript
function assertAllowedIpcSender(event: IpcMainInvokeEvent): void {
  const senderUrl = event.senderFrame.url;
  if (!isAllowedRendererUrl(senderUrl)) {
    throw new Error(`Rejected IPC from untrusted renderer: ${senderUrl}`);
  }
}

function isAllowedRendererUrl(targetUrl: string): boolean {
  try {
    const target = new URL(targetUrl);
    const allowed = new URL(allowedRendererUrl);

    if (isDevelopment && devServerUrl !== undefined) {
      return target.origin === allowed.origin;
    }

    return target.protocol === "file:" && target.host === allowed.host && target.pathname === allowed.pathname;
  } catch {
    return false;
  }
}
```

**Preload exposure pattern** (`apps/desktop-electron/src/preload/index.ts` lines 57-116):
```typescript
if (allowedRendererUrl !== undefined && isAllowedRendererLocation(window.location.href, allowedRendererUrl)) {
  contextBridge.exposeInMainWorld("videoEditorCore", {
    ping: () => ipcRenderer.invoke("core:ping"),
    version: () => ipcRenderer.invoke("core:version"),
    probeMediaRuntime: () => ipcRenderer.invoke("core:probeMediaRuntime"),
    probeRuntimeCapabilities: () => ipcRenderer.invoke("core:probeRuntimeCapabilities"),
    createProjectSession: (request: CreateProjectSessionRequest) => ipcRenderer.invoke("core:createProjectSession", request),
    openProjectSession: (request: OpenProjectSessionRequest) => ipcRenderer.invoke("core:openProjectSession", request),
    executeProjectIntent: (request: ExecuteProjectIntentRequest) => ipcRenderer.invoke("core:executeProjectIntent", request),
    beginProjectInteraction: (request: BeginProjectInteractionRequest) =>
      ipcRenderer.invoke("core:beginProjectInteraction", request),
    updateProjectInteraction: (request: UpdateProjectInteractionRequest) =>
      ipcRenderer.invoke("core:updateProjectInteraction", request),
    commitProjectInteraction: (request: CommitProjectInteractionRequest) =>
      ipcRenderer.invoke("core:commitProjectInteraction", request),
    cancelProjectInteraction: (request: CancelProjectInteractionRequest) =>
      ipcRenderer.invoke("core:cancelProjectInteraction", request),
    startProjectSessionExport: (request: StartProjectSessionExportRequest) =>
      ipcRenderer.invoke("core:startProjectSessionExport", request),
    getExportJobStatus: (request: ExportJobRequest) => ipcRenderer.invoke("core:getExportJobStatus", request),
    cancelExport: (request: ExportJobRequest) => ipcRenderer.invoke("core:cancelExport", request),
    closeProjectSession: (request: ProjectSessionRequest) => ipcRenderer.invoke("core:closeProjectSession", request)
  });
}
```

**Sanitizer pattern** (`apps/desktop-electron/src/preload/index.ts` lines 224-255):
```typescript
function sanitizeHostRect(rect: RealtimePreviewHostRect): RealtimePreviewHostRect {
  return {
    x: finiteRounded(rect.x),
    y: finiteRounded(rect.y),
    width: finiteRounded(rect.width),
    height: finiteRounded(rect.height),
    scaleFactorMillis: finiteRounded(rect.scaleFactorMillis)
  };
}

function sanitizeTargetTimeMicroseconds(value: number): number {
  return Number.isFinite(value) ? Math.max(0, Math.round(value)) : 0;
}

function sanitizeProjectSessionId(value: string): string {
  return typeof value === "string" ? value : "";
}
```

**Guard required:** Main currently has test mock paths such as `maybeBuildTestRuntimeCapabilitiesResponse` and `VIDEO_EDITOR_TEST_MOCK_EXPORT_COMMANDS` (`apps/desktop-electron/src/main/index.ts` lines 1457-1557). Phase 18 source guards must ensure mock paths cannot satisfy server/runtime/binding product success.

---

### `crates/bindings_c/src/lib.rs` (adapter, request-response + FFI)

**Analog:** no exact C ABI analog exists. Use `bindings_node/src/lib.rs` for adapter thinness and `media_runtime/src/frame.rs` for handle/status modeling.

**Status and error shape to mirror from existing typed errors** (`crates/media_runtime/src/frame.rs` lines 138-160):
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FramePoolErrorKind {
    LeaseNotFound,
    OwnerSessionMismatch,
    LeaseLimitExceeded,
}

#[derive(Debug, Clone, PartialEq, Eq, Error, Serialize, Deserialize)]
#[error("frame pool failed: {message}")]
#[serde(rename_all = "camelCase")]
pub struct FramePoolError {
    pub kind: FramePoolErrorKind,
    pub message: String,
}
```

**Adapter parse/delegate rule from Node** (`crates/bindings_node/src/lib.rs` lines 245-258):
```rust
#[napi(js_name = "startProjectSessionExport")]
pub fn start_project_session_export(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = match serde_json::from_value::<StartProjectSessionExportRequest>(request) {
        Ok(request) => request,
        Err(error) => {
            return to_js_value(error_envelope(
                CommandErrorKind::InvalidPayload,
                format!("Invalid startProjectSessionExport payload: {error}"),
                Some("startProjectSessionExport".to_string()),
            ));
        }
    };
    start_project_session_export_command(request)
}
```

**C ABI implementation guidance:** Export `#[repr(C)]` handles/status/error codes, convert C strings/buffers into typed `editor_runtime` requests, call the shared runtime, write bounded UTF-8 JSON or typed output structs, and provide explicit release functions. Do not depend on `bindings_node` or `napi`.

---

### `crates/bindings_c/tests/abi_smoke.rs` (test, FFI request-response)

**Analog:** `crates/bindings_node/tests/binding_smoke.rs` and `crates/media_runtime/tests/frame_pool.rs`

**Standard envelope smoke pattern** (`crates/bindings_node/tests/binding_smoke.rs` lines 12-20, 60-76):
```rust
#[test]
fn ping_returns_standard_ok_envelope() {
    let envelope = ping().expect("ping returns a JSON envelope");

    assert_eq!(envelope["ok"], true);
    assert_eq!(envelope["data"], json!({ "pong": true }));
    assert_eq!(envelope["error"], Value::Null);
    assert_eq!(envelope["events"], json!([]));
}

#[test]
fn execute_command_rejects_unknown_command_with_structured_error() {
    let envelope = execute_command(json!({
        "command": "renderTimeline",
        "payload": { "kind": "renderTimeline" },
        "requestId": "req-render-timeline"
    }))
    .expect("unsupported command returns an error envelope");

    assert_eq!(envelope["ok"], false);
    assert_eq!(envelope["data"], Value::Null);
    assert_eq!(envelope["error"]["kind"], serde_json::to_value(CommandErrorKind::UnsupportedCommand).unwrap());
    assert_eq!(envelope["events"], json!([]));
}
```

**Handle smoke pattern** (`crates/media_runtime/tests/frame_pool.rs` lines 34-105, 168-191):
```rust
#[test]
fn frame_pool_session_close_releases_unreleased_frame_and_texture_handles_with_leak_diagnostics() {
    let mut pool = FramePool::new(
        MediaSessionId("session-1".to_owned()),
        FramePoolLimits { max_outstanding_leases: 4 },
    );
    let cpu = pool.acquire_video_frame(cpu_request(Some(5), color)).expect("CPU frame lease should be acquired");
    let texture = pool.acquire_video_frame(FrameLeaseRequest { /* texture handle with owner/generation */ })
        .expect("texture frame lease should be acquired");

    let report = pool.close_session();

    assert_eq!(pool.outstanding_lease_count(), 0);
    assert_eq!(report.owner_session, MediaSessionId("session-1".to_owned()));
    assert_eq!(report.leak_diagnostics.len(), 2);
    assert!(report.leak_diagnostics.iter().any(|leak| leak.frame_handle_id == cpu.handle_id));
    assert!(report.leak_diagnostics.iter().any(|leak| leak.frame_handle_id == texture.handle_id));
}

#[test]
fn frame_pool_rejects_release_from_wrong_owner_session() {
    let error = pool
        .release_for_session(&MediaSessionId("foreign-session".to_owned()), frame.release.clone())
        .expect_err("foreign session must not release a lease it does not own");

    assert_eq!(error.kind, FramePoolErrorKind::OwnerSessionMismatch);
    assert_eq!(pool.outstanding_lease_count(), 1);
}
```

**Copy rule:** C smoke tests must call exported ABI functions, verify status codes, error buffers/required lengths, handle release, wrong-owner/stale/double-release failures, and session-close leak diagnostics.

---

### `crates/server_runtime/src/main.rs` and `crates/server_runtime/tests/server_export_smoke.rs` (controller/test, batch + file-I/O)

**Analog:** `crates/testkit/tests/render_smoke.rs`, `crates/bindings_node/tests/export_commands.rs`, `crates/bindings_node/tests/scheduler_export.rs`

**Tiny render smoke evidence pattern** (`crates/testkit/tests/render_smoke.rs` lines 5-14):
```rust
#[test]
fn render_smoke_asserts_generated_output_metadata() {
    let smoke = run_tiny_render_smoke().expect(
        "ffmpeg and ffprobe must be available in the bundled runtime directory; run pnpm --dir apps/desktop-electron run provision:ffmpeg-runtime",
    );

    assert!(smoke.output_path().is_file());
    assert_tiny_smoke_metadata(smoke.metadata())
        .expect("tiny render smoke metadata should match the Phase 1 harness contract");
}
```

**Export start/progress/validation smoke pattern** (`crates/bindings_node/tests/export_commands.rs` lines 19-58):
```rust
#[test]
fn export_commands_start_status_and_complete_through_binding_registry() {
    let sandbox = Sandbox::new("export-complete");
    let _ffmpeg = sandbox.ffmpeg_complete();
    let _ffprobe = sandbox.ffprobe_success(1_920, 1_080, true);
    let _runtime_dir = RuntimeDirectoryGuard::set(&sandbox.root);
    let output = sandbox.root.join("导出.mp4");

    let started = execute_command(json!({
        "command": "startExport",
        "payload": {
            "kind": "startExport",
            "draft": export_draft("draft-export-complete"),
            "outputPath": output,
            "preset": ExportPreset::H264AacBalanced
        },
        "requestId": "req-export-start"
    }))
    .expect("start export should return envelope");

    assert_eq!(started["ok"], true, "{started:#}");
    assert_eq!(started["data"]["phase"], "running");
    assert_eq!(started["data"]["progressPerMille"], 0);

    let completed = wait_for_export_phase(&job_id, ExportJobPhase::Completed);
    assert_eq!(completed["data"]["phase"], "completed");
    assert_eq!(completed["data"]["progressPerMille"], 1000);
    assert_eq!(completed["data"]["validation"]["width"], 1_920);
    assert_eq!(completed["data"]["validation"]["height"], 1_080);
    assert_eq!(completed["data"]["validation"]["hasAudio"], true);
}
```

**Scheduler telemetry/cancel smoke pattern** (`crates/bindings_node/tests/scheduler_export.rs` lines 50-124, 126-202):
```rust
#[test]
fn scheduler_export_start_status_completion_and_validation_report_scheduler_telemetry() {
    let started = execute_command(json!({ "command": "startExport", "payload": { /* ... */ } }))
        .expect("start export should return envelope");

    assert_eq!(started["data"]["scheduler"]["domain"], "export");
    assert_eq!(started["data"]["scheduler"]["resourceClass"], "ffmpegProcess");
    assert_eq!(started["data"]["scheduler"]["validationResourceClass"], "validationProbe");

    let completed = wait_for_export_phase(&job_id, ExportJobPhase::Completed);
    assert_eq!(completed["data"]["phase"], "completed");
    assert_eq!(completed["data"]["scheduler"]["jobId"], job_id);
    assert!(completed["data"]["scheduler"]["completedCount"].as_u64().unwrap_or_default() >= 2);
}

#[test]
fn scheduler_export_cancel_queued_job_without_running_ffmpeg() {
    let first = start_export("draft-scheduler-export-running", &first_output);
    let queued = start_export("draft-scheduler-export-queued", &queued_output);
    assert_eq!(queued["data"]["phase"], "queued", "{queued:#}");

    let cancelled = cancel_export(json!({ "jobId": queued_job_id }))
        .expect("explicit queued cancel should return envelope");
    assert_eq!(cancelled["data"]["phase"], "cancelled");
    assert_eq!(cancelled["data"]["diagnostic"]["kind"], "cancelled");

    let runs = sandbox.ffmpeg_runs();
    assert!(
        !runs.contains(&queued_output.display().to_string()),
        "queued export should be cancelled before FFmpeg starts; runs={runs:#?}"
    );
}
```

**Project-bundle fixture pattern** (`crates/project_store/tests/project_bundle.rs` lines 11-25, 181-200):
```rust
#[test]
fn create_project_bundle_writes_valid_project_json() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("created.veproj");
    let draft = Draft::new("draft-001", "Created draft");

    let bundle = create_project_bundle(&StdPlatformFileSystem, &bundle_path, &draft)
        .expect("bundle should be created");
    let opened = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("created bundle should open");

    assert_eq!(bundle.project_json_path, project_json_path(&bundle_path));
    assert_eq!(opened.bundle.draft, draft);
    assert!(opened.warnings.is_empty());
}

#[test]
fn open_project_bundle_preserves_missing_material_entries() {
    save_project_bundle(&StdPlatformFileSystem, &bundle_path, &draft)
        .expect("draft with missing material should save");
    let opened = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("missing material path should not block open");

    assert_eq!(opened.bundle.draft, draft);
    assert_eq!(opened.warnings, vec![ProjectStoreWarning::MissingMaterial { /* ... */ }]);
}
```

**Copy rule:** Server smoke must use real `.veproj` fixtures and assert output file/metadata/progress/cancel. A metadata-only CLI is not a valid analog.

---

### `docs/mobile-runtime-contracts.md` and `docs/runtime-boundaries.md` (docs, lifecycle/boundary)

**Analog:** `docs/runtime-boundaries.md`, `docs/no-product-fallback-policy.md`, `docs/refactor-and-legacy-cleanup-policy.md`

**Boundary doc pattern** (`docs/runtime-boundaries.md` lines 8-21):
```markdown
## Trait Placement

Platform traits live at the consuming service boundary:

- `media_runtime::FfmpegExecutor` owns the FFmpeg and ffprobe process execution
  boundary.
- `project_store::PlatformFileSystem` owns filesystem access for `.veproj`
  project bundle persistence.
- `preview_service::PreviewRenderer` reserves the future preview rendering
  boundary for frames, segments, thumbnails, waveform cache, and invalidation.

There is no generic `platform` crate. Electron, future iOS, future Android, and
future server backends should inject implementations at the app shell or service
boundary rather than leaking platform traits into semantic crates.
```

**Ownership map pattern** (`docs/runtime-boundaries.md` lines 114-124):
```markdown
| Layer | Owns | Does Not Own |
|-------|------|--------------|
| Electron renderer | UI controls, DOM measurement, preview host rectangle reporting, Chinese telemetry display | FFmpeg commands, render graphs, GPU devices, GPU command lists, cache keys, dirty ranges, fallback selection, timeline mutation, keyframe evaluation |
| Electron main/preload | `BrowserWindow.getNativeWindowHandle()` acquisition, safe IPC routing, integer host bounds forwarding | Preview composition semantics, fallback decisions, graph interpretation |
| `bindings_node` | Thin JSON/Node-API route and type mapping, opaque session IDs | GPU rendering logic, native handle exposure to renderer, timeline math |
| `realtime_preview_runtime` | `TimelineClock`, `PlaybackGeneration`, sessions, `wgpu` device/surface/offscreen targets, compositor, diagnostics, telemetry | Draft command mutation, FFmpeg export compilation, hardware decode, audio output, priority scheduling |
| `preview_service` | Supported realtime routing and frame provider/cache boundaries | Renderer UI, primary GPU composition internals, export behavior decisions |
| `engine_core` / `render_graph` | Accepted draft normalization, integer-microsecond frame state, renderer-neutral graph intent | `wgpu`, OS handles, FFmpeg process execution |
```

**No-fallback doc rule** (`docs/no-product-fallback-policy.md` lines 8-17, 24-34):
```markdown
Normal product behavior must not report success through fallback output. If the
production implementation for a supported path is unavailable, the feature must
fail closed with a clear unavailable diagnostic instead of silently switching to
an approximate, mock, debug, artifact, CPU, or legacy path.

When a fallback path already exists and can be exercised by normal users, remove
or gate that path before replacing it with the production implementation. Do not
leave the fallback active as a temporary product behavior.

The product path must not use any of these as proof that playback works:

- mock realtime backends or synthetic frame tokens
- preview PNG/frame requests during playback
- preview artifact or FFmpeg artifact frames
- FFmpeg CPU decode probes or decoded-frame fingerprints
- offscreen/CPU readback evidence standing in for the visible compositor
```

**Refactor doc rule** (`docs/refactor-and-legacy-cleanup-policy.md` lines 7-15):
```markdown
This project is a greenfield editor. When a product path is being upgraded, do
not over-preserve compatibility with incomplete, legacy, mock, fallback, or
temporary implementations. Replace the path with the intended current
architecture, and remove or gate obsolete code that would let normal users keep
using the old behavior.

Compatibility layers are allowed only for explicit external formats or platform
capability reports. They must be named as adapters, diagnostics, or unsupported
reports, not as product success paths.
```

**Copy rule:** `docs/mobile-runtime-contracts.md` should use ownership tables and lifecycle sections for Android JNI, Swift/ObjC, C ABI, file permissions, background/foreground, cancellation, explicit release, session close, texture/device identity, and degraded diagnostics.

---

### `scripts/phase18-source-guards.sh` and `scripts/phase18-abi-drift.sh` (test/guard, batch)

**Analog:** `scripts/phase17-1-source-guards.sh`, `scripts/phase16-source-guards.sh`, `scripts/no-product-fallback-guards.sh`

**Guard helper pattern** (`scripts/phase17-1-source-guards.sh` lines 1-45):
```bash
#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "phase17.1 source guard violation: $1" >&2
  exit 1
}

require_file() {
  local file="$1"
  [ -f "$file" ] || fail "missing required file ${file}"
}

require_fixed() {
  local file="$1"
  local text="$2"
  if ! rg -n --fixed-strings "$text" "$file" >/dev/null; then
    fail "missing required text '${text}' in ${file}"
  fi
}

matches_for_pattern() {
  local pattern="$1"
  shift
  rg -n --pcre2 "$pattern" "$@" 2>/dev/null | strip_comments
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

**Negative self-test pattern** (`scripts/phase17-1-source-guards.sh` lines 47-64, 107-122):
```bash
assert_pattern_rejects() {
  local description="$1"
  local pattern="$2"
  local source="$3"
  local tmp_dir
  tmp_dir="$(mktemp -d)"
  trap 'rm -rf "$tmp_dir"' RETURN
  printf '%s\n' "$source" >"$tmp_dir/InjectedPhase171Violation.tsx"
  if [ -z "$(matches_for_pattern "$pattern" "$tmp_dir/InjectedPhase171Violation.tsx" || true)" ]; then
    fail "negative check did not catch injected ${description}"
  fi
  printf '%s\n' "// $source" >"$tmp_dir/CommentOnly.tsx"
  if [ -n "$(matches_for_pattern "$pattern" "$tmp_dir/CommentOnly.tsx" || true)" ]; then
    fail "comment-filtered negative check matched comment-only ${description}"
  fi
}

assert_pattern_rejects \
  "high-frequency canonical timeline intent loop" \
  "$HIGH_FREQUENCY_CANONICAL_INTENT_PATTERN" \
  'function handleDragMove() { executeProjectTimelineIntent({ kind: "moveSelectedSegmentIntent" }, "拖拽"); }'
```

**Scheduler/source-ownership guard pattern** (`scripts/phase16-source-guards.sh` lines 162-205):
```bash
fail_matches \
  "realtime preview binding must not keep legacy worker maps or idle poll cadence" \
  '\b(?:still_frame_workers|playback_workers|rt-preview-still|rt-preview-playback|REALTIME_PLAYBACK_IDLE_POLL_INTERVAL|presentPlaybackTick|schedulerCompositedEvidence)\b' \
  "$BINDINGS_DIR/realtime_preview_service.rs"

fail_matches \
  "export binding must not reintroduce a binding-owned export registry or raw export thread" \
  '\b(?:ExportJobRegistry|run_export_thread|export_thread_registry|thread::spawn[[:space:]]*\([[:space:]]*move[[:space:]]*\|\|[^{]*run_export)\b' \
  "$BINDINGS_DIR/preview_export_service.rs"

fail_matches \
  "renderer and preload must not mutate scheduler capacities, queue policy, priority, freshness, retry, fallback, or resource budgets" \
  "$RENDERER_POLICY_MUTATION_PATTERN" \
  "$RENDERER_DIR" "$PRELOAD_FILE" \
  --glob '!commandHelpers.ts'
```

**No-product-fallback guard pattern** (`scripts/no-product-fallback-guards.sh` lines 15-33, 88-109):
```bash
fail_if_matches \
  "Electron realtime preview host must not request decoded/FFmpeg content evidence or expose mock/fallback playback displays" \
  'requestRealtimePreviewContentEvidence|shouldCollectContentEvidence|requestContentEvidence|mockFrameDisplay|VIDEO_EDITOR_TEST_EXPOSE_MOCK_FRAME_DISPLAY|VIDEO_EDITOR_TEST_MOCK_REALTIME_PREVIEW_FFMPEG_FALLBACK|requestFallbackFrame|ffmpegArtifactGenerated' \
  apps/desktop-electron/src/main/realtimePreviewHost.ts

SCHEDULER_STRESS_SPEC="apps/desktop-electron/tests/product-scheduler-stress.spec.ts"
if [ -f "$SCHEDULER_STRESS_SPEC" ]; then
  fail_if_matches \
    "Product scheduler stress success must not be satisfied by test runtime/export/artifact/audio mocks" \
    'VIDEO_EDITOR_TEST_MOCK_EXPORT_COMMANDS|VIDEO_EDITOR_TEST_MOCK_ARTIFACT_COMMANDS|VIDEO_EDITOR_TEST_MOCK_AUDIO_COMMANDS|VIDEO_EDITOR_TEST_MOCK_RUNTIME_CAPABILITIES:\s*"1"|mockSchedulerSuccess|artifactSchedulerSuccess|cpuProbeSchedulerSuccess|domOnlySchedulerSuccess' \
    "$SCHEDULER_STRESS_SPEC"
fi
```

**Phase 18 guard targets:** fail if:
- `bindings_c` depends on `bindings_node`
- `server_runtime` depends on Electron, preload, DOM, or `apps/desktop-electron`
- adapters contain duplicate project/export/session semantics instead of `editor_runtime`
- renderer/main constructs render graph, FFmpeg jobs, or export behavior
- product success uses mock/fallback/artifact/CPU evidence
- C header drift is unchecked

**ABI drift script:** no exact repo analog. Use the guard helper style above plus research's cbindgen gate:
```bash
cbindgen --config crates/bindings_c/cbindgen.toml --crate bindings_c --output crates/bindings_c/include/video_editor_runtime.h
git diff --exit-code crates/bindings_c/include/video_editor_runtime.h
```

---

### `package.json` (config, batch)

**Analog:** current phase aggregate scripts

**Aggregate script pattern** (`package.json` lines 84-93, 103-104):
```json
"test:phase16-rust": "cargo test -p task_runtime -- --nocapture && cargo test -p bindings_node --test scheduler_preview_audio -- --nocapture && cargo test -p bindings_node --test scheduler_export -- --nocapture && cargo test -p bindings_node --test scheduler_artifact_probe -- --nocapture && cargo test -p bindings_node --test scheduler_runtime -- --nocapture",
"test:phase16-source-guards": "bash scripts/phase16-source-guards.sh",
"test:phase16-desktop": "pnpm --filter @video-editor/desktop package:dir && pnpm --filter @video-editor/desktop exec playwright test tests/product-scheduler-stress.spec.ts --reporter=line",
"test:phase16": "pnpm run test:phase16-rust && pnpm run test:phase16-source-guards && pnpm run test:no-product-fallback && pnpm --filter @video-editor/desktop test:runtime-diagnostics && pnpm run test:phase16-desktop && pnpm run test:contracts",
"test:phase17-1:guards": "bash scripts/phase17-1-source-guards.sh",
"test:phase17-1:desktop": "pnpm --filter @video-editor/desktop package:dir && pnpm --filter @video-editor/desktop exec playwright test tests/interaction-preview-inspector.spec.ts tests/interaction-timeline-keyframe.spec.ts tests/template-import.spec.ts tests/ui-regression.spec.ts --reporter=line --workers=1",
"test:phase17-1:rust": "cargo test -p draft_model interaction -- --nocapture && cargo test -p bindings_node --test project_interaction_session -- --nocapture && cargo test -p task_runtime scheduler_contracts_full_preview_queue_coalesces_obsolete_jobs -- --nocapture && cargo test -p task_runtime scheduler_contracts_stale_completion_does_not_commit_visible_state -- --nocapture && cargo test -p task_runtime starvation_interactive_preview_audio_and_analysis_start_under_background_pressure -- --nocapture && cargo test -p realtime_preview_runtime media_io_handoff_rejects_stale_generation_after_decode_and_counts_telemetry -- --nocapture",
"test:phase17-1": "pnpm run test:phase17-1:desktop && pnpm run test:phase17-1:guards && pnpm run test:phase17-1:rust && cargo check --workspace --locked && pnpm run test:contracts",
"test:contracts": "git diff --exit-code schemas apps/desktop-electron/src/generated",
"test": "pnpm run test:rust && pnpm run test:schema && ..."
```

**Copy rule:** Add `test:phase18-rust`, `test:phase18-source-guards`, `test:phase18-abi`, `test:phase18-server`, and `test:phase18` in the same style. The aggregate should include C ABI smoke, server export smoke, Node adapter smoke, header drift, source guards, `test:no-product-fallback`, `cargo check --workspace --locked`, and `test:contracts`.

## Shared Patterns

### Thin Adapter Boundary
**Source:** `crates/bindings_node/src/lib.rs` lines 188-246 and `apps/desktop-electron/src/main/nativeBinding.ts` lines 1364-1497  
**Apply to:** `bindings_node`, `bindings_c`, server CLI, Electron main/preload.

Adapters parse transport values, verify shape, call shared Rust runtime, and serialize transport-specific responses. They do not own draft, project lifecycle, scheduler, export, preview, or handle metadata.

### Rust-Owned Handles
**Source:** `crates/media_runtime/src/frame.rs` lines 163-291 and `crates/media_runtime/src/texture.rs` lines 95-249  
**Apply to:** `editor_runtime/src/handles.rs`, `bindings_c`, `bindings_node`, server runtime.

Copy owner-session/generation validation, explicit release, lease limits, cascading close, and leak diagnostics.

### Scheduler And Cancellation
**Source:** `crates/task_runtime/src/scheduler.rs` lines 134-350  
**Apply to:** shared export service, server runtime jobs, adapter smoke tests.

Use `JobEnvelope`, `TaskCancellationToken`, `ResourceClass`, telemetry snapshots, and stale/cancelled completion rejection. Do not spawn ad hoc unbounded export/server threads.

### `.veproj` Source Of Truth
**Source:** `crates/project_store/src/bundle.rs` lines 22-127 and `docs/runtime-boundaries.md` lines 75-99  
**Apply to:** shared project session service and server export.

Open/save through `project_store`; keep `.veproj/project.json` canonical; keep render graphs, scripts, previews, proxies, caches, and outputs derived.

### No Product Fallback
**Source:** `docs/no-product-fallback-policy.md` lines 8-17, 24-34 and `scripts/no-product-fallback-guards.sh` lines 15-33  
**Apply to:** low-copy handles, server export, preview evidence, phase guards.

Fallback/mock/artifact/CPU paths may exist as diagnostics or test utilities only. They cannot satisfy Phase 18 product success.

### Source Guard Style
**Source:** `scripts/phase17-1-source-guards.sh` lines 1-64 and `scripts/phase16-source-guards.sh` lines 162-205  
**Apply to:** `scripts/phase18-source-guards.sh`, `scripts/phase18-abi-drift.sh`.

Use `rg`, comment filtering, negative self-tests, required-file checks, required-text checks, and explicit forbidden-pattern scans.

## No Analog Found

Files with no close codebase analog; planner should use RESEARCH.md official-source patterns plus the guard style above.

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `crates/bindings_c/cbindgen.toml` | config | batch | No C ABI/header-generation config exists in the repo. Use pinned `cbindgen` guidance from RESEARCH.md. |
| `crates/bindings_c/include/video_editor_runtime.h` | generated interface | request-response + FFI | No generated C header exists. It should be produced by `cbindgen` and protected by drift checks. |

## Metadata

**Analog search scope:** `crates/**`, `apps/desktop-electron/src/**`, `scripts/**`, `docs/**`, root manifests, Phase 17.1 summaries.  
**Files scanned:** 18 crate manifests, Rust source/test files under `crates`, 18 guard scripts, desktop main/preload/native binding files, current docs, root `Cargo.toml`, root `package.json`.  
**Pattern extraction date:** 2026-06-25
