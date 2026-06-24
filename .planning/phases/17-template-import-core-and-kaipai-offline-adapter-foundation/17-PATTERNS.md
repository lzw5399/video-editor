# Phase 17: Template Import Core And Kaipai Offline Adapter Foundation - Pattern Map

**Mapped:** 2026-06-24
**Files analyzed:** 31 planned files / file groups
**Analogs found:** 26 / 31

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `Cargo.toml` | config | config | `Cargo.toml` workspace members lines 1-19 | exact |
| `package.json` | config | batch | `package.json` phase test scripts lines 20-27, 84-87, 93-94 | exact |
| `scripts/phase17-source-guards.sh` | utility | batch | `scripts/phase16-source-guards.sh` | exact |
| `crates/draft_import/Cargo.toml` | config | config | existing crate manifests via workspace `Cargo.toml` lines 1-19 | role-match |
| `crates/draft_import/src/lib.rs` | provider | transform | `crates/draft_model/src/draft.rs`; old branch `adapter_kaipai/src/lib.rs` reference-only | role-match |
| `crates/draft_import/src/import_plan.rs` | model | transform | `crates/draft_model/src/draft.rs`, `timeline.rs`, `material.rs` | role-match |
| `crates/draft_import/src/adaptation_report.rs` | model | transform | old branch `adapter_kaipai/src/compatibility_report.rs` reference-only | partial |
| `crates/draft_import/src/resource_localizer.rs` | service | file-I/O | `crates/project_store/src/paths.rs`; `crates/artifact_store/src/resource_index.rs`; old branch localizer reference-only | role-match |
| `crates/draft_import/src/validation.rs` | utility | transform | `crates/draft_model/src/validation.rs` | exact |
| `crates/draft_import/tests/import_plan.rs` | test | transform | `crates/draft_model/tests/draft_fixtures.rs`, `crates/artifact_store/tests/resource_index.rs` | role-match |
| `crates/draft_import/tests/adaptation_report.rs` | test | transform | old branch `adapter_kaipai/tests/compatibility_report.rs` reference-only | partial |
| `crates/draft_import/tests/schema_exports.rs` | test | batch | `crates/draft_model/tests/schema_exports.rs`; old branch schema export test reference-only | exact |
| `schemas/draft-import-plan.schema.json` | config | transform | `schemas/draft.schema.json` via schema export pattern | generated |
| `schemas/adaptation-report.schema.json` | config | transform | old branch `schemas/compatibility-report.schema.json` reference-only | generated |
| `crates/adapter_kaipai/Cargo.toml` | config | config | old branch `crates/adapter_kaipai/Cargo.toml` reference-only; workspace `Cargo.toml` | partial |
| `crates/adapter_kaipai/src/lib.rs` | provider | transform | old branch `adapter_kaipai/src/lib.rs` reference-only | partial |
| `crates/adapter_kaipai/src/error.rs` | utility | request-response | old branch `adapter_kaipai/src/error.rs` reference-only | partial |
| `crates/adapter_kaipai/src/formula_bundle.rs` | model | transform | old branch `adapter_kaipai/src/formula_bundle.rs` reference-only | partial |
| `crates/adapter_kaipai/src/mapper.rs` | service | transform | `crates/draft_model/src/timeline.rs`, `material.rs`, `canvas.rs`; no existing adapter mapper on current main | partial |
| `crates/adapter_kaipai/tests/formula_bundle_contract.rs` | test | transform | old branch `adapter_kaipai/tests/fixtures.rs` and `schema_exports.rs` reference-only | partial |
| `crates/adapter_kaipai/tests/resource_localizer.rs` | test | file-I/O | old branch `adapter_kaipai/tests/resource_localizer.rs` reference-only | partial |
| `crates/adapter_kaipai/tests/mapper.rs` | test | transform | `crates/artifact_store/tests/resource_index.rs`; no current adapter mapper test | partial |
| `crates/adapter_kaipai/tests/schema_exports.rs` | test | batch | `crates/draft_model/tests/schema_exports.rs`; old branch schema export test reference-only | exact |
| `fixtures/kaipai/**` | test fixture | file-I/O | old branch `fixtures/kaipai/**` reference-only | partial |
| `fixtures/kaipai/expected-reports/**` | test fixture | transform | old branch expected report snapshots reference-only | partial |
| `schemas/kaipai-formula-bundle.schema.json` | config | transform | old branch schema export pattern reference-only | generated |
| `crates/bindings_node/Cargo.toml` | config | config | existing workspace crate dependency pattern | role-match |
| `crates/bindings_node/src/project_session_service.rs` | service | request-response + file-I/O | same file `execute_project_intent`, `import_material`, save path | exact |
| `crates/bindings_node/src/lib.rs` | route | request-response | same file N-API project-session exports | exact |
| `crates/bindings_node/tests/project_session_import_kaipai.rs` or `project_session.rs` | test | request-response + file-I/O | `crates/bindings_node/tests/project_session.rs` | exact |
| `crates/testkit/tests/template_import_exports.rs` | test | batch + file-I/O | `crates/testkit/tests/preview_export_parity.rs`, `render_smoke.rs` | role-match |

## Pattern Assignments

### `Cargo.toml` and New Crate Manifests (config, config)

**Apply to:** `Cargo.toml`, `crates/draft_import/Cargo.toml`, `crates/adapter_kaipai/Cargo.toml`, `crates/bindings_node/Cargo.toml`

**Analog:** `Cargo.toml`

**Workspace registration pattern** (lines 1-19):

```toml
[workspace]
members = [
  "crates/draft_model",
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
```

**Version policy pattern** (lines 22-25):

```toml
[workspace.package]
edition = "2024"
rust-version = "1.95.0"
license = "MIT"
```

Use workspace edition/rust-version/license. Add `crates/draft_import` and `crates/adapter_kaipai` as workspace members; add dependencies from `bindings_node` to the new crates only after Rust-side adapter/import tests exist.

---

### `crates/draft_import/src/import_plan.rs` (model, transform)

**Analog:** `crates/draft_model/src/draft.rs`, `crates/draft_model/src/material.rs`, `crates/draft_model/src/timeline.rs`, `crates/draft_model/src/time.rs`

**Imports/derive pattern** (draft model lines 1-5):

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{DraftCanvasConfig, DraftId, Material, Track};
```

**Strict contract pattern** (draft model lines 41-50):

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Draft {
    pub schema_version: DraftSchemaVersion,
    pub draft_id: DraftId,
    pub metadata: DraftMetadata,
    pub canvas_config: DraftCanvasConfig,
    pub materials: Vec<Material>,
    pub tracks: Vec<Track>,
}
```

**Material plan target pattern** (material lines 7-15, 85-94):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum MaterialKind {
    Video,
    Image,
    Audio,
    Text,
    Sticker,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Material {
    pub material_id: MaterialId,
    pub kind: MaterialKind,
    pub uri: String,
    pub display_name: String,
    pub metadata: MaterialMetadata,
    pub status: MaterialStatus,
}
```

**Timeline target pattern** (timeline lines 31-52, 817-839, 865-875):

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceTimerange {
    pub start: Microseconds,
    pub duration: Microseconds,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TargetTimerange {
    pub start: Microseconds,
    pub duration: Microseconds,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Segment {
    pub segment_id: SegmentId,
    pub material_id: MaterialId,
    pub source_timerange: SourceTimerange,
    pub target_timerange: TargetTimerange,
    pub main_track_magnet: MainTrackMagnet,
    pub keyframes: Vec<Keyframe>,
    pub filters: Vec<Filter>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub transition: Option<Transition>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub text: Option<TextSegment>,
    #[serde(default)]
    pub volume: SegmentVolume,
    #[serde(default)]
    pub audio: SegmentAudio,
    #[serde(default)]
    pub visual: SegmentVisual,
}
```

**Time model pattern** (time lines 5-19):

```rust
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema, TS,
)]
pub struct Microseconds(pub u64);

impl Microseconds {
    pub const ZERO: Self = Self(0);

    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn get(self) -> u64 {
        self.0
    }
}
```

`DraftImportPlan` should use canonical `DraftCanvasConfig`, `Material`, `Track`, `Segment`, `SourceTimerange`, `TargetTimerange`, `Microseconds`, `RationalFrameRate`, `SegmentTransform`, `SegmentAudio`, `TextSegment`, and `Keyframe` rather than provider-specific structs. Do not store `templateId`, raw formula JSON, safe-area provider evidence, Android worker IDs, or remote render URLs in the plan fields that become `.veproj/project.json`.

---

### `crates/draft_import/src/adaptation_report.rs` (model, transform)

**Analog:** old branch `origin/work/kaipai-adapter-poc:crates/adapter_kaipai/src/compatibility_report.rs` (reference-only)

**Report contract pattern** (old branch lines 19-30):

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompatibilityReport {
    pub schema_version: CompatibilityReportSchemaVersion,
    pub source_kind: String,
    pub source_id: String,
    pub generated_at: String,
    pub summary: CompatibilityReportSummary,
    pub items: Vec<CompatibilityReportItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance_digest: Option<String>,
}
```

**Summary-from-items pattern** (old branch lines 32-63):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompatibilityReportSummary {
    pub supported: u32,
    pub degraded: u32,
    pub unsupported: u32,
    pub missing_resource: u32,
    pub needs_native_effect: u32,
}

impl CompatibilityReportSummary {
    pub fn from_items(items: &[CompatibilityReportItem]) -> Self {
        let mut summary = Self {
            supported: 0,
            degraded: 0,
            unsupported: 0,
            missing_resource: 0,
            needs_native_effect: 0,
        };

        for item in items {
            match item.status {
                CompatibilityStatus::Supported => summary.supported += 1,
                CompatibilityStatus::Degraded => summary.degraded += 1,
                CompatibilityStatus::Unsupported => summary.unsupported += 1,
                CompatibilityStatus::MissingResource => summary.missing_resource += 1,
                CompatibilityStatus::NeedsNativeEffect => summary.needs_native_effect += 1,
            }
        }

        summary
    }
}
```

**Status taxonomy to evolve** (old branch lines 82-90):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum CompatibilityStatus {
    Supported,
    Degraded,
    Unsupported,
    MissingResource,
    NeedsNativeEffect,
}
```

For Phase 17, rename/evolve this to provider-neutral `AdaptationReport`. Required statuses are `supported`, `approximated`, `dropped`, `missingResource`, and `needsNativeEffect`. Map old `Degraded` to `Approximated`; map old `Unsupported` to `Dropped` only when the mapper intentionally omits unsupported semantics. Keep `external_path`, `external_id`, and provenance as evidence only.

**Native effect handling pattern** (old branch lines 354-369):

```rust
fn native_effect_item(path: &str, effect: &Value) -> CompatibilityReportItem {
    let external_id = effect
        .get("nativeEffectId")
        .or_else(|| effect.get("name"))
        .and_then(Value::as_str)
        .unwrap_or("nativeEffect");
    CompatibilityReportItem {
        status: CompatibilityStatus::NeedsNativeEffect,
        severity: CompatibilitySeverity::Warning,
        category: CompatibilityCategory::NativeEffect,
        external_path: path.to_owned(),
        external_id: Some(external_id.to_owned()),
        canonical_target: None,
        message: "Provider-native effect requires explicit compatibility handling before it can be represented locally.".to_owned(),
        details: Some("Do not smuggle native effect data into generic filter parameters.".to_owned()),
    }
}
```

Do not copy the old names blindly; copy the shape and tests, then make the report provider-neutral.

---

### `crates/draft_import/src/resource_localizer.rs` (service, file-I/O)

**Analogs:** `crates/project_store/src/paths.rs`, `crates/artifact_store/src/resource_index.rs`, old branch localizer reference-only

**Bundle URI classification pattern** (`project_store/src/paths.rs` lines 25-57):

```rust
pub fn classify_material_uri(
    bundle_path: impl AsRef<Path>,
    uri: &str,
) -> Result<MaterialUri, ProjectStoreError> {
    let trimmed = uri.trim();
    if trimmed.is_empty() {
        return invalid_uri(uri, "URI must not be empty");
    }

    let path = Path::new(trimmed);
    if is_absolute_material_path(trimmed, path) {
        return Ok(MaterialUri {
            kind: MaterialUriKind::ExternalAbsolute,
            uri: trimmed.to_owned(),
            resolved_path: Some(path.to_path_buf()),
        });
    }

    if has_uri_scheme(trimmed) {
        return Ok(MaterialUri {
            kind: MaterialUriKind::ExternalUri,
            uri: trimmed.to_owned(),
            resolved_path: None,
        });
    }

    validate_bundle_relative_path(trimmed)?;
    Ok(MaterialUri {
        kind: MaterialUriKind::InBundleRelative,
        uri: trimmed.to_owned(),
        resolved_path: Some(bundle_path.as_ref().join(path)),
    })
}
```

**Traversal rejection pattern** (`project_store/src/paths.rs` lines 88-108):

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

**Resource index hook pattern** (`artifact_store/src/resource_index.rs` lines 148-187):

```rust
pub fn index_draft_resources(
    bundle_path: impl AsRef<Path>,
    draft: &Draft,
) -> Result<ResourceIndex, ArtifactStoreError> {
    let bundle_path = bundle_path.as_ref();
    let store = open_artifact_store(bundle_path)?;
    let mut index = ResourceIndex::default();

    for material in &draft.materials {
        let resource_ref = resource_ref_for_material(material.material_id.as_str());
        let classified = classify_material_uri(bundle_path, &material.uri).map_err(|source| {
            ArtifactStoreError::InvalidResourceRef {
                resource_id: resource_ref.resource_id.as_str().to_owned(),
                reason: source.to_string(),
            }
        })?;
        let project_relative_ref = match classified.kind {
            MaterialUriKind::InBundleRelative => Some(classified.uri),
            MaterialUriKind::ExternalAbsolute | MaterialUriKind::ExternalUri => None,
        };
        let resource = IndexedResource {
            resource_id: resource_ref.resource_id.clone(),
            kind: ResourceKind::Material,
            stable_key: resource_ref.stable_key,
            parent_material_id: Some(material.material_id.clone()),
            source_ref: Some(material.uri.clone()),
            project_relative_ref,
            status: resource_status_from_material(material.status),
        };
        upsert_indexed_resource(&mut index, resource)?;
    }

    for track in &draft.tracks {
        for segment in &track.segments {
            index_segment_resources(&mut index, segment)?;
        }
    }

    persist_resource_index(store.connection(), &index, 0)?;
    Ok(index)
}
```

**Old branch localizer status shape, reference-only** (old branch lines 53-61):

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum LocalizedResourceStatus {
    Available,
    Missing,
    Sha256Mismatch,
    UnsafePath,
    RemoteRenderUrl,
}
```

**Old branch localize-before-map pattern, reference-only** (old branch lines 70-106):

```rust
impl ResourceLocalizer {
    pub fn localize(
        &self,
        request: ResourceLocalizationRequest,
    ) -> Result<ResourceLocalizationResult, AdapterKaipaiError> {
        let mut resources = Vec::new();
        let mut diagnostics = Vec::new();
        let mut seen_destinations = BTreeSet::new();
        let canonical_source_root = canonicalize_existing_dir(&request.source_root)?;
        let canonical_bundle_path = match request.mode {
            ResourceLocalizationMode::CopyRenderableResources
            | ResourceLocalizationMode::ReferenceExistingBundleResources => {
                Some(canonicalize_existing_dir(&request.bundle_path)?)
            }
            ResourceLocalizationMode::PreserveExternalSourceMedia => None,
        };

        for (index, resource) in request.resources.iter().enumerate() {
            let localized = localize_resource(
                &request,
                &canonical_source_root,
                canonical_bundle_path.as_deref(),
                &mut seen_destinations,
                resource,
                index,
            )?;
            if localized.status != LocalizedResourceStatus::Available {
                diagnostics.push(missing_resource_diagnostic(resource, index, &localized));
            }
            resources.push(localized);
        }

        Ok(ResourceLocalizationResult {
            manifest: LocalizedResourceManifest { resources },
            diagnostics,
        })
    }
}
```

Use current-main path and resource-index helpers as the implementation base. Preserve old security cases, but do not copy old hand-rolled SHA-256 code from old branch lines 584-715; research explicitly approved `sha2::Sha256`.

---

### `crates/adapter_kaipai/src/formula_bundle.rs` (model, transform)

**Analog:** old branch `adapter_kaipai/src/formula_bundle.rs` (reference-only)

**Strict offline input contract pattern** (old branch lines 30-42):

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct KaipaiFormulaBundle {
    pub schema_version: FormulaBundleSchemaVersion,
    pub kind: FormulaBundleKind,
    pub provenance: FormulaProvenance,
    pub source_media: FormulaSourceMedia,
    pub recognizer_result: RecognizerResult,
    pub safe_area: SafeAreaEvidence,
    pub direct_materials: Vec<DirectMaterialRef>,
    pub formula: Value,
    pub resources: Vec<FormulaResourceRef>,
}
```

**Parse + validate pattern** (old branch lines 44-62):

```rust
impl KaipaiFormulaBundle {
    pub fn from_json_str(json: &str) -> Result<Self, AdapterKaipaiError> {
        let bundle: Self =
            serde_json::from_str(json).map_err(|source| AdapterKaipaiError::InvalidBundleJson {
                message: source.to_string(),
            })?;
        bundle.validate()?;
        Ok(bundle)
    }

    pub fn from_json_value(value: Value) -> Result<Self, AdapterKaipaiError> {
        let bundle: Self = serde_json::from_value(value).map_err(|source| {
            AdapterKaipaiError::InvalidBundleJson {
                message: source.to_string(),
            }
        })?;
        bundle.validate()?;
        Ok(bundle)
    }
}
```

**Unsafe evidence rejection pattern** (old branch lines 282-295, 298-333):

```rust
fn reject_unsafe_external_reference(path: &str, value: &str) -> Result<(), AdapterKaipaiError> {
    if looks_like_signed_url(value) {
        return Err(AdapterKaipaiError::UnsafeFormulaEvidence {
            path: path.to_owned(),
            reason: "signed URLs are not allowed in formula evidence",
        });
    }
    if looks_like_remote_url(value) {
        return Err(AdapterKaipaiError::UnsafeFormulaEvidence {
            path: path.to_owned(),
            reason: "remote resource references are not allowed in sanitized formula bundles",
        });
    }
    Ok(())
}

fn reject_unsafe_formula_evidence(value: &Value, path: &str) -> Result<(), AdapterKaipaiError> {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                let child_path = format!("{path}.{key}");
                if is_credential_like_key(key) {
                    return Err(AdapterKaipaiError::UnsafeFormulaEvidence {
                        path: child_path,
                        reason: "credential-like fields are not allowed in formula evidence",
                    });
                }
                reject_unsafe_formula_evidence(child, &child_path)?;
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                reject_unsafe_formula_evidence(child, &format!("{path}[{index}]"))?;
            }
        }
        Value::String(text) if looks_like_remote_url(text) => {
            return Err(AdapterKaipaiError::UnsafeFormulaEvidence {
                path: path.to_owned(),
                reason: "remote URLs are not allowed in formula evidence",
            });
        }
        Value::String(text) if looks_like_signed_url(text) => {
            return Err(AdapterKaipaiError::UnsafeFormulaEvidence {
                path: path.to_owned(),
                reason: "signed URLs are not allowed in formula evidence",
            });
        }
        _ => {}
    }

    Ok(())
}
```

Keep this adapter-specific parser in `adapter_kaipai`; no core/render/session crate should import or interpret raw formula JSON.

---

### `crates/adapter_kaipai/src/mapper.rs` (service, transform)

**Analog:** current canonical semantics in `draft_model`; no current adapter mapper exists.

**Canvas target pattern** (`draft_model/src/canvas.rs` lines 20-30, 36-45):

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DraftCanvasConfig {
    pub aspect_ratio: CanvasAspectRatio,
    pub width: u32,
    pub height: u32,
    pub frame_rate: RationalFrameRate,
    pub background: CanvasBackground,
    #[serde(default)]
    pub adaptation_policy: CanvasAdaptationPolicy,
}

impl DraftCanvasConfig {
    pub fn mvp_default() -> Self {
        Self {
            aspect_ratio: CanvasAspectRatio::preset(CanvasAspectRatioPreset::Ratio16x9),
            width: Self::DEFAULT_WIDTH,
            height: Self::DEFAULT_HEIGHT,
            frame_rate: RationalFrameRate::new(30, 1),
            background: CanvasBackground::Black,
            adaptation_policy: CanvasAdaptationPolicy::Auto,
        }
    }
}
```

**Transform/text/audio target pattern** (`timeline.rs` lines 214-220, 267-287, 499-546, 694-714, 787-807):

```rust
pub struct TextFont {
    pub family: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub font_ref: Option<String>,
}

pub struct TextStyle {
    #[serde(default)]
    pub font: TextFont,
    pub font_size: u32,
    pub color: String,
    pub alignment: TextAlignment,
    #[serde(default = "default_text_line_height_millis")]
    pub line_height_millis: u32,
    #[serde(default)]
    pub letter_spacing_millis: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub stroke: Option<TextStroke>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub shadow: Option<TextShadow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub background: Option<TextBackground>,
}

pub struct SegmentAudio {
    pub gain_millis: u32,
    pub pan_balance_millis: AudioPanBalance,
    pub fade_in_duration: AudioFade,
    pub fade_out_duration: AudioFade,
    pub effect_slots: Vec<AudioEffectSlot>,
}

pub struct SegmentTransform {
    pub position: SegmentPosition,
    pub scale: SegmentScale,
    pub rotation: SegmentRotation,
    pub opacity: SegmentOpacity,
    pub crop: SegmentCrop,
    pub anchor: SegmentAnchor,
}

pub struct SegmentVisual {
    pub visible: bool,
    pub transform: SegmentTransform,
    pub fit_mode: SegmentFitMode,
    pub background_filling: SegmentBackgroundFilling,
    pub blend_mode: SegmentBlendMode,
    pub mask: SegmentMask,
}
```

Map Kaipai `level` to generic track ordering/z-order, main video/PIP/stickers to material-backed tracks/segments, text sticker fields to canonical `TextSegment`, BGM to `SegmentAudio`, and simple animation to canonical `Keyframe`s. If rotation export parity is not implemented in the same phase wave, emit an `approximated`/degraded report entry instead of claiming support.

---

### `crates/bindings_node/src/project_session_service.rs` (service, request-response + file-I/O)

**Analog:** same file

**Request struct pattern** (lines 101-107):

```rust
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ExecuteProjectIntentRequest {
    session_id: String,
    expected_revision: u64,
    intent: ProjectIntent,
}
```

**Command boundary + error envelope pattern** (lines 651-664):

```rust
pub fn execute_project_intent(request: serde_json::Value) -> Result<serde_json::Value> {
    let request = match serde_json::from_value::<ExecuteProjectIntentRequest>(request) {
        Ok(request) => request,
        Err(error) => {
            return crate::to_js_value(crate::error_envelope(
                CommandErrorKind::InvalidPayload,
                format!("Invalid executeProjectIntent payload: {error}"),
                Some("executeProjectIntent".to_string()),
            ));
        }
    };

    with_project_session_registry(|registry| registry.execute_intent(request))
}
```

**Stale revision guard** (lines 1243-1263):

```rust
fn execute_intent(
    &mut self,
    request: ExecuteProjectIntentRequest,
) -> Result<serde_json::Value> {
    let Some(session) = self.sessions.get_mut(&request.session_id) else {
        return crate::to_js_value(crate::error_envelope(
            CommandErrorKind::InvalidProject,
            format!("Project session not found: {}", request.session_id),
            Some("executeProjectIntent".to_string()),
        ));
    };
    if request.expected_revision != session.revision {
        return crate::to_js_value(crate::error_envelope(
            CommandErrorKind::InvalidPayload,
            format!(
                "Stale project session revision: expected {}, current {}",
                request.expected_revision, session.revision
            ),
            Some("executeProjectIntent".to_string()),
        ));
    }
```

**Save + revision mutation pattern** (lines 2143-2171):

```rust
let fs = StdPlatformFileSystem;
let saved = match run_project_io_job("timeline-save", self.revision, || {
    save_project_bundle(&fs, &self.bundle_path, &response.draft)
}) {
    Ok(saved) => saved,
    Err(error) => {
        return project_session_store_error("executeProjectIntent", error);
    }
};
self.revision = self.revision.saturating_add(1);
self.draft = saved.draft;
self.bundle_path = saved.bundle_path;
self.project_json_path = saved.project_json_path;
self.command_state = response.command_state;
self.selection = response.selection;

crate::to_js_value(crate::ok_envelope(ProjectSessionIntentResponse {
    session_id: self.session_id.clone(),
    revision: self.revision,
    view_model: project_session_view_model(
        &self.draft,
        &self.command_state,
        &self.selection,
    ),
    events: response.events,
    delta: response.delta,
    bundle_path: self.bundle_path.display().to_string(),
    project_json_path: self.project_json_path.display().to_string(),
}))
```

The new offline Kaipai import API should follow this exact request/envelope/stale-revision/save/revision pattern, but it should apply a validated provider-neutral `DraftImportPlan` atomically rather than executing many renderer-sent intents.

---

### `crates/bindings_node/src/lib.rs` (route, request-response)

**Analog:** same file

**N-API forwarding pattern** (lines 188-217):

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
```

Add the narrow import command here only after Rust-side import/localization/report tests pass. Do not expose raw formula fields to renderer code.

---

### `crates/draft_import/tests/schema_exports.rs`, `crates/adapter_kaipai/tests/schema_exports.rs`, and `schemas/*.json` (test/config, batch)

**Analogs:** `crates/draft_model/tests/schema_exports.rs`; old branch schema export test reference-only

**Current-main schema export pattern** (`draft_model/tests/schema_exports.rs` lines 129-148):

```rust
#[test]
fn schema_exports_generated_contract_artifacts_from_rust() {
    let root = project_root();
    let schema_path = root.join("schemas/command.schema.json");
    let draft_schema_path = root.join("schemas/draft.schema.json");
    let generated_dir = root.join("apps/desktop-electron/src/generated");

    let schema_json = command_schema_json();
    assert_command_schema_rejects_zero_frame_rates(&schema_json);
    assert_command_schema_rejects_invalid_canvas_config(&schema_json);
    assert_command_schema_rejects_invalid_text_contracts(&schema_json);
    assert_command_schema_rejects_invalid_keyframe_contracts(&schema_json);
    assert_or_update_contract_file(&schema_path, &format!("{schema_json}\n"));

    let draft_schema_json = draft_schema_json();
    assert_draft_schema_rejects_zero_frame_rates(&draft_schema_json);
    assert_draft_schema_rejects_invalid_canvas_config(&draft_schema_json);
    assert_draft_schema_rejects_invalid_text_contracts(&draft_schema_json);
    assert_draft_schema_rejects_invalid_keyframe_contracts(&draft_schema_json);
    assert_or_update_contract_file(&draft_schema_path, &format!("{draft_schema_json}\n"));
```

**Old branch adapter schema export pattern, reference-only** (old branch `tests/schema_exports.rs` lines 23-53):

```rust
#[test]
fn schema_exports_generated_formula_bundle_contracts_from_rust() {
    let root = project_root();
    let schema_path = root.join("schemas/kaipai-formula-bundle.schema.json");
    let generated_ts_path = root.join("apps/desktop-electron/src/generated/KaipaiFormulaBundle.ts");

    let schema_json = formula_bundle_schema_json();
    assert_formula_bundle_schema_requires_evidence_fields(&schema_json);
    assert_or_update_contract_file(&schema_path, &format!("{schema_json}\n"));

    let formula_bundle_ts = formula_bundle_ts_contract();
    assert!(
        formula_bundle_ts.contains("word_list"),
        "generated TypeScript should preserve provider recognizer `word_list` evidence"
    );
    assert!(
        formula_bundle_ts.contains("safeArea"),
        "generated TypeScript should expose safeArea as adapter evidence"
    );
    assert_or_update_contract_file(generated_ts_path, &formula_bundle_ts);
}

#[test]
fn schema_exports_generated_compatibility_report_contract_from_rust() {
    let root = project_root();
    let schema_path = root.join("schemas/compatibility-report.schema.json");

    let schema_json = compatibility_report_schema_json();
    assert_compatibility_report_schema_requires_diagnostic_fields(&schema_json);
    assert_or_update_contract_file(&schema_path, &format!("{schema_json}\n"));
}
```

For Phase 17, generate `kaipai-formula-bundle.schema.json`, `draft-import-plan.schema.json`, and `adaptation-report.schema.json`. Generate TS only when a desktop UI/report surface is actually added.

---

### `fixtures/kaipai/**` and Report Snapshot Tests (test fixture, file-I/O/transform)

**Analog:** old branch fixture/report tests (reference-only)

**Fixture classification pattern** (old branch `tests/fixtures.rs` lines 18-35):

```rust
#[test]
fn formula_bundle_fixtures_are_explicitly_classified() {
    let root = project_root();
    let fixture_dir = root.join("fixtures/kaipai");

    let actual = formula_fixture_paths(&fixture_dir);
    let expected = positive_formula_fixtures()
        .iter()
        .copied()
        .chain(negative_formula_fixtures().iter().map(|(path, _, _)| *path))
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();

    assert_eq!(
        actual, expected,
        "every Kaipai formula fixture must be explicitly classified"
    );
}
```

**Unsafe fixture payload tests** (old branch `tests/fixtures.rs` lines 77-90, 100-127):

```rust
#[test]
fn formula_bundle_fixtures_reject_in_memory_unsafe_payloads() {
    let base = read_formula_fixture(
        &project_root().join("fixtures/kaipai"),
        "positive/sanitized-formula-bundle.json",
    );

    for (case_name, payload, expected_error) in [
        (
            "remote formula URL",
            patch(&base, |value| {
                value["formula"]["sourceUrl"] = json!("https://example.invalid/source.mp4");
            }),
            "unsafe Kaipai formula evidence at `formula.sourceUrl`: remote URLs are not allowed in formula evidence",
        ),
        (
            "authorization key",
            patch(&base, |value| {
                value["formula"]["Authorization"] = json!("redacted");
            }),
            "unsafe Kaipai formula evidence at `formula.Authorization`: credential-like fields are not allowed in formula evidence",
        ),
        (
            "access token key",
            patch(&base, |value| {
                value["formula"]["access_token"] = json!("redacted");
            }),
            "unsafe Kaipai formula evidence at `formula.access_token`: credential-like fields are not allowed in formula evidence",
        ),
```

**Report snapshot status coverage pattern** (old branch `tests/compatibility_report.rs` lines 89-160):

```rust
#[test]
fn compatibility_report_snapshots_cover_locked_statuses() {
    let root = project_root();
    let report_dir = root.join("fixtures/kaipai/expected-reports");
    let schema = compatibility_report_schema_validator();

    let actual = report_snapshot_paths(&report_dir);
    let expected = expected_report_snapshots()
        .iter()
        .map(|case| case.path.to_owned())
        .collect::<BTreeSet<_>>();

    assert_eq!(
        actual, expected,
        "every Kaipai compatibility report snapshot must be explicitly classified"
    );

    let mut statuses = BTreeSet::new();
    for case in expected_report_snapshots() {
        let expected_report = case.report(&root);
        let expected_json = serde_json::to_string_pretty(&expected_report)
            .expect("expected report should serialize")
            + "\n";
        let actual_json = fs::read_to_string(report_dir.join(case.path)).unwrap_or_else(|error| {
            panic!("report snapshot should be readable: {}: {error}", case.path)
        });
        assert_eq!(
            actual_json, expected_json,
            "report snapshot drifted: {}",
            case.path
        );

        let report_value = read_report_snapshot(&report_dir, case.path);
        schema.validate(&report_value).unwrap_or_else(|error| {
            panic!(
                "report snapshot should validate against generated schema: {}: {error}",
                case.path
            )
        });
        let report: CompatibilityReport = serde_json::from_value(report_value.clone())
            .unwrap_or_else(|error| {
                panic!(
                    "report snapshot should deserialize through Rust contract: {}: {error}",
                    case.path
                )
            });

        let item = report_value["items"]
            .as_array()
            .and_then(|items| items.first())
            .unwrap_or_else(|| panic!("snapshot should contain at least one item: {}", case.path));
        assert_eq!(item["status"], Value::String(case.status.to_owned()));
        assert_eq!(report.items[0].status, case.expected_status);
        statuses.insert(case.status);
    }

    assert_eq!(
        statuses,
        BTreeSet::from([
            "supported",
            "degraded",
            "unsupported",
            "missingResource",
            "needsNativeEffect",
        ])
    );
}
```

Update the report snapshots to the Phase 17 taxonomy: `supported`, `approximated`, `dropped`, `missingResource`, `needsNativeEffect`. Fixture families required by context: main video, PIP, text sticker, BGM/audio, missing resource, native effect degradation.

---

### `crates/adapter_kaipai/tests/resource_localizer.rs` (test, file-I/O)

**Analog:** old branch same test (reference-only)

**Copy local assets pattern** (old branch lines 21-63):

```rust
#[test]
fn resource_localizer_copies_local_assets_to_bundle_relative_resources() {
    let bundle = read_bundle_fixture("positive/resource-bundle-with-local-assets.json");
    let temp = temp_case_dir("positive");
    let source_root = temp.join("formula-bundle");
    let bundle_path = temp.join("localized.veproj");
    seed_local_assets(&source_root, &bundle);
    fs::create_dir_all(&bundle_path).expect("project bundle dir should create");

    let result = ResourceLocalizer::default()
        .localize(ResourceLocalizationRequest {
            bundle_path: bundle_path.clone(),
            source_root,
            resources: bundle.resources.clone(),
            mode: ResourceLocalizationMode::CopyRenderableResources,
        })
        .expect("local resources should localize");

    assert!(result.diagnostics.is_empty());
    assert_eq!(result.manifest.resources.len(), 3);
    assert!(result.manifest.resources.iter().all(|resource| {
        resource.status == LocalizedResourceStatus::Available
            && resource
                .bundle_relative_uri
                .as_deref()
                .is_some_and(|uri| uri.starts_with("resources/"))
    }));
    assert!(
        bundle_path
            .join("resources/fonts/redacted-font.ttf")
            .exists()
    );
}
```

**Traversal/remote URL rejection pattern** (old branch lines 111-172):

```rust
#[test]
fn resource_localizer_rejects_traversal_and_remote_render_urls_without_writes() {
    let traversal = read_bundle_fixture("negative/path-traversal-resource.json");
    let remote = patch(
        read_bundle_fixture("positive/resource-bundle-with-local-assets.json"),
        |value| {
            value["resources"] = json!([
                {
                    "resourceId": "remote-template-video",
                    "kind": "video",
                    "uri": "https://example.invalid/render/video.mp4",
                    "displayName": "remote-template-video.mp4"
                }
            ]);
        },
    );
    let remote_bundle: KaipaiFormulaBundle = serde_json::from_value(remote)
        .expect("remote case bypasses bundle sanitizer for localizer");

    for (bundle, expected_status, expected_id) in [
        (
            traversal,
            LocalizedResourceStatus::UnsafePath,
            "unsafe-sticker",
        ),
        (
            remote_bundle,
            LocalizedResourceStatus::RemoteRenderUrl,
            "remote-template-video",
        ),
    ] {
        let temp = temp_case_dir(expected_id);
        let source_root = temp.join("formula-bundle");
        let bundle_path = temp.join("localized.veproj");
        fs::create_dir_all(&source_root).expect("source root should create");
        seed_local_assets(&source_root, &bundle);
        fs::create_dir_all(&bundle_path).expect("project bundle dir should create");

        let result = ResourceLocalizer::default()
            .localize(ResourceLocalizationRequest {
                bundle_path: bundle_path.clone(),
                source_root,
                resources: bundle.resources.clone(),
                mode: ResourceLocalizationMode::CopyRenderableResources,
            })
            .expect("unsafe resources should report diagnostics");

        assert_eq!(result.manifest.resources[0].status, expected_status);
        assert!(result.manifest.resources[0].bundle_relative_uri.is_none());
        assert_eq!(
            result.diagnostics[0].status,
            CompatibilityStatus::MissingResource
        );
        assert_eq!(
            result.diagnostics[0].external_id.as_deref(),
            Some(expected_id)
        );
        assert!(
            !bundle_path.join("resources/stickers/escape.png").exists(),
            "unsafe traversal output must not be created"
        );
    }
}
```

Also preserve symlink escape and duplicate destination coverage from old branch lines 215-365.

---

### `crates/bindings_node/tests/project_session_import_kaipai.rs` (test, request-response + file-I/O)

**Analog:** `crates/bindings_node/tests/project_session.rs`

**Import material session test pattern** (lines 1251-1310):

```rust
let imported = execute_project_intent(json!({
    "sessionId": "test-session-import-add",
    "expectedRevision": 0,
    "intent": {
        "kind": "importMaterial",
        "materialPath": video.path().display().to_string(),
        "materialId": "session-video-material",
        "displayName": "session-video.mp4"
    }
}))
.expect("session importMaterial intent should return an envelope");
assert_eq!(imported["ok"], true, "{imported:#}");
assert_eq!(imported["data"]["revision"], 1);
assert_eq!(
    imported["data"]["material"]["materialId"],
    "session-video-material"
);
assert_eq!(imported["data"]["material"]["status"], "available");
assert_no_renderer_project_state_payload(&imported);

let reopened = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
    .expect("session import and add should save canonical project.json");
assert_eq!(reopened.bundle.draft.materials.len(), 1);
assert_eq!(reopened.bundle.draft.tracks[0].segments.len(), 1);
```

**Stale revision no-persist test pattern** (lines 1499-1541):

```rust
fn project_session_stale_revision_is_rejected_without_persisting() {
    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("session-stale.veproj");
    save_timeline_draft(&bundle_path);
    open_project_session(json!({
        "bundlePath": bundle_path.display().to_string(),
        "sessionId": "test-session-stale"
    }))
    .expect("openProjectSession should return an envelope");

    let first = execute_project_intent(json!({
        "sessionId": "test-session-stale",
        "expectedRevision": 0,
        "intent": {
            "kind": "addTimelineSegmentIntent",
            "materialId": "video-material"
        }
    }))
    .expect("first executeProjectIntent should return an envelope");
    assert_eq!(first["ok"], true, "{first:#}");
    assert_eq!(first["data"]["revision"], 1);

    let stale = execute_project_intent(json!({
        "sessionId": "test-session-stale",
        "expectedRevision": 0,
        "intent": {
            "kind": "addTimelineSegmentIntent",
            "materialId": "video-material"
        }
    }))
    .expect("stale executeProjectIntent should return an envelope");
    assert_eq!(stale["ok"], false, "{stale:#}");
    assert_eq!(stale["error"]["kind"], "invalidPayload");

    let reopened = open_project_bundle(&StdPlatformFileSystem, &bundle_path)
        .expect("stale command must not mutate project.json");
    assert_eq!(reopened.bundle.draft.tracks[0].segments.len(), 1);
}
```

For offline import, assert: report returned, revision increments once, `.veproj/project.json` contains canonical draft semantics only, resources are localized under `.veproj/resources`, and stale imports do not partially write resources or mutate draft state.

---

### `scripts/phase17-source-guards.sh` and `package.json` (utility/config, batch)

**Analogs:** `scripts/phase16-source-guards.sh`, `scripts/no-product-fallback-guards.sh`, `package.json`

**Guard helper pattern** (`phase16-source-guards.sh` lines 1-44):

```bash
#!/usr/bin/env bash
set -euo pipefail

fail() {
  echo "phase16 source guard violation: $1" >&2
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

strip_comments() {
  rg -v ':[[:space:]]*(//|/\*|\*|#)' \
    | rg -v '^\s*(//|/\*|\*|#)' \
    || true
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

**Negative injection pattern** (`phase16-source-guards.sh` lines 87-104):

```bash
assert_pattern_rejects() {
  local description="$1"
  local pattern="$2"
  local source="$3"
  local tmp_dir
  tmp_dir="$(mktemp -d)"
  trap 'rm -rf "$tmp_dir"' RETURN
  printf '%s\n' "$source" >"$tmp_dir/InjectedPhase16Violation.ts"
  if [ -z "$(matches_for_pattern "$pattern" "$tmp_dir/InjectedPhase16Violation.ts" || true)" ]; then
    fail "negative check did not catch injected ${description}"
  fi
  printf '%s\n' "// $source" >"$tmp_dir/CommentOnly.ts"
  if [ -n "$(matches_for_pattern "$pattern" "$tmp_dir/CommentOnly.ts" || true)" ]; then
    fail "comment-filtered negative check matched comment-only ${description}"
  fi
  rm -rf "$tmp_dir"
  trap - RETURN
}
```

**No-fallback product evidence pattern** (`no-product-fallback-guards.sh` lines 15-33, 83-85):

```bash
fail_if_matches \
  "Electron realtime preview host must not request decoded/FFmpeg content evidence or expose mock/fallback playback displays" \
  'requestRealtimePreviewContentEvidence|shouldCollectContentEvidence|requestContentEvidence|mockFrameDisplay|VIDEO_EDITOR_TEST_EXPOSE_MOCK_FRAME_DISPLAY|VIDEO_EDITOR_TEST_MOCK_REALTIME_PREVIEW_FFMPEG_FALLBACK|requestFallbackFrame|ffmpegArtifactGenerated' \
  apps/desktop-electron/src/main/realtimePreviewHost.ts

fail_if_matches \
  "Rust realtime preview binding must not compute FFmpeg CPU fingerprints for product playback evidence" \
  'decode_ffmpeg_cpu_frame_fingerprint|FfmpegCpuFrameFingerprintRequest|request_content_evidence|RealtimePreviewContentEvidenceSource::Decoded|RealtimePreviewContentEvidenceBindingRequest|RealtimePreviewContentEvidenceBindingResponse' \
  crates/bindings_node/src/realtime_preview_service.rs crates/bindings_node/src/lib.rs

if ! rg -q 'renderGraphGpuComposited' apps/desktop-electron/tests/product-user-journey.spec.ts apps/desktop-electron/tests/helpers/userJourney.ts; then
  echo "no-product-fallback violation: product playback must require renderGraphGpuComposited evidence" >&2
  exit 1
fi
```

**Package script pattern** (`package.json` lines 84-87, 93):

```json
"test:phase16-rust": "cargo test -p task_runtime -- --nocapture && cargo test -p bindings_node --test scheduler_preview_audio -- --nocapture && cargo test -p bindings_node --test scheduler_export -- --nocapture && cargo test -p bindings_node --test scheduler_artifact_probe -- --nocapture && cargo test -p bindings_node --test scheduler_runtime -- --nocapture",
"test:phase16-source-guards": "bash scripts/phase16-source-guards.sh",
"test:phase16-desktop": "pnpm --filter @video-editor/desktop package:dir && pnpm --filter @video-editor/desktop exec playwright test tests/product-scheduler-stress.spec.ts --reporter=line",
"test:phase16": "pnpm run test:phase16-rust && pnpm run test:phase16-source-guards && pnpm run test:no-product-fallback && pnpm --filter @video-editor/desktop test:runtime-diagnostics && pnpm run test:phase16-desktop && pnpm run test:contracts",
"test:contracts": "git diff --exit-code schemas apps/desktop-electron/src/generated",
```

Phase 17 guard must prove:
- Core/render/preview/export/session crates do not import `adapter_kaipai` or interpret raw Kaipai formula JSON.
- `.veproj/project.json`, schemas, generated draft types, and renderer code do not require remote template URLs or raw formula data.
- Android worker/live provider/API/oracle artifacts are not product success evidence.
- `scripts/no-product-fallback-guards.sh` remains wired into the aggregate phase gate.

## Shared Patterns

### Canonical Draft Validation
**Source:** `crates/draft_model/src/validation.rs`
**Apply to:** `DraftImportPlan` validation, project-session import application, mapper tests

Use `validate_draft` after building the canonical draft. Copy its discipline of checking IDs, required fields, timeranges, keyframes, text, audio, and visual transforms before save. Relevant current-main lines: validation lines 115-220 for global draft validation; lines 305-328 for canvas validation; lines 408-430 for keyframe validation; lines 919-977 for visual transform validation.

### Project Store Save Boundary
**Source:** `crates/project_store/src/bundle.rs`
**Apply to:** project-session import API only

```rust
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

Source lines: `crates/project_store/src/bundle.rs` lines 38-50. Adapters must never write `.veproj/project.json` directly.

### Font Closure
**Source:** `crates/draft_model/src/font_registry.rs`
**Apply to:** text sticker import, font resource localization, report entries

```rust
pub const BUNDLED_TEXT_FONT_REF: &str = "font://bundled/noto-sans-cjk-sc-regular";
pub const BUNDLED_TEXT_FONT_FAMILY: &str = "Noto Sans CJK SC";

pub fn resolve_bundled_font(font_ref: &str) -> Option<&'static BundledFontRegistryEntry> {
    BUNDLED_FONTS
        .iter()
        .find(|entry| entry.font_ref == font_ref)
}
```

Source lines: `font_registry.rs` lines 6-11 and 103-107. Imported text should use local `fontRef` where available, bundled fallback where not, and report missing/localization failures.

### Resource Indexing
**Source:** `crates/artifact_store/src/resource_index.rs`
**Apply to:** localized media/font/effect refs

```rust
pub fn resource_ref_for_font(font_ref: impl AsRef<str>) -> ResourceRef {
    let font_ref = font_ref.as_ref();
    ResourceRef::new(
        ResourceKind::Font,
        format!("font:{font_ref}"),
        font_ref.to_owned(),
    )
}

fn index_text_resources(
    index: &mut ResourceIndex,
    text: &TextSegment,
) -> Result<(), ArtifactStoreError> {
    if let Some(font_ref) = text.style.font.font_ref.as_deref() {
        upsert_resource(index, resource_ref_for_font(font_ref), Some(font_ref))?;
    }
```

Source lines: `resource_index.rs` lines 236-243 and 278-284.

### Provider Boundary
**Source:** AGENTS.md constraints and `scripts/phase16-source-guards.sh`
**Apply to:** all Phase 17 code

Keep Kaipai in `adapter_kaipai` and provider evidence/report fields. Core crates (`draft_model`, `draft_commands`, `engine_core`, `render_graph`, `ffmpeg_compiler`, `preview_service`, `realtime_preview_runtime`, `project_store`, `artifact_store`) should consume canonical draft/import/report/resource concepts only. Source guards should fail provider leakage, Android worker references, live API/provider auth, raw formula interpretation in core/render/session semantics, and fallback success evidence.

### No Product Fallback
**Source:** `scripts/no-product-fallback-guards.sh`
**Apply to:** import preview/export acceptance

Product success must use realtime render-graph GPU preview and normal export. Do not count Android oracle output, old branch artifacts, CPU readback, mock frames, fallback artifacts, or report-only UI as success evidence.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `crates/draft_import/src/import_plan.rs` | model | transform | No current provider-neutral import-plan crate exists. Use `draft_model` contract style and validation patterns. |
| `crates/draft_import/src/adaptation_report.rs` | model | transform | No current provider-neutral adaptation report exists. Old compatibility report is reference-only and must be renamed/evolved. |
| `crates/adapter_kaipai/src/mapper.rs` | service | transform | No current external draft adapter mapper exists. Use canonical `draft_model` semantics and emit `DraftImportPlan`. |
| `crates/testkit/tests/template_import_exports.rs` | test | batch + file-I/O | No template import export fixture gate exists. Compose from existing preview/export testkit patterns during planning. |
| Desktop UI/report panel files | component/route | request-response | Out of initial backend-first scope per D-40; add only after backend import/report/localization/preview/export gates are stable. |

## Metadata

**Analog search scope:** `crates/`, `apps/desktop-electron/`, `scripts/`, `schemas/`, `fixtures/`, `.planning/phases/17-*`, and old branch `origin/work/kaipai-adapter-poc` reference files.
**Files scanned:** 650 current-main files from `rg --files`; old branch adapter/fixture/schema paths from `git ls-tree`.
**Strong current-main analogs:** `draft_model`, `project_store`, `artifact_store`, `bindings_node::project_session_service`, `scripts/phase16-source-guards.sh`, `scripts/no-product-fallback-guards.sh`.
**Reference-only old branch analogs:** `adapter_kaipai` formula bundle, resource localizer, compatibility report, schema exports, fixtures, and report snapshots.
**Pattern extraction date:** 2026-06-24
