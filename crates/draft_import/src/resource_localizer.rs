use std::{
    collections::BTreeSet,
    error::Error,
    fmt, fs, io,
    path::{Component, Path, PathBuf},
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    AdaptationCategory, AdaptationReportItem, AdaptationSeverity, AdaptationStatus,
    AdaptationTargetKind, AdaptationTargetRef, ExternalProvenanceRef,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceLocalizationRequest {
    pub bundle_path: PathBuf,
    pub source_root: PathBuf,
    pub import_id: String,
    pub resources: Vec<TemplateResourceRef>,
    pub mode: ResourceLocalizationMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceLocalizationResult {
    pub manifest: LocalizedResourceManifest,
    pub diagnostics: Vec<AdaptationReportItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceLocalizationMode {
    CopyRenderableResources,
    ReferenceExistingBundleResources,
    PreserveExternalSourceMedia,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TemplateResourceRef {
    pub stable_id: String,
    pub kind: TemplateResourceKind,
    pub source_uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum TemplateResourceKind {
    Material,
    Video,
    Image,
    Audio,
    Sticker,
    Font,
    Effect,
    Filter,
    Transition,
    Other,
}

impl TemplateResourceKind {
    fn destination_dir(self) -> &'static str {
        match self {
            Self::Material => "materials",
            Self::Video => "videos",
            Self::Image => "images",
            Self::Audio => "audio",
            Self::Sticker => "stickers",
            Self::Font => "fonts",
            Self::Effect => "effects",
            Self::Filter => "filters",
            Self::Transition => "transitions",
            Self::Other => "other",
        }
    }

    fn resource_index_kind(self) -> LocalizedResourceIndexKind {
        match self {
            Self::Font => LocalizedResourceIndexKind::Font,
            Self::Effect => LocalizedResourceIndexKind::Effect,
            Self::Filter => LocalizedResourceIndexKind::Filter,
            Self::Transition => LocalizedResourceIndexKind::Transition,
            Self::Material
            | Self::Video
            | Self::Image
            | Self::Audio
            | Self::Sticker
            | Self::Other => LocalizedResourceIndexKind::Material,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocalizedResourceManifest {
    pub import_id: String,
    pub resources: Vec<LocalizedResource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocalizedResource {
    pub stable_id: String,
    pub kind: TemplateResourceKind,
    pub source_uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_relative_ref: Option<String>,
    pub resource_index_ref: LocalizedResourceIndexRef,
    pub status: LocalizedResourceStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocalizedResourceIndexRef {
    pub kind: LocalizedResourceIndexKind,
    pub resource_id: String,
    pub stable_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_relative_ref: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum LocalizedResourceIndexKind {
    Material,
    Font,
    Effect,
    Filter,
    Transition,
}

impl LocalizedResourceIndexKind {
    fn as_resource_id_prefix(self) -> &'static str {
        match self {
            Self::Material => "material",
            Self::Font => "font",
            Self::Effect => "effect",
            Self::Filter => "filter",
            Self::Transition => "transition",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum LocalizedResourceStatus {
    Available,
    Missing,
    Sha256Mismatch,
    UnsafePath,
    RemoteRenderUrl,
    DuplicateDestination,
}

#[derive(Debug)]
pub enum ResourceLocalizationError {
    Io { path: PathBuf, source: io::Error },
    InvalidRoot { path: PathBuf, reason: String },
}

impl fmt::Display for ResourceLocalizationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io { path, source } => {
                write!(
                    formatter,
                    "resource localization IO failed at {}: {source}",
                    path.display()
                )
            }
            Self::InvalidRoot { path, reason } => {
                write!(
                    formatter,
                    "invalid resource localization root {}: {reason}",
                    path.display()
                )
            }
        }
    }
}

impl Error for ResourceLocalizationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::InvalidRoot { .. } => None,
        }
    }
}

pub fn localize_template_resources(
    request: ResourceLocalizationRequest,
) -> Result<ResourceLocalizationResult, ResourceLocalizationError> {
    let canonical_source_root = canonicalize_existing_dir(&request.source_root)?;
    let canonical_bundle_path = canonicalize_existing_dir(&request.bundle_path)?;
    let import_stem = safe_stem(&request.import_id, "import");
    let mut localized_resources = Vec::with_capacity(request.resources.len());
    let mut diagnostics = Vec::new();
    let mut seen_destinations = BTreeSet::new();

    for resource in &request.resources {
        let localized = localize_resource(
            &request,
            &canonical_source_root,
            &canonical_bundle_path,
            &import_stem,
            &mut seen_destinations,
            resource,
        )?;
        if localized.status != LocalizedResourceStatus::Available {
            diagnostics.push(resource_diagnostic(resource, &localized));
        }
        localized_resources.push(localized);
    }

    Ok(ResourceLocalizationResult {
        manifest: LocalizedResourceManifest {
            import_id: request.import_id,
            resources: localized_resources,
        },
        diagnostics,
    })
}

fn localize_resource(
    request: &ResourceLocalizationRequest,
    canonical_source_root: &Path,
    canonical_bundle_path: &Path,
    import_stem: &str,
    seen_destinations: &mut BTreeSet<String>,
    resource: &TemplateResourceRef,
) -> Result<LocalizedResource, ResourceLocalizationError> {
    let source_uri = resource.source_uri.trim();
    if source_uri.is_empty() {
        return Ok(failed_resource(
            resource,
            LocalizedResourceStatus::Missing,
            None,
        ));
    }
    if looks_like_remote_url(source_uri) {
        return Ok(failed_resource(
            resource,
            LocalizedResourceStatus::RemoteRenderUrl,
            None,
        ));
    }

    let Some(source_relative_path) = relative_path_for_uri(source_uri) else {
        return Ok(failed_resource(
            resource,
            LocalizedResourceStatus::UnsafePath,
            None,
        ));
    };
    let project_relative_ref =
        match destination_uri_for_resource(import_stem, resource, &source_relative_path) {
            Some(uri) => uri,
            None => {
                return Ok(failed_resource(
                    resource,
                    LocalizedResourceStatus::UnsafePath,
                    None,
                ));
            }
        };
    if !seen_destinations.insert(project_relative_ref.clone()) {
        return Ok(failed_resource(
            resource,
            LocalizedResourceStatus::DuplicateDestination,
            None,
        ));
    }

    let source_path = canonical_source_root.join(&source_relative_path);
    let source_path = match trusted_existing_file_path(canonical_source_root, &source_path)? {
        TrustedFilePath::Available(path) => path,
        TrustedFilePath::Missing => {
            return Ok(failed_resource(
                resource,
                LocalizedResourceStatus::Missing,
                None,
            ));
        }
        TrustedFilePath::Unsafe => {
            return Ok(failed_resource(
                resource,
                LocalizedResourceStatus::UnsafePath,
                None,
            ));
        }
    };

    if let Some(expected) = resource.sha256.as_deref() {
        let actual = sha256_file_hex(&source_path)?;
        if !actual.eq_ignore_ascii_case(expected.trim()) {
            return Ok(failed_resource(
                resource,
                LocalizedResourceStatus::Sha256Mismatch,
                None,
            ));
        }
    }

    match request.mode {
        ResourceLocalizationMode::CopyRenderableResources => {
            let Some(destination_path) =
                writable_destination_path(canonical_bundle_path, &project_relative_ref)?
            else {
                return Ok(failed_resource(
                    resource,
                    LocalizedResourceStatus::UnsafePath,
                    None,
                ));
            };
            fs::copy(&source_path, &destination_path).map_err(|source| {
                ResourceLocalizationError::Io {
                    path: destination_path,
                    source,
                }
            })?;
        }
        ResourceLocalizationMode::ReferenceExistingBundleResources => {
            let destination_path = canonical_bundle_path.join(Path::new(&project_relative_ref));
            match trusted_existing_file_path(canonical_bundle_path, &destination_path)? {
                TrustedFilePath::Available(_) => {}
                TrustedFilePath::Missing => {
                    return Ok(failed_resource(
                        resource,
                        LocalizedResourceStatus::Missing,
                        None,
                    ));
                }
                TrustedFilePath::Unsafe => {
                    return Ok(failed_resource(
                        resource,
                        LocalizedResourceStatus::UnsafePath,
                        None,
                    ));
                }
            }
        }
        ResourceLocalizationMode::PreserveExternalSourceMedia => {}
    }

    Ok(successful_resource(resource, project_relative_ref))
}

fn successful_resource(
    resource: &TemplateResourceRef,
    project_relative_ref: String,
) -> LocalizedResource {
    LocalizedResource {
        stable_id: resource.stable_id.clone(),
        kind: resource.kind,
        source_uri: resource.source_uri.clone(),
        project_relative_ref: Some(project_relative_ref.clone()),
        resource_index_ref: resource_index_ref(resource, Some(project_relative_ref)),
        status: LocalizedResourceStatus::Available,
        sha256: resource.sha256.clone(),
        display_name: resource.display_name.clone(),
    }
}

fn failed_resource(
    resource: &TemplateResourceRef,
    status: LocalizedResourceStatus,
    project_relative_ref: Option<String>,
) -> LocalizedResource {
    LocalizedResource {
        stable_id: resource.stable_id.clone(),
        kind: resource.kind,
        source_uri: resource.source_uri.clone(),
        project_relative_ref: project_relative_ref.clone(),
        resource_index_ref: resource_index_ref(resource, project_relative_ref),
        status,
        sha256: resource.sha256.clone(),
        display_name: resource.display_name.clone(),
    }
}

fn resource_index_ref(
    resource: &TemplateResourceRef,
    project_relative_ref: Option<String>,
) -> LocalizedResourceIndexRef {
    let kind = resource.kind.resource_index_kind();
    let stable_key = format!(
        "template-import:{}:{}",
        kind.as_resource_id_prefix(),
        safe_stem(&resource.stable_id, "resource")
    );
    LocalizedResourceIndexRef {
        kind,
        resource_id: format!("{}:{stable_key}", kind.as_resource_id_prefix()),
        stable_key,
        project_relative_ref,
    }
}

fn resource_diagnostic(
    resource: &TemplateResourceRef,
    localized: &LocalizedResource,
) -> AdaptationReportItem {
    AdaptationReportItem {
        status: AdaptationStatus::MissingResource,
        severity: AdaptationSeverity::Error,
        category: AdaptationCategory::Resource,
        target: Some(AdaptationTargetRef {
            kind: AdaptationTargetKind::Resource,
            id: Some(resource.stable_id.clone()),
        }),
        message: resource_status_message(localized.status).to_owned(),
        details: Some(resource.source_uri.clone()),
        provenance: vec![ExternalProvenanceRef {
            source_kind: "offlineTemplateResource".to_owned(),
            external_id: Some(resource.stable_id.clone()),
            external_path: Some(resource.source_uri.clone()),
            note: Some(format!("{:?}", localized.status)),
        }],
    }
}

fn resource_status_message(status: LocalizedResourceStatus) -> &'static str {
    match status {
        LocalizedResourceStatus::Available => "Resource was localized.",
        LocalizedResourceStatus::Missing => {
            "Referenced resource is missing from the offline bundle."
        }
        LocalizedResourceStatus::Sha256Mismatch => {
            "Referenced resource failed sha256 validation and was not copied."
        }
        LocalizedResourceStatus::UnsafePath => {
            "Referenced resource path is unsafe, may involve a symlink escape, and was not copied."
        }
        LocalizedResourceStatus::RemoteRenderUrl => {
            "Remote resource URL must be localized before preview or export."
        }
        LocalizedResourceStatus::DuplicateDestination => {
            "Duplicate resource destination was rejected before copying."
        }
    }
}

fn destination_uri_for_resource(
    import_stem: &str,
    resource: &TemplateResourceRef,
    source_relative_path: &Path,
) -> Option<String> {
    if !is_safe_relative_path(source_relative_path) {
        return None;
    }
    let destination = PathBuf::from("resources")
        .join("template-import")
        .join(import_stem)
        .join(resource.kind.destination_dir())
        .join(safe_stem(&resource.stable_id, "resource"))
        .join(source_relative_path);
    let uri = path_to_uri(&destination)?;
    if validate_project_relative_resource_uri(&uri) {
        Some(uri)
    } else {
        None
    }
}

fn relative_path_for_uri(source_uri: &str) -> Option<PathBuf> {
    let normalized = source_uri.trim().replace('\\', "/");
    let trimmed = normalized.strip_prefix("./").unwrap_or(&normalized);
    let path = Path::new(trimmed);
    if path.is_absolute()
        || is_windows_drive_absolute_path(trimmed)
        || has_uri_scheme(trimmed)
        || !is_safe_relative_path(path)
    {
        return None;
    }
    Some(path.to_path_buf())
}

enum TrustedFilePath {
    Available(PathBuf),
    Missing,
    Unsafe,
}

fn canonicalize_existing_dir(path: &Path) -> Result<PathBuf, ResourceLocalizationError> {
    let canonical = path
        .canonicalize()
        .map_err(|source| ResourceLocalizationError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    if !canonical.is_dir() {
        return Err(ResourceLocalizationError::InvalidRoot {
            path: path.to_path_buf(),
            reason: "path is not a directory".to_owned(),
        });
    }
    Ok(canonical)
}

fn trusted_existing_file_path(
    canonical_root: &Path,
    path: &Path,
) -> Result<TrustedFilePath, ResourceLocalizationError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(source) if source.kind() == io::ErrorKind::NotFound => {
            return Ok(TrustedFilePath::Missing);
        }
        Err(source) => {
            return Err(ResourceLocalizationError::Io {
                path: path.to_path_buf(),
                source,
            });
        }
    };
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Ok(TrustedFilePath::Unsafe);
    }
    let canonical = path
        .canonicalize()
        .map_err(|source| ResourceLocalizationError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    if !canonical.starts_with(canonical_root) {
        return Ok(TrustedFilePath::Unsafe);
    }
    Ok(TrustedFilePath::Available(canonical))
}

fn writable_destination_path(
    canonical_bundle_root: &Path,
    project_relative_ref: &str,
) -> Result<Option<PathBuf>, ResourceLocalizationError> {
    if !validate_project_relative_resource_uri(project_relative_ref) {
        return Ok(None);
    }
    let relative_path = Path::new(project_relative_ref);
    let Some(parent) = relative_path.parent() else {
        return Ok(None);
    };
    let Some(file_name) = relative_path.file_name() else {
        return Ok(None);
    };
    let Some(parent) = ensure_directory_without_symlink(canonical_bundle_root, parent)? else {
        return Ok(None);
    };
    let destination = parent.join(file_name);
    match fs::symlink_metadata(&destination) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || metadata.file_type().is_dir() {
                return Ok(None);
            }
            let canonical =
                destination
                    .canonicalize()
                    .map_err(|source| ResourceLocalizationError::Io {
                        path: destination.clone(),
                        source,
                    })?;
            if !canonical.starts_with(canonical_bundle_root) {
                return Ok(None);
            }
        }
        Err(source) if source.kind() == io::ErrorKind::NotFound => {}
        Err(source) => {
            return Err(ResourceLocalizationError::Io {
                path: destination.clone(),
                source,
            });
        }
    }
    Ok(Some(destination))
}

fn ensure_directory_without_symlink(
    canonical_root: &Path,
    relative_path: &Path,
) -> Result<Option<PathBuf>, ResourceLocalizationError> {
    if !is_safe_relative_path(relative_path) {
        return Ok(None);
    }

    let mut current = canonical_root.to_path_buf();
    for component in relative_path.components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => {
                let next = current.join(part);
                match fs::symlink_metadata(&next) {
                    Ok(metadata) => {
                        if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
                            return Ok(None);
                        }
                    }
                    Err(source) if source.kind() == io::ErrorKind::NotFound => {
                        fs::create_dir(&next).map_err(|source| ResourceLocalizationError::Io {
                            path: next.clone(),
                            source,
                        })?;
                    }
                    Err(source) => {
                        return Err(ResourceLocalizationError::Io { path: next, source });
                    }
                }
                let canonical =
                    next.canonicalize()
                        .map_err(|source| ResourceLocalizationError::Io {
                            path: next.clone(),
                            source,
                        })?;
                if !canonical.starts_with(canonical_root) {
                    return Ok(None);
                }
                current = canonical;
            }
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return Ok(None),
        }
    }

    Ok(Some(current))
}

fn validate_project_relative_resource_uri(uri: &str) -> bool {
    uri.starts_with("resources/template-import/") && is_safe_relative_path(Path::new(uri))
}

fn is_safe_relative_path(path: &Path) -> bool {
    if path.components().next().is_none() {
        return false;
    }
    for component in path.components() {
        match component {
            Component::Normal(_) | Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return false,
        }
    }
    true
}

fn path_to_uri(path: &Path) -> Option<String> {
    path.to_str().map(|value| value.replace('\\', "/"))
}

fn safe_stem(value: &str, fallback: &str) -> String {
    let mut stem = String::new();
    for character in value.trim().chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
            stem.push(character.to_ascii_lowercase());
        } else if !stem.ends_with('-') {
            stem.push('-');
        }
    }
    let stem = stem.trim_matches('-');
    if stem.is_empty() {
        fallback.to_owned()
    } else {
        stem.to_owned()
    }
}

fn sha256_file_hex(path: &Path) -> Result<String, ResourceLocalizationError> {
    let bytes = fs::read(path).map_err(|source| ResourceLocalizationError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let digest = Sha256::digest(&bytes);
    let mut hex = String::with_capacity(64);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    Ok(hex)
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

fn is_windows_drive_absolute_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/')
}

fn looks_like_remote_url(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}
