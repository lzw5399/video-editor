use std::collections::BTreeMap;
use std::path::Path;

use draft_model::{
    Draft, MaterialId, MaterialStatus, Segment, TextBubbleRef, TextEffectRef, TextSegment,
};
use project_store::{MaterialUriKind, classify_material_uri};
use rusqlite::params;
use serde::{Deserialize, Serialize};

use crate::ArtifactStoreError;
use crate::schema::open_artifact_store;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResourceKind {
    Material,
    Font,
    Effect,
    Filter,
    Transition,
    Proxy,
    Thumbnail,
    Waveform,
    GraphSnapshot,
    PreviewArtifact,
}

impl ResourceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Material => "material",
            Self::Font => "font",
            Self::Effect => "effect",
            Self::Filter => "filter",
            Self::Transition => "transition",
            Self::Proxy => "proxy",
            Self::Thumbnail => "thumbnail",
            Self::Waveform => "waveform",
            Self::GraphSnapshot => "graphSnapshot",
            Self::PreviewArtifact => "previewArtifact",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ResourceId(String);

impl ResourceId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceRef {
    pub kind: ResourceKind,
    pub resource_id: ResourceId,
    pub stable_key: String,
    pub parent_material_id: Option<MaterialId>,
}

impl ResourceRef {
    pub fn new(
        kind: ResourceKind,
        resource_id: impl Into<String>,
        stable_key: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            resource_id: ResourceId::new(resource_id),
            stable_key: stable_key.into(),
            parent_material_id: None,
        }
    }

    pub fn derived_role(&self, kind: ResourceKind, role: impl AsRef<str>) -> Self {
        let role = role.as_ref();
        Self {
            kind,
            resource_id: ResourceId::new(format!(
                "{}:{}:{}",
                kind.as_str(),
                self.resource_id.as_str(),
                role
            )),
            stable_key: format!("{}:{}:{}", self.stable_key, kind.as_str(), role),
            parent_material_id: self
                .parent_material_id
                .clone()
                .or_else(|| material_id_from_resource(self)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ResourceStatus {
    Ready,
    Missing,
    ProbeFailed,
}

impl ResourceStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Missing => "missing",
            Self::ProbeFailed => "probeFailed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct IndexedResource {
    pub resource_id: ResourceId,
    pub kind: ResourceKind,
    pub stable_key: String,
    pub parent_material_id: Option<MaterialId>,
    pub source_ref: Option<String>,
    pub project_relative_ref: Option<String>,
    pub status: ResourceStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceIndex {
    resources: BTreeMap<ResourceId, IndexedResource>,
}

impl ResourceIndex {
    pub fn resource(&self, resource_id: &str) -> Option<&IndexedResource> {
        self.resources.get(&ResourceId::new(resource_id))
    }

    pub fn resources(&self) -> impl Iterator<Item = &IndexedResource> {
        self.resources.values()
    }
}

pub fn index_draft_resources(
    bundle_path: impl AsRef<Path>,
    draft: &Draft,
) -> Result<ResourceIndex, ArtifactStoreError> {
    index_draft_resources_with_extra_refs(
        bundle_path,
        draft,
        std::iter::empty::<(ResourceRef, Option<String>)>(),
    )
}

pub fn index_draft_resources_with_extra_refs(
    bundle_path: impl AsRef<Path>,
    draft: &Draft,
    extra_resources: impl IntoIterator<Item = (ResourceRef, Option<String>)>,
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
    for (resource_ref, project_relative_ref) in extra_resources {
        upsert_resource(&mut index, resource_ref, project_relative_ref.as_deref())?;
    }

    persist_resource_index(store.connection(), &index, 0)?;
    Ok(index)
}

pub fn upsert_resource(
    index: &mut ResourceIndex,
    resource_ref: ResourceRef,
    project_relative_ref: Option<&str>,
) -> Result<IndexedResource, ArtifactStoreError> {
    if let Some(path) = project_relative_ref {
        validate_project_relative_ref(resource_ref.resource_id.as_str(), path)?;
    }
    let resource = IndexedResource {
        resource_id: resource_ref.resource_id,
        kind: resource_ref.kind,
        stable_key: resource_ref.stable_key,
        parent_material_id: resource_ref.parent_material_id,
        source_ref: project_relative_ref.map(str::to_owned),
        project_relative_ref: project_relative_ref.map(str::to_owned),
        status: ResourceStatus::Ready,
    };
    upsert_indexed_resource(index, resource.clone())?;
    Ok(resource)
}

pub fn list_resources_for_material<'a>(
    index: &'a ResourceIndex,
    material_id: &str,
) -> Vec<&'a IndexedResource> {
    index
        .resources()
        .filter(|resource| {
            resource
                .parent_material_id
                .as_ref()
                .is_some_and(|parent| parent.as_str() == material_id)
        })
        .collect()
}

pub fn resource_ref_for_material(material_id: impl AsRef<str>) -> ResourceRef {
    let material_id = material_id.as_ref();
    ResourceRef {
        kind: ResourceKind::Material,
        resource_id: ResourceId::new(format!("material:{material_id}")),
        stable_key: format!("material:{material_id}"),
        parent_material_id: Some(MaterialId::new(material_id)),
    }
}

pub fn resource_ref_for_font(font_ref: impl AsRef<str>) -> ResourceRef {
    let font_ref = font_ref.as_ref();
    ResourceRef::new(
        ResourceKind::Font,
        format!("font:{font_ref}"),
        font_ref.to_owned(),
    )
}

pub fn resource_ref_for_effect(kind: ResourceKind, effect_ref: impl AsRef<str>) -> ResourceRef {
    let effect_ref = effect_ref.as_ref();
    ResourceRef::new(
        kind,
        format!("{}:{effect_ref}", kind.as_str()),
        effect_ref.to_owned(),
    )
}

fn index_segment_resources(
    index: &mut ResourceIndex,
    segment: &Segment,
) -> Result<(), ArtifactStoreError> {
    if let Some(text) = &segment.text {
        index_text_resources(index, text)?;
    }
    for filter in &segment.filters {
        let display_name = filter.display_name();
        upsert_resource(
            index,
            resource_ref_for_effect(ResourceKind::Filter, filter.capability_id()),
            Some(&display_name),
        )?;
    }
    if let Some(transition) = &segment.transition {
        let display_name = transition.display_name();
        upsert_resource(
            index,
            resource_ref_for_effect(ResourceKind::Transition, transition.capability_id()),
            Some(&display_name),
        )?;
    }
    Ok(())
}

fn index_text_resources(
    index: &mut ResourceIndex,
    text: &TextSegment,
) -> Result<(), ArtifactStoreError> {
    if let Some(font_ref) = text.style.font.font_ref.as_deref() {
        upsert_resource(index, resource_ref_for_font(font_ref), Some(font_ref))?;
    }
    if let Some(effect) = &text.effect {
        let effect_ref = text_effect_key(effect);
        upsert_resource(
            index,
            resource_ref_for_effect(ResourceKind::Effect, effect_ref),
            None,
        )?;
    }
    if let Some(bubble) = &text.bubble {
        let bubble_ref = text_bubble_key(bubble);
        upsert_resource(
            index,
            resource_ref_for_effect(ResourceKind::Effect, bubble_ref),
            None,
        )?;
    }
    Ok(())
}

fn upsert_indexed_resource(
    index: &mut ResourceIndex,
    resource: IndexedResource,
) -> Result<(), ArtifactStoreError> {
    validate_resource_id(resource.resource_id.as_str())?;
    if let Some(path) = resource.project_relative_ref.as_deref() {
        validate_project_relative_ref(resource.resource_id.as_str(), path)?;
    }
    index
        .resources
        .insert(resource.resource_id.clone(), resource);
    Ok(())
}

fn persist_resource_index(
    conn: &rusqlite::Connection,
    index: &ResourceIndex,
    now_unix_ms: i64,
) -> Result<(), ArtifactStoreError> {
    let transaction =
        conn.unchecked_transaction()
            .map_err(|source| ArtifactStoreError::Sqlite {
                path: "artifact-store.sqlite".into(),
                source,
            })?;
    for resource in index.resources() {
        transaction.execute(
            "INSERT INTO resource (
                resource_id, resource_kind, stable_key, source_uri, project_relative_ref,
                source_fingerprint, source_byte_count, status, created_at_unix_ms, updated_at_unix_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, NULL, NULL, ?6, ?7, ?7)
            ON CONFLICT(resource_id) DO UPDATE SET
                resource_kind = excluded.resource_kind,
                stable_key = excluded.stable_key,
                source_uri = excluded.source_uri,
                project_relative_ref = excluded.project_relative_ref,
                status = excluded.status,
                updated_at_unix_ms = excluded.updated_at_unix_ms",
            params![
                resource.resource_id.as_str(),
                resource.kind.as_str(),
                &resource.stable_key,
                &resource.source_ref,
                &resource.project_relative_ref,
                resource.status.as_str(),
                now_unix_ms,
            ],
        )
        .map_err(|source| ArtifactStoreError::Sqlite {
            path: "artifact-store.sqlite".into(),
            source,
        })?;
    }
    transaction
        .commit()
        .map_err(|source| ArtifactStoreError::Sqlite {
            path: "artifact-store.sqlite".into(),
            source,
        })?;
    Ok(())
}

fn resource_status_from_material(status: MaterialStatus) -> ResourceStatus {
    match status {
        MaterialStatus::Available => ResourceStatus::Ready,
        MaterialStatus::Missing => ResourceStatus::Missing,
        MaterialStatus::ProbeFailed => ResourceStatus::ProbeFailed,
    }
}

fn material_id_from_resource(resource_ref: &ResourceRef) -> Option<MaterialId> {
    resource_ref
        .resource_id
        .as_str()
        .strip_prefix("material:")
        .map(MaterialId::new)
}

fn text_effect_key(effect: &TextEffectRef) -> &str {
    match effect {
        TextEffectRef::Unsupported { name, .. } => name,
    }
}

fn text_bubble_key(effect: &TextBubbleRef) -> &str {
    match effect {
        TextBubbleRef::Unsupported { name, .. } => name,
    }
}

fn validate_resource_id(resource_id: &str) -> Result<(), ArtifactStoreError> {
    if resource_id.trim().is_empty() {
        return Err(ArtifactStoreError::InvalidResourceRef {
            resource_id: resource_id.to_owned(),
            reason: "resource id must not be empty".to_owned(),
        });
    }
    Ok(())
}

fn validate_project_relative_ref(
    resource_id: &str,
    project_relative_ref: &str,
) -> Result<(), ArtifactStoreError> {
    let trimmed = project_relative_ref.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('/')
        || trimmed.starts_with("..")
        || trimmed.contains("/../")
        || is_windows_absolute(trimmed)
    {
        return Err(ArtifactStoreError::InvalidResourceRef {
            resource_id: resource_id.to_owned(),
            reason: format!("resource ref must be project-relative: {project_relative_ref}"),
        });
    }
    Ok(())
}

fn is_windows_absolute(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'\\' | b'/')
}
