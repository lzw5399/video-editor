use std::error::Error;
use std::fmt;
use std::path::{Path, PathBuf};

use draft_model::{
    Draft, DraftValidationError, Material, MaterialId, MaterialKind, MaterialMetadata,
    MaterialStatus, Microseconds, RationalFrameRate, mark_material_available,
    mark_material_missing, mark_material_probe_failed, upsert_material,
};
use media_runtime::{
    FfmpegExecutor, MaterialProbeError, MaterialProbeErrorKind, MaterialProbeKind,
    MaterialProbeMetadata, RuntimeConfig, probe_material_metadata,
};
use project_store::{
    MaterialUriKind, PlatformFileSystem, ProjectBundle, ProjectStoreError, classify_material_uri,
    material_uri_for_save, save_project_bundle,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportMaterialRequest {
    pub material_id: Option<MaterialId>,
    pub path: PathBuf,
    pub display_name: Option<String>,
    pub material_kind_hint: Option<MaterialKind>,
}

impl ImportMaterialRequest {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            material_id: None,
            path: path.into(),
            display_name: None,
            material_kind_hint: None,
        }
    }

    pub fn with_material_id(mut self, material_id: impl Into<MaterialId>) -> Self {
        self.material_id = Some(material_id.into());
        self
    }

    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = Some(display_name.into());
        self
    }

    pub fn with_material_kind_hint(mut self, kind: MaterialKind) -> Self {
        self.material_kind_hint = Some(kind);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MaterialImportResult {
    pub material: Material,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<MissingMaterialDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SavedMaterialImport {
    pub draft: Draft,
    pub material: Material,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostic: Option<MissingMaterialDiagnostic>,
    pub bundle_path: PathBuf,
    pub project_json_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MissingMaterialDiagnostic {
    pub material_id: MaterialId,
    pub kind: MissingMaterialDiagnosticKind,
    pub original_uri: String,
    pub last_known_resolved_path: Option<PathBuf>,
    pub status: MaterialStatus,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MissingMaterialDiagnosticKind {
    MissingFile,
    MarkedMissing,
    ProbeFailed,
    UnresolvedExternalUri,
}

#[derive(Debug)]
pub enum MaterialServiceError {
    ProjectStore(ProjectStoreError),
    Draft(DraftValidationError),
}

impl fmt::Display for MaterialServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProjectStore(error) => write!(formatter, "{error}"),
            Self::Draft(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for MaterialServiceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ProjectStore(error) => Some(error),
            Self::Draft(error) => Some(error),
        }
    }
}

impl From<ProjectStoreError> for MaterialServiceError {
    fn from(error: ProjectStoreError) -> Self {
        Self::ProjectStore(error)
    }
}

impl From<DraftValidationError> for MaterialServiceError {
    fn from(error: DraftValidationError) -> Self {
        Self::Draft(error)
    }
}

pub fn import_material(
    draft: &mut Draft,
    request: ImportMaterialRequest,
    fs: &impl PlatformFileSystem,
    executor: &impl FfmpegExecutor,
    runtime: &RuntimeConfig,
    bundle_path: impl AsRef<Path>,
) -> Result<MaterialImportResult, MaterialServiceError> {
    let bundle_path = bundle_path.as_ref();
    let uri = material_uri_for_save(bundle_path, &request.path)?;
    let material_id = request
        .material_id
        .clone()
        .unwrap_or_else(|| deterministic_material_id(&uri));
    let display_name = request
        .display_name
        .clone()
        .unwrap_or_else(|| display_name_for_path(&request.path, &uri));

    if !fs.exists(&request.path) {
        let material = recoverable_material(
            material_id,
            request.material_kind_hint.unwrap_or(MaterialKind::Video),
            uri,
            display_name,
            MaterialStatus::Missing,
            Some(format!(
                "material path does not exist: {}",
                request.path.display()
            )),
        );
        upsert_material(draft, material.clone())?;
        mark_material_missing(
            draft,
            &material.material_id,
            material.metadata.probe_error.clone().unwrap_or_default(),
        )?;
        return Ok(MaterialImportResult {
            diagnostic: Some(diagnostic_for_material(
                fs,
                bundle_path,
                &material,
                MissingMaterialDiagnosticKind::MissingFile,
            )?),
            material,
        });
    }

    match probe_material_metadata(executor, runtime, &request.path) {
        Ok(metadata) => {
            let material = material_from_probe(material_id, uri, display_name, metadata);
            upsert_material(draft, material.clone())?;
            mark_material_available(draft, &material.material_id)?;
            Ok(MaterialImportResult {
                material,
                diagnostic: None,
            })
        }
        Err(error) => {
            let status = if error.kind == MaterialProbeErrorKind::MissingInput {
                MaterialStatus::Missing
            } else {
                MaterialStatus::ProbeFailed
            };
            let diagnostic_kind = if status == MaterialStatus::Missing {
                MissingMaterialDiagnosticKind::MissingFile
            } else {
                MissingMaterialDiagnosticKind::ProbeFailed
            };
            let material = recoverable_material(
                material_id,
                request.material_kind_hint.unwrap_or(MaterialKind::Video),
                uri,
                display_name,
                status,
                Some(error.message.clone()),
            );

            add_or_update_recoverable_material(draft, material.clone(), &error)?;
            Ok(MaterialImportResult {
                diagnostic: Some(diagnostic_for_material(
                    fs,
                    bundle_path,
                    &material,
                    diagnostic_kind,
                )?),
                material,
            })
        }
    }
}

pub fn import_material_and_save(
    draft: &mut Draft,
    request: ImportMaterialRequest,
    fs: &impl PlatformFileSystem,
    executor: &impl FfmpegExecutor,
    runtime: &RuntimeConfig,
    bundle_path: impl AsRef<Path>,
) -> Result<SavedMaterialImport, MaterialServiceError> {
    let bundle_path = bundle_path.as_ref();
    let imported = import_material(draft, request, fs, executor, runtime, bundle_path)?;
    let bundle = save_project_bundle(fs, bundle_path, draft)?;

    Ok(saved_import(imported, bundle))
}

pub fn list_materials(draft: &Draft) -> Vec<Material> {
    draft.materials.clone()
}

pub fn list_missing_materials(
    draft: &Draft,
    fs: &impl PlatformFileSystem,
    bundle_path: impl AsRef<Path>,
) -> Result<Vec<MissingMaterialDiagnostic>, MaterialServiceError> {
    let bundle_path = bundle_path.as_ref();
    let mut diagnostics = Vec::new();

    for material in &draft.materials {
        if material.status == MaterialStatus::ProbeFailed {
            diagnostics.push(diagnostic_for_material(
                fs,
                bundle_path,
                material,
                MissingMaterialDiagnosticKind::ProbeFailed,
            )?);
            continue;
        }

        let classified = classify_material_uri(bundle_path, &material.uri)?;
        match (material.status, classified.resolved_path) {
            (MaterialStatus::Missing, Some(_)) => diagnostics.push(diagnostic_for_material(
                fs,
                bundle_path,
                material,
                MissingMaterialDiagnosticKind::MarkedMissing,
            )?),
            (MaterialStatus::Missing, None) => diagnostics.push(diagnostic_for_material(
                fs,
                bundle_path,
                material,
                MissingMaterialDiagnosticKind::UnresolvedExternalUri,
            )?),
            (_, Some(resolved_path)) if !fs.exists(&resolved_path) => {
                diagnostics.push(diagnostic_for_material(
                    fs,
                    bundle_path,
                    material,
                    MissingMaterialDiagnosticKind::MissingFile,
                )?);
            }
            _ => {}
        }
    }

    Ok(diagnostics)
}

fn material_from_probe(
    material_id: MaterialId,
    uri: String,
    display_name: String,
    metadata: MaterialProbeMetadata,
) -> Material {
    Material {
        material_id,
        kind: material_kind_from_probe(metadata.kind),
        uri,
        display_name,
        metadata: MaterialMetadata {
            duration: metadata.duration_microseconds.map(Microseconds::new),
            width: metadata.width,
            height: metadata.height,
            frame_rate: metadata.frame_rate.map(|frame_rate| RationalFrameRate {
                numerator: frame_rate.numerator,
                denominator: frame_rate.denominator,
            }),
            has_video: metadata.has_video_stream,
            has_audio: metadata.has_audio_stream,
            audio_sample_rate: metadata.audio.map(|audio| audio.sample_rate),
            audio_channels: metadata.audio.map(|audio| audio.channels),
            probe_error: None,
        },
        status: MaterialStatus::Available,
    }
}

fn recoverable_material(
    material_id: MaterialId,
    kind: MaterialKind,
    uri: String,
    display_name: String,
    status: MaterialStatus,
    probe_error: Option<String>,
) -> Material {
    let mut metadata = MaterialMetadata::empty();
    metadata.probe_error = probe_error;
    Material {
        material_id,
        kind,
        uri,
        display_name,
        metadata,
        status,
    }
}

fn add_or_update_recoverable_material(
    draft: &mut Draft,
    material: Material,
    error: &MaterialProbeError,
) -> Result<(), MaterialServiceError> {
    let material_id = material.material_id.clone();
    let status = material.status;
    let message = error.message.clone();
    upsert_material(draft, material)?;

    match status {
        MaterialStatus::Available => mark_material_available(draft, &material_id)?,
        MaterialStatus::Missing => mark_material_missing(draft, &material_id, message)?,
        MaterialStatus::ProbeFailed => mark_material_probe_failed(draft, &material_id, message)?,
    }

    Ok(())
}

fn saved_import(imported: MaterialImportResult, bundle: ProjectBundle) -> SavedMaterialImport {
    SavedMaterialImport {
        draft: bundle.draft,
        material: imported.material,
        diagnostic: imported.diagnostic,
        bundle_path: bundle.bundle_path,
        project_json_path: bundle.project_json_path,
    }
}

fn material_kind_from_probe(kind: MaterialProbeKind) -> MaterialKind {
    match kind {
        MaterialProbeKind::Video => MaterialKind::Video,
        MaterialProbeKind::Image => MaterialKind::Image,
        MaterialProbeKind::Audio => MaterialKind::Audio,
    }
}

fn deterministic_material_id(uri: &str) -> MaterialId {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in uri.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }

    MaterialId::new(format!("material-{hash:016x}"))
}

fn display_name_for_path(path: &Path, uri: &str) -> String {
    path.file_name()
        .and_then(|file_name| file_name.to_str())
        .filter(|file_name| !file_name.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| uri.to_owned())
}

fn diagnostic_for_material(
    fs: &impl PlatformFileSystem,
    bundle_path: &Path,
    material: &Material,
    kind: MissingMaterialDiagnosticKind,
) -> Result<MissingMaterialDiagnostic, MaterialServiceError> {
    let classified = classify_material_uri(bundle_path, &material.uri)?;
    let last_known_resolved_path = classified.resolved_path.clone();
    let kind = match (
        kind,
        material.status,
        classified.kind,
        classified.resolved_path.as_ref(),
    ) {
        (_, _, _, Some(path)) if !fs.exists(path) => MissingMaterialDiagnosticKind::MissingFile,
        (MissingMaterialDiagnosticKind::MarkedMissing, MaterialStatus::Missing, _, _) => {
            MissingMaterialDiagnosticKind::MarkedMissing
        }
        (_, MaterialStatus::Missing, MaterialUriKind::ExternalUri, None) => {
            MissingMaterialDiagnosticKind::UnresolvedExternalUri
        }
        (other, _, _, _) => other,
    };

    Ok(MissingMaterialDiagnostic {
        material_id: material.material_id.clone(),
        kind,
        original_uri: material.uri.clone(),
        last_known_resolved_path,
        status: material.status,
        message: diagnostic_message(material, kind),
    })
}

fn diagnostic_message(material: &Material, kind: MissingMaterialDiagnosticKind) -> String {
    if let Some(error) = &material.metadata.probe_error {
        return error.clone();
    }

    match kind {
        MissingMaterialDiagnosticKind::MissingFile => {
            format!(
                "material file is missing for {}",
                material.material_id.as_str()
            )
        }
        MissingMaterialDiagnosticKind::MarkedMissing => {
            format!(
                "material is marked missing: {}",
                material.material_id.as_str()
            )
        }
        MissingMaterialDiagnosticKind::ProbeFailed => {
            format!("material probe failed: {}", material.material_id.as_str())
        }
        MissingMaterialDiagnosticKind::UnresolvedExternalUri => {
            format!(
                "material URI cannot be resolved locally: {}",
                material.material_id.as_str()
            )
        }
    }
}
