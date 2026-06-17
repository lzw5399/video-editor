use std::path::{Path, PathBuf};

use draft_model::{Draft, DraftValidationError, migrate_draft_json, validate_draft};

use crate::PlatformFileSystem;
use crate::error::{ProjectStoreError, ProjectStoreWarning};
use crate::paths::{classify_material_uri, project_json_path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectBundle {
    pub bundle_path: PathBuf,
    pub project_json_path: PathBuf,
    pub draft: Draft,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectBundleOpenResult {
    pub bundle: ProjectBundle,
    pub warnings: Vec<ProjectStoreWarning>,
}

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

pub fn autosave_project_bundle(
    fs: &impl PlatformFileSystem,
    bundle_path: impl AsRef<Path>,
    draft: &Draft,
) -> Result<ProjectBundle, ProjectStoreError> {
    save_project_bundle(fs, bundle_path, draft)
}

pub fn open_project_bundle(
    fs: &impl PlatformFileSystem,
    bundle_path: impl AsRef<Path>,
) -> Result<ProjectBundleOpenResult, ProjectStoreError> {
    let bundle_path = bundle_path.as_ref();
    let project_json_path = project_json_path(bundle_path);
    let contents =
        fs.read_to_string(&project_json_path)
            .map_err(|source| ProjectStoreError::Io {
                path: project_json_path.clone(),
                source,
            })?;
    let value: serde_json::Value =
        serde_json::from_str(&contents).map_err(|error| ProjectStoreError::InvalidProjectJson {
            path: project_json_path.clone(),
            message: error.to_string(),
        })?;
    let draft = migrate_draft_json(value)
        .map_err(|source| draft_validation_error(&project_json_path, source))?;
    let warnings = collect_warnings(fs, bundle_path, &draft)?;

    Ok(ProjectBundleOpenResult {
        bundle: ProjectBundle {
            bundle_path: bundle_path.to_path_buf(),
            project_json_path,
            draft,
        },
        warnings,
    })
}

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

fn draft_validation_error(path: &Path, source: DraftValidationError) -> ProjectStoreError {
    match source {
        DraftValidationError::InvalidSchemaVersion { found, .. } => {
            ProjectStoreError::UnsupportedSchemaVersion {
                path: path.to_path_buf(),
                found,
            }
        }
        other => semantic_error(path, other),
    }
}

fn semantic_error(path: &Path, source: DraftValidationError) -> ProjectStoreError {
    ProjectStoreError::SemanticValidation {
        path: path.to_path_buf(),
        source,
    }
}

#[cfg(test)]
mod tests {
    use draft_model::Draft;

    use crate::{StdPlatformFileSystem, create_project_bundle};

    #[test]
    fn create_project_bundle_writes_valid_project_json() {
        let temp_dir = tempfile::tempdir().expect("tempdir should be created");
        let bundle_path = temp_dir.path().join("new-draft.veproj");
        let draft = Draft::new("draft-001", "New draft");

        let bundle = create_project_bundle(&StdPlatformFileSystem, &bundle_path, &draft)
            .expect("bundle should be created");

        assert_eq!(bundle.bundle_path, bundle_path);
        assert!(bundle.project_json_path.exists());
        assert_eq!(bundle.draft, draft);
    }
}
