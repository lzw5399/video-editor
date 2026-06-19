use std::path::{Component, Path, PathBuf};

use crate::ArtifactStoreError;
use crate::schema::ARTIFACT_STORE_DB_FILE_NAME;

pub const DERIVED_DIR_NAME: &str = "derived";
pub const BLOB_DIR_NAME: &str = "blobs";
pub const TMP_DIR_NAME: &str = "tmp";

pub fn derived_root_path(bundle_path: impl AsRef<Path>) -> PathBuf {
    bundle_path.as_ref().join(DERIVED_DIR_NAME)
}

pub fn artifact_store_db_path(bundle_path: impl AsRef<Path>) -> PathBuf {
    derived_root_path(bundle_path).join(ARTIFACT_STORE_DB_FILE_NAME)
}

pub fn blob_root_path(bundle_path: impl AsRef<Path>) -> PathBuf {
    derived_root_path(bundle_path).join(BLOB_DIR_NAME)
}

pub fn blob_tmp_path(bundle_path: impl AsRef<Path>) -> PathBuf {
    blob_root_path(bundle_path).join(TMP_DIR_NAME)
}

pub fn validate_derived_relative_path(
    derived_root: impl AsRef<Path>,
    relative_path: &str,
) -> Result<PathBuf, ArtifactStoreError> {
    let trimmed = relative_path.trim();
    if trimmed.is_empty() {
        return invalid_derived_path(relative_path, "derived relative path must not be empty");
    }
    if is_windows_drive_absolute_path(trimmed) || trimmed.starts_with(r"\\") {
        return invalid_derived_path(relative_path, "derived relative path must not be absolute");
    }

    let relative = Path::new(trimmed);
    if relative.is_absolute() {
        return invalid_derived_path(relative_path, "derived relative path must not be absolute");
    }

    for component in relative.components() {
        match component {
            Component::Normal(_) => {}
            Component::CurDir => {
                return invalid_derived_path(
                    relative_path,
                    "current directory components are not allowed",
                );
            }
            Component::ParentDir => {
                return invalid_derived_path(
                    relative_path,
                    "parent directory traversal is not allowed",
                );
            }
            Component::RootDir | Component::Prefix(_) => {
                return invalid_derived_path(
                    relative_path,
                    "derived relative path must not be absolute",
                );
            }
        }
    }

    let derived_root = derived_root.as_ref();
    let candidate = derived_root.join(relative);
    reject_existing_symlink_escape(derived_root, relative, relative_path)?;
    Ok(candidate)
}

pub fn path_to_slash_string(path: impl AsRef<Path>) -> Result<String, ArtifactStoreError> {
    let path = path.as_ref();
    let value = path
        .to_str()
        .ok_or_else(|| ArtifactStoreError::InvalidDerivedPath {
            path: path.to_string_lossy().into_owned(),
            reason: "derived relative path must be valid UTF-8".to_owned(),
        })?
        .replace('\\', "/");
    Ok(value)
}

fn reject_existing_symlink_escape(
    derived_root: &Path,
    relative: &Path,
    original: &str,
) -> Result<(), ArtifactStoreError> {
    let Ok(canonical_root) = derived_root.canonicalize() else {
        return Ok(());
    };
    let mut cursor = derived_root.to_path_buf();
    for component in relative.components() {
        cursor.push(component.as_os_str());
        if cursor.exists() {
            let canonical = cursor
                .canonicalize()
                .map_err(|source| ArtifactStoreError::Io {
                    path: cursor.clone(),
                    source,
                })?;
            if !canonical.starts_with(&canonical_root) {
                return invalid_derived_path(original, "path escapes derived root through symlink");
            }
        }
    }
    Ok(())
}

fn is_windows_drive_absolute_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/')
}

fn invalid_derived_path<T>(path: &str, reason: impl Into<String>) -> Result<T, ArtifactStoreError> {
    Err(ArtifactStoreError::InvalidDerivedPath {
        path: path.to_owned(),
        reason: reason.into(),
    })
}
