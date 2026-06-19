use std::collections::BTreeSet;

use draft_model::MaterialId;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::ArtifactStoreError;
use crate::dependencies::{ArtifactDependency, artifact_ids_for_dependency};
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
