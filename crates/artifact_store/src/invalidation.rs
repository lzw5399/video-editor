use std::collections::BTreeSet;

use draft_model::{
    ChangedEntity, CommandDelta, DirtyDomain, DirtyRange, MaterialId, TargetTimerange,
};
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::ArtifactStoreError;
use crate::dependencies::{
    ArtifactDependency, ArtifactDependencyKind, artifact_ids_for_dependency,
};
use crate::schema::ArtifactStore;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceChangeKind {
    Replaced,
    Relinked,
    Renamed,
    Deleted,
}

impl SourceChangeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Replaced => "replaced",
            Self::Relinked => "relinked",
            Self::Renamed => "renamed",
            Self::Deleted => "deleted",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceChange {
    pub kind: SourceChangeKind,
    pub material_id: Option<MaterialId>,
    pub resource_id: Option<String>,
    pub old_project_relative_ref: Option<String>,
    pub new_project_relative_ref: Option<String>,
    pub old_source_fingerprint: Option<String>,
    pub new_source_fingerprint: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ArtifactDirtyReason {
    SourceChange,
    DependencyMatch,
    FingerprintMismatch,
    FullDraftFallback,
}

impl ArtifactDirtyReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::SourceChange => "sourceChange",
            Self::DependencyMatch => "dependencyMatch",
            Self::FingerprintMismatch => "fingerprintMismatch",
            Self::FullDraftFallback => "fullDraftFallback",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InvalidationFallbackReason {
    UnknownDependency,
    RangeOverflow,
}

impl InvalidationFallbackReason {
    fn as_str(self) -> &'static str {
        match self {
            Self::UnknownDependency => "unknownDependency",
            Self::RangeOverflow => "rangeOverflow",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DirtyArtifactRow {
    pub artifact_id: String,
    pub status: String,
    pub reason: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InvalidationFallback {
    pub reason: InvalidationFallbackReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactInvalidationResult {
    pub dirty_artifacts: Vec<DirtyArtifactRow>,
    pub fallbacks: Vec<InvalidationFallback>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FingerprintChange {
    pub key: String,
    pub fingerprint: String,
}

impl FingerprintChange {
    pub fn new(key: impl Into<String>, fingerprint: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            fingerprint: fingerprint.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactInvalidationRequest {
    pub dirty_ranges: Vec<DirtyRange>,
    pub changed_material_ids: Vec<MaterialId>,
    pub changed_resource_ids: Vec<String>,
    pub changed_graph_node_keys: Vec<String>,
    pub changed_domains: Vec<DirtyDomain>,
    pub source_fingerprint: Option<FingerprintChange>,
    pub graph_fingerprint: Option<FingerprintChange>,
    pub runtime_capability_fingerprint: Option<FingerprintChange>,
    pub output_profile_fingerprint: Option<FingerprintChange>,
    pub artifact_schema_version: Option<u32>,
    pub generator_version: Option<String>,
    pub full_draft: bool,
    pub reason: String,
}

impl ArtifactInvalidationRequest {
    pub fn from_command_delta(delta: &CommandDelta) -> Self {
        let mut changed_material_ids = delta.invalidation.material_ids.clone();
        changed_material_ids.extend(delta.changed_entities.iter().filter_map(|entity| {
            if let ChangedEntity::Material { material_id } = entity {
                Some(material_id.clone())
            } else {
                None
            }
        }));
        changed_material_ids.sort_by(|first, second| first.as_str().cmp(second.as_str()));
        changed_material_ids.dedup();

        let mut changed_graph_node_keys = delta.invalidation.graph_node_ids.clone();
        changed_graph_node_keys.sort();
        changed_graph_node_keys.dedup();

        let mut changed_domains = delta.changed_domains.clone();
        changed_domains.extend(delta.invalidation.consumer_domains.iter().copied());
        changed_domains.sort();
        changed_domains.dedup();

        Self {
            dirty_ranges: delta.changed_ranges.clone(),
            changed_material_ids,
            changed_resource_ids: Vec::new(),
            changed_graph_node_keys,
            changed_domains,
            source_fingerprint: None,
            graph_fingerprint: None,
            runtime_capability_fingerprint: None,
            output_profile_fingerprint: None,
            artifact_schema_version: None,
            generator_version: None,
            full_draft: delta.invalidation.full_draft,
            reason: delta.reason.clone(),
        }
    }
}

pub fn mark_dirty_for_source_change(
    store: &mut ArtifactStore,
    change: SourceChange,
) -> Result<ArtifactInvalidationResult, ArtifactStoreError> {
    if matches!(
        change.kind,
        SourceChangeKind::Relinked | SourceChangeKind::Renamed
    ) {
        update_resource_ref(store, &change)?;
    }

    let artifact_ids = artifact_ids_for_source_change(store, &change)?;
    if artifact_ids.is_empty() {
        return record_full_draft_fallback(
            store,
            InvalidationFallbackReason::UnknownDependency,
            source_change_reason(&change),
        );
    }

    let status = if change.kind == SourceChangeKind::Deleted {
        "tombstoned"
    } else {
        "dirty"
    };
    let dirty_artifacts = mark_artifacts_dirty(
        store,
        artifact_ids,
        status,
        source_change_reason(&change),
        Some(change.kind),
    )?;
    Ok(ArtifactInvalidationResult {
        dirty_artifacts,
        fallbacks: Vec::new(),
    })
}

pub fn mark_dirty_from_command_delta(
    store: &mut ArtifactStore,
    delta: &CommandDelta,
) -> Result<ArtifactInvalidationResult, ArtifactStoreError> {
    let request = ArtifactInvalidationRequest::from_command_delta(delta);
    let range_overflow = request
        .dirty_ranges
        .iter()
        .any(|range| range.target_timerange.checked_end().is_none());
    if request.full_draft || range_overflow {
        return record_full_draft_fallback(
            store,
            if range_overflow {
                InvalidationFallbackReason::RangeOverflow
            } else {
                InvalidationFallbackReason::UnknownDependency
            },
            request.reason,
        );
    }
    mark_dirty_by_dependencies(store, &request)
}

pub fn mark_dirty_by_dependencies(
    store: &mut ArtifactStore,
    request: &ArtifactInvalidationRequest,
) -> Result<ArtifactInvalidationResult, ArtifactStoreError> {
    let artifact_ids = artifact_ids_for_request_dependencies(store, request)?;
    let dirty_artifacts = mark_artifacts_dirty(
        store,
        artifact_ids,
        "dirty",
        dependency_match_reason(request),
        None,
    )?;
    Ok(ArtifactInvalidationResult {
        dirty_artifacts,
        fallbacks: Vec::new(),
    })
}

pub fn mark_dirty_by_fingerprint_mismatch(
    store: &mut ArtifactStore,
    request: &ArtifactInvalidationRequest,
) -> Result<ArtifactInvalidationResult, ArtifactStoreError> {
    if request.full_draft {
        return record_full_draft_fallback(
            store,
            InvalidationFallbackReason::UnknownDependency,
            request.reason.clone(),
        );
    }

    let mut ids = BTreeSet::new();
    if let Some(change) = &request.source_fingerprint {
        extend_fingerprint_mismatch_ids(
            store,
            &mut ids,
            ArtifactDependencyKind::SourceFingerprint,
            change,
        )?;
    }
    if let Some(change) = &request.graph_fingerprint {
        extend_fingerprint_mismatch_ids(
            store,
            &mut ids,
            ArtifactDependencyKind::GraphFingerprint,
            change,
        )?;
    }
    if let Some(change) = &request.runtime_capability_fingerprint {
        extend_fingerprint_mismatch_ids(
            store,
            &mut ids,
            ArtifactDependencyKind::RuntimeCapabilityFingerprint,
            change,
        )?;
    }
    if let Some(change) = &request.output_profile_fingerprint {
        extend_fingerprint_mismatch_ids(
            store,
            &mut ids,
            ArtifactDependencyKind::OutputProfileFingerprint,
            change,
        )?;
    }
    if let Some(version) = request.artifact_schema_version {
        extend_version_mismatch_ids(
            store,
            &mut ids,
            ArtifactDependencyKind::SchemaVersion,
            &format!("schemaVersion:{version}"),
        )?;
    }
    if let Some(version) = &request.generator_version {
        extend_version_mismatch_ids(
            store,
            &mut ids,
            ArtifactDependencyKind::GeneratorVersion,
            &format!("generatorVersion:{version}"),
        )?;
    }

    let dirty_artifacts = mark_artifacts_dirty(
        store,
        ids.into_iter().collect(),
        "dirty",
        fingerprint_mismatch_reason(request),
        None,
    )?;
    Ok(ArtifactInvalidationResult {
        dirty_artifacts,
        fallbacks: Vec::new(),
    })
}

pub fn record_full_draft_fallback(
    store: &mut ArtifactStore,
    fallback_reason: InvalidationFallbackReason,
    reason: impl Into<String>,
) -> Result<ArtifactInvalidationResult, ArtifactStoreError> {
    let reason = format!(
        "{}:{}:{}",
        ArtifactDirtyReason::FullDraftFallback.as_str(),
        fallback_reason.as_str(),
        reason.into()
    );
    let artifact_ids = all_artifact_ids(store)?;
    let dirty_artifacts = mark_artifacts_dirty(store, artifact_ids, "dirty", reason, None)?;
    Ok(ArtifactInvalidationResult {
        dirty_artifacts,
        fallbacks: vec![InvalidationFallback {
            reason: fallback_reason,
        }],
    })
}

fn artifact_ids_for_source_change(
    store: &ArtifactStore,
    change: &SourceChange,
) -> Result<Vec<String>, ArtifactStoreError> {
    let mut ids = BTreeSet::new();
    if let Some(material_id) = &change.material_id {
        extend_dependency_ids(
            store,
            &mut ids,
            ArtifactDependency::material(material_id.as_str()),
        )?;
        extend_dependency_ids(
            store,
            &mut ids,
            ArtifactDependency::source_fingerprint(
                crate::dependencies::DependencyFingerprint::new(
                    format!("source:{}", material_id.as_str()),
                    change.old_source_fingerprint.clone().unwrap_or_default(),
                ),
            ),
        )?;
    }
    if let Some(resource_id) = &change.resource_id {
        extend_dependency_ids(store, &mut ids, ArtifactDependency::resource(resource_id))?;
        extend_dependency_ids(
            store,
            &mut ids,
            ArtifactDependency::source_fingerprint(
                crate::dependencies::DependencyFingerprint::new(
                    resource_id,
                    change.old_source_fingerprint.clone().unwrap_or_default(),
                ),
            ),
        )?;
    }
    if let Some(old_ref) = &change.old_project_relative_ref {
        extend_dependency_ids(
            store,
            &mut ids,
            ArtifactDependency::source_fingerprint(
                crate::dependencies::DependencyFingerprint::new(
                    old_ref,
                    change.old_source_fingerprint.clone().unwrap_or_default(),
                ),
            ),
        )?;
    }
    if let Some(old_fingerprint) = &change.old_source_fingerprint {
        extend_source_fingerprint_value_ids(store, &mut ids, old_fingerprint)?;
    }
    Ok(ids.into_iter().collect())
}

fn artifact_ids_for_request_dependencies(
    store: &ArtifactStore,
    request: &ArtifactInvalidationRequest,
) -> Result<Vec<String>, ArtifactStoreError> {
    let mut ids = BTreeSet::new();
    for material_id in &request.changed_material_ids {
        extend_dependency_ids(
            store,
            &mut ids,
            ArtifactDependency::material(material_id.as_str()),
        )?;
    }
    for resource_id in &request.changed_resource_ids {
        extend_dependency_ids(store, &mut ids, ArtifactDependency::resource(resource_id))?;
    }
    for graph_node_key in &request.changed_graph_node_keys {
        extend_dependency_ids(
            store,
            &mut ids,
            ArtifactDependency::graph_node(graph_node_key),
        )?;
    }
    let has_specific_dependency = !request.dirty_ranges.is_empty()
        || !request.changed_material_ids.is_empty()
        || !request.changed_resource_ids.is_empty()
        || !request.changed_graph_node_keys.is_empty();
    if !has_specific_dependency {
        for domain in &request.changed_domains {
            extend_dependency_ids(store, &mut ids, ArtifactDependency::dirty_domain(*domain))?;
        }
    }
    for range in normalize_dirty_ranges(&request.dirty_ranges)? {
        extend_range_overlap_ids(store, &mut ids, &range.target_timerange, request)?;
    }
    Ok(ids.into_iter().collect())
}

fn normalize_dirty_ranges(ranges: &[DirtyRange]) -> Result<Vec<DirtyRange>, ArtifactStoreError> {
    let target_ranges = ranges
        .iter()
        .map(|range| range.target_timerange.clone())
        .collect::<Vec<_>>();
    let merged = TargetTimerange::merge_sorted(target_ranges).ok_or_else(|| {
        let overflowing = ranges
            .iter()
            .find(|range| range.target_timerange.checked_end().is_none())
            .map(|range| &range.target_timerange);
        ArtifactStoreError::RangeOverflow {
            start_us: overflowing
                .map(|range| range.start.get())
                .unwrap_or(u64::MAX),
            duration_us: overflowing.map(|range| range.duration.get()).unwrap_or(1),
        }
    })?;
    Ok(merged
        .into_iter()
        .map(|target_timerange| DirtyRange {
            target_timerange,
            source: draft_model::DirtyRangeSource::PreviousAndCurrent,
        })
        .collect())
}

fn extend_dependency_ids(
    store: &ArtifactStore,
    ids: &mut BTreeSet<String>,
    dependency: ArtifactDependency,
) -> Result<(), ArtifactStoreError> {
    for artifact_id in artifact_ids_for_dependency(store, dependency)? {
        ids.insert(artifact_id);
    }
    Ok(())
}

fn extend_range_overlap_ids(
    store: &ArtifactStore,
    ids: &mut BTreeSet<String>,
    changed_range: &TargetTimerange,
    request: &ArtifactInvalidationRequest,
) -> Result<(), ArtifactStoreError> {
    let mut statement = store
        .connection()
        .prepare(
            "SELECT artifact_id, target_start_us, target_duration_us
             FROM artifact_dependency
             WHERE dependency_kind = 'targetRange'
             ORDER BY artifact_id",
        )
        .map_err(|source| ArtifactStoreError::Sqlite {
            path: store.db_path.clone(),
            source,
        })?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(|source| ArtifactStoreError::Sqlite {
            path: store.db_path.clone(),
            source,
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| ArtifactStoreError::Sqlite {
            path: store.db_path.clone(),
            source,
        })?;

    for (artifact_id, start_us, duration_us) in rows {
        let artifact_range = TargetTimerange::new(start_us as u64, duration_us as u64);
        if changed_range
            .overlaps_half_open(&artifact_range)
            .unwrap_or(false)
            && artifact_matches_domains(store, &artifact_id, &request.changed_domains)?
        {
            ids.insert(artifact_id);
        }
    }
    Ok(())
}

fn artifact_matches_domains(
    store: &ArtifactStore,
    artifact_id: &str,
    changed_domains: &[DirtyDomain],
) -> Result<bool, ArtifactStoreError> {
    if changed_domains.is_empty() {
        return Ok(true);
    }
    let mut statement = store
        .connection()
        .prepare(
            "SELECT dirty_domain
             FROM artifact_dependency
             WHERE artifact_id = ?1 AND dependency_kind = 'dirtyDomain'",
        )
        .map_err(|source| ArtifactStoreError::Sqlite {
            path: store.db_path.clone(),
            source,
        })?;
    let domains = statement
        .query_map([artifact_id], |row| row.get::<_, Option<String>>(0))
        .map_err(|source| ArtifactStoreError::Sqlite {
            path: store.db_path.clone(),
            source,
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| ArtifactStoreError::Sqlite {
            path: store.db_path.clone(),
            source,
        })?;
    if domains.is_empty() {
        return Ok(true);
    }
    Ok(domains.into_iter().flatten().any(|domain| {
        changed_domains
            .iter()
            .any(|changed| domain == dirty_domain_to_str(*changed))
    }))
}

fn extend_fingerprint_mismatch_ids(
    store: &ArtifactStore,
    ids: &mut BTreeSet<String>,
    kind: ArtifactDependencyKind,
    change: &FingerprintChange,
) -> Result<(), ArtifactStoreError> {
    let mut statement = store
        .connection()
        .prepare(
            "SELECT artifact_id
             FROM artifact_dependency
             WHERE dependency_kind = ?1
                AND dependency_key = ?2
                AND COALESCE(dependency_fingerprint, '') != ?3
             ORDER BY artifact_id",
        )
        .map_err(|source| ArtifactStoreError::Sqlite {
            path: store.db_path.clone(),
            source,
        })?;
    for artifact_id in statement
        .query_map(
            params![kind.as_str(), change.key, change.fingerprint],
            |row| row.get::<_, String>(0),
        )
        .map_err(|source| ArtifactStoreError::Sqlite {
            path: store.db_path.clone(),
            source,
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| ArtifactStoreError::Sqlite {
            path: store.db_path.clone(),
            source,
        })?
    {
        ids.insert(artifact_id);
    }
    Ok(())
}

fn extend_version_mismatch_ids(
    store: &ArtifactStore,
    ids: &mut BTreeSet<String>,
    kind: ArtifactDependencyKind,
    expected_dependency_key: &str,
) -> Result<(), ArtifactStoreError> {
    let mut statement = store
        .connection()
        .prepare(
            "SELECT artifact_id
             FROM artifact_dependency
             WHERE dependency_kind = ?1 AND dependency_key != ?2
             ORDER BY artifact_id",
        )
        .map_err(|source| ArtifactStoreError::Sqlite {
            path: store.db_path.clone(),
            source,
        })?;
    for artifact_id in statement
        .query_map(params![kind.as_str(), expected_dependency_key], |row| {
            row.get::<_, String>(0)
        })
        .map_err(|source| ArtifactStoreError::Sqlite {
            path: store.db_path.clone(),
            source,
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| ArtifactStoreError::Sqlite {
            path: store.db_path.clone(),
            source,
        })?
    {
        ids.insert(artifact_id);
    }
    Ok(())
}

fn extend_source_fingerprint_value_ids(
    store: &ArtifactStore,
    ids: &mut BTreeSet<String>,
    source_fingerprint: &str,
) -> Result<(), ArtifactStoreError> {
    let mut statement = store
        .connection()
        .prepare(
            "SELECT artifact_id
             FROM artifact_dependency
             WHERE dependency_kind = 'sourceFingerprint'
                AND dependency_fingerprint = ?1
             ORDER BY artifact_id",
        )
        .map_err(|source| ArtifactStoreError::Sqlite {
            path: store.db_path.clone(),
            source,
        })?;
    for artifact_id in statement
        .query_map([source_fingerprint], |row| row.get::<_, String>(0))
        .map_err(|source| ArtifactStoreError::Sqlite {
            path: store.db_path.clone(),
            source,
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| ArtifactStoreError::Sqlite {
            path: store.db_path.clone(),
            source,
        })?
    {
        ids.insert(artifact_id);
    }
    Ok(())
}

fn all_artifact_ids(store: &ArtifactStore) -> Result<Vec<String>, ArtifactStoreError> {
    let mut statement = store
        .connection()
        .prepare("SELECT artifact_id FROM artifact ORDER BY artifact_id")
        .map_err(|source| ArtifactStoreError::Sqlite {
            path: store.db_path.clone(),
            source,
        })?;
    statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|source| ArtifactStoreError::Sqlite {
            path: store.db_path.clone(),
            source,
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| ArtifactStoreError::Sqlite {
            path: store.db_path.clone(),
            source,
        })
}

fn dirty_domain_to_str(domain: DirtyDomain) -> &'static str {
    match domain {
        DirtyDomain::Track => "track",
        DirtyDomain::Timing => "timing",
        DirtyDomain::Visual => "visual",
        DirtyDomain::Text => "text",
        DirtyDomain::Audio => "audio",
        DirtyDomain::Material => "material",
        DirtyDomain::Effect => "effect",
        DirtyDomain::Filter => "filter",
        DirtyDomain::Transition => "transition",
        DirtyDomain::Canvas => "canvas",
        DirtyDomain::OutputProfile => "outputProfile",
        DirtyDomain::RuntimeCapabilities => "runtimeCapabilities",
        DirtyDomain::Preview => "preview",
        DirtyDomain::ExportPrep => "exportPrep",
        DirtyDomain::Thumbnail => "thumbnail",
        DirtyDomain::Waveform => "waveform",
        DirtyDomain::Proxy => "proxy",
        DirtyDomain::GraphSnapshot => "graphSnapshot",
        DirtyDomain::PreviewCache => "previewCache",
    }
}

fn mark_artifacts_dirty(
    store: &mut ArtifactStore,
    artifact_ids: Vec<String>,
    status: &str,
    reason: String,
    source_change_kind: Option<SourceChangeKind>,
) -> Result<Vec<DirtyArtifactRow>, ArtifactStoreError> {
    let db_path = store.db_path.clone();
    let tx = store
        .connection_mut()
        .transaction()
        .map_err(|source| ArtifactStoreError::Sqlite {
            path: db_path.clone(),
            source,
        })?;
    let mut dirty_artifacts = Vec::new();
    for artifact_id in artifact_ids {
        tx.execute(
            "UPDATE artifact
             SET dirty = 1,
                 status = ?2,
                 dirty_reason = ?3,
                 dirty_source_change_kind = ?4,
                 dirty_at_unix_ms = ?5,
                 updated_at_unix_ms = ?5
             WHERE artifact_id = ?1",
            params![
                artifact_id,
                status,
                reason,
                source_change_kind.map(SourceChangeKind::as_str),
                0_i64,
            ],
        )
        .map_err(|source| ArtifactStoreError::Sqlite {
            path: db_path.clone(),
            source,
        })?;
        dirty_artifacts.push(DirtyArtifactRow {
            artifact_id,
            status: status.to_owned(),
            reason: reason.clone(),
        });
    }
    tx.commit().map_err(|source| ArtifactStoreError::Sqlite {
        path: db_path,
        source,
    })?;
    Ok(dirty_artifacts)
}

fn update_resource_ref(
    store: &ArtifactStore,
    change: &SourceChange,
) -> Result<(), ArtifactStoreError> {
    let Some(resource_id) = change.resource_id.as_deref() else {
        return Ok(());
    };
    let Some(new_ref) = change.new_project_relative_ref.as_deref() else {
        return Ok(());
    };
    validate_project_relative_ref(resource_id, new_ref)?;
    store
        .connection()
        .execute(
            "UPDATE resource
             SET source_uri = ?2,
                 project_relative_ref = ?2,
                 source_fingerprint = COALESCE(?3, source_fingerprint),
                 updated_at_unix_ms = ?4
             WHERE resource_id = ?1",
            params![resource_id, new_ref, change.new_source_fingerprint, 0_i64],
        )
        .map_err(|source| ArtifactStoreError::Sqlite {
            path: store.db_path.clone(),
            source,
        })?;
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

fn source_change_reason(change: &SourceChange) -> String {
    format!(
        "{}:{}:{}",
        ArtifactDirtyReason::SourceChange.as_str(),
        change.kind.as_str(),
        change.reason
    )
}

fn dependency_match_reason(request: &ArtifactInvalidationRequest) -> String {
    format!(
        "{}:{}",
        ArtifactDirtyReason::DependencyMatch.as_str(),
        request.reason
    )
}

fn fingerprint_mismatch_reason(request: &ArtifactInvalidationRequest) -> String {
    format!(
        "{}:{}",
        ArtifactDirtyReason::FingerprintMismatch.as_str(),
        request.reason
    )
}

#[allow(dead_code)]
fn artifact_exists(store: &ArtifactStore, artifact_id: &str) -> Result<bool, ArtifactStoreError> {
    store
        .connection()
        .query_row(
            "SELECT 1 FROM artifact WHERE artifact_id = ?1",
            [artifact_id],
            |_| Ok(()),
        )
        .optional()
        .map(|row| row.is_some())
        .map_err(|source| ArtifactStoreError::Sqlite {
            path: store.db_path.clone(),
            source,
        })
}
