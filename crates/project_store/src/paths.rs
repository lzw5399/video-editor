use std::path::{Component, Path, PathBuf};

use crate::ProjectStoreError;

pub const PROJECT_JSON_FILE_NAME: &str = "project.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MaterialUriKind {
    InBundleRelative,
    ExternalAbsolute,
    ExternalUri,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MaterialUri {
    pub kind: MaterialUriKind,
    pub uri: String,
    pub resolved_path: Option<PathBuf>,
}

pub fn project_json_path(bundle_path: impl AsRef<Path>) -> PathBuf {
    bundle_path.as_ref().join(PROJECT_JSON_FILE_NAME)
}

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

pub fn resolve_material_uri(
    bundle_path: impl AsRef<Path>,
    uri: &str,
) -> Result<Option<PathBuf>, ProjectStoreError> {
    Ok(classify_material_uri(bundle_path, uri)?.resolved_path)
}

pub fn material_uri_for_save(
    bundle_path: impl AsRef<Path>,
    material_path: impl AsRef<Path>,
) -> Result<String, ProjectStoreError> {
    let bundle_path = bundle_path.as_ref();
    let material_path = material_path.as_ref();

    if material_path.is_absolute() {
        if let Ok(relative) = material_path.strip_prefix(bundle_path) {
            let uri = path_to_uri(relative)?;
            validate_bundle_relative_path(&uri)?;
            return Ok(uri);
        }

        return Ok(material_path.to_string_lossy().into_owned());
    }

    let uri = path_to_uri(material_path)?;
    validate_bundle_relative_path(&uri)?;
    Ok(uri)
}

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

fn has_uri_scheme(value: &str) -> bool {
    let Some(colon_index) = value.find(':') else {
        return false;
    };
    let scheme = &value[..colon_index];
    !scheme.is_empty()
        && scheme
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'+' | b'-' | b'.'))
}

fn is_absolute_material_path(value: &str, path: &Path) -> bool {
    path.is_absolute() || is_windows_drive_absolute_path(value) || is_windows_unc_path(value)
}

fn is_windows_drive_absolute_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/')
}

fn is_windows_unc_path(value: &str) -> bool {
    value.starts_with(r"\\")
}

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

fn invalid_uri<T>(uri: &str, reason: impl Into<String>) -> Result<T, ProjectStoreError> {
    Err(ProjectStoreError::InvalidMaterialUri {
        uri: uri.to_owned(),
        reason: reason.into(),
    })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{MaterialUriKind, classify_material_uri, material_uri_for_save};

    #[test]
    fn path_resolution_classifies_bundle_relative_material_uri() {
        let bundle = Path::new("/projects/cut.veproj");
        let material =
            classify_material_uri(bundle, "media/video.mp4").expect("relative URI should classify");

        assert_eq!(material.kind, MaterialUriKind::InBundleRelative);
        assert_eq!(
            material.resolved_path.as_deref(),
            Some(Path::new("/projects/cut.veproj/media/video.mp4"))
        );
    }

    #[test]
    fn path_resolution_preserves_external_absolute_material_uri() {
        let material = classify_material_uri("/projects/cut.veproj", "/Users/me/video.mp4")
            .expect("absolute URI should classify");

        assert_eq!(material.kind, MaterialUriKind::ExternalAbsolute);
        assert_eq!(
            material.resolved_path.as_deref(),
            Some(Path::new("/Users/me/video.mp4"))
        );
    }

    #[test]
    fn path_resolution_preserves_windows_drive_material_uri() {
        let material = classify_material_uri("C:/projects/cut.veproj", "C:\\Users\\me\\video.mp4")
            .expect("Windows drive path should classify as absolute");

        assert_eq!(material.kind, MaterialUriKind::ExternalAbsolute);
        assert_eq!(
            material.resolved_path.as_deref(),
            Some(Path::new("C:\\Users\\me\\video.mp4"))
        );

        let forward_slash =
            classify_material_uri("C:/projects/cut.veproj", "C:/Users/me/video.mp4")
                .expect("Windows drive path with forward slashes should classify as absolute");
        assert_eq!(forward_slash.kind, MaterialUriKind::ExternalAbsolute);
        assert_eq!(
            forward_slash.resolved_path.as_deref(),
            Some(Path::new("C:/Users/me/video.mp4"))
        );
    }

    #[test]
    fn path_resolution_rejects_traversal_material_uri() {
        let error = classify_material_uri("/projects/cut.veproj", "../video.mp4")
            .expect_err("traversal should fail");

        assert!(
            error
                .to_string()
                .contains("parent directory traversal is not allowed")
        );
    }

    #[test]
    fn path_resolution_saves_in_bundle_paths_as_relative_uris() {
        let uri = material_uri_for_save(
            "/projects/cut.veproj",
            "/projects/cut.veproj/media/video.mp4",
        )
        .expect("in-bundle path should become relative");

        assert_eq!(uri, "media/video.mp4");
    }
}
