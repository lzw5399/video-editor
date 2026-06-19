use draft_model::DirtyDomain;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ArtifactStoreError;
use crate::schema::ArtifactStore;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ArtifactDependencyKind {
    Material,
    Resource,
    GraphNode,
    DirtyDomain,
    TargetRange,
    SourceRange,
    SourceFingerprint,
    GraphFingerprint,
    RuntimeCapabilityFingerprint,
    OutputProfileFingerprint,
    GenerationParameters,
    SchemaVersion,
    GeneratorVersion,
}

impl ArtifactDependencyKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Material => "material",
            Self::Resource => "resource",
            Self::GraphNode => "graphNode",
            Self::DirtyDomain => "dirtyDomain",
            Self::TargetRange => "targetRange",
            Self::SourceRange => "sourceRange",
            Self::SourceFingerprint => "sourceFingerprint",
            Self::GraphFingerprint => "graphFingerprint",
            Self::RuntimeCapabilityFingerprint => "runtimeCapabilityFingerprint",
            Self::OutputProfileFingerprint => "outputProfileFingerprint",
            Self::GenerationParameters => "generationParameters",
            Self::SchemaVersion => "schemaVersion",
            Self::GeneratorVersion => "generatorVersion",
        }
    }

    fn from_db(value: &str) -> Result<Self, ArtifactStoreError> {
        match value {
            "material" => Ok(Self::Material),
            "resource" => Ok(Self::Resource),
            "graphNode" => Ok(Self::GraphNode),
            "dirtyDomain" => Ok(Self::DirtyDomain),
            "targetRange" => Ok(Self::TargetRange),
            "sourceRange" => Ok(Self::SourceRange),
            "sourceFingerprint" => Ok(Self::SourceFingerprint),
            "graphFingerprint" => Ok(Self::GraphFingerprint),
            "runtimeCapabilityFingerprint" => Ok(Self::RuntimeCapabilityFingerprint),
            "outputProfileFingerprint" => Ok(Self::OutputProfileFingerprint),
            "generationParameters" => Ok(Self::GenerationParameters),
            "schemaVersion" => Ok(Self::SchemaVersion),
            "generatorVersion" => Ok(Self::GeneratorVersion),
            _ => Err(ArtifactStoreError::InvalidDependency {
                dependency_key: value.to_owned(),
                reason: "unknown dependency kind".to_owned(),
            }),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DependencyRange {
    pub start_us: u64,
    pub duration_us: u64,
}

impl DependencyRange {
    pub fn new(start_us: u64, duration_us: u64) -> Self {
        Self {
            start_us,
            duration_us,
        }
    }

    fn checked_end(self) -> Result<u64, ArtifactStoreError> {
        self.start_us.checked_add(self.duration_us).ok_or_else(|| {
            ArtifactStoreError::RangeOverflow {
                start_us: self.start_us,
                duration_us: self.duration_us,
            }
        })
    }

    fn validate_sqlite_integer(self) -> Result<(), ArtifactStoreError> {
        self.checked_end()?;
        if self.start_us > i64::MAX as u64 || self.duration_us > i64::MAX as u64 {
            return Err(ArtifactStoreError::RangeOverflow {
                start_us: self.start_us,
                duration_us: self.duration_us,
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DependencyFingerprint {
    pub key: String,
    pub fingerprint: String,
}

impl DependencyFingerprint {
    pub fn new(key: impl Into<String>, fingerprint: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            fingerprint: fingerprint.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactDependency {
    pub kind: ArtifactDependencyKind,
    pub dependency_key: String,
    pub target_range: Option<DependencyRange>,
    pub source_range: Option<DependencyRange>,
    pub dirty_domain: Option<DirtyDomain>,
    pub dependency_fingerprint: Option<String>,
}

impl ArtifactDependency {
    pub fn material(material_id: impl Into<String>) -> Self {
        Self::plain(ArtifactDependencyKind::Material, material_id)
    }

    pub fn resource(resource_id: impl Into<String>) -> Self {
        Self::plain(ArtifactDependencyKind::Resource, resource_id)
    }

    pub fn graph_node(stable_key: impl Into<String>) -> Self {
        Self::plain(ArtifactDependencyKind::GraphNode, stable_key)
    }

    pub fn dirty_domain(domain: DirtyDomain) -> Self {
        Self {
            kind: ArtifactDependencyKind::DirtyDomain,
            dependency_key: dirty_domain_to_str(domain).to_owned(),
            target_range: None,
            source_range: None,
            dirty_domain: Some(domain),
            dependency_fingerprint: None,
        }
    }

    pub fn target_range(start_us: u64, duration_us: u64) -> Self {
        let range = DependencyRange::new(start_us, duration_us);
        Self {
            kind: ArtifactDependencyKind::TargetRange,
            dependency_key: format!("targetRange:{start_us}:{duration_us}"),
            target_range: Some(range),
            source_range: None,
            dirty_domain: None,
            dependency_fingerprint: None,
        }
    }

    pub fn source_range(start_us: u64, duration_us: u64) -> Self {
        let range = DependencyRange::new(start_us, duration_us);
        Self {
            kind: ArtifactDependencyKind::SourceRange,
            dependency_key: format!("sourceRange:{start_us}:{duration_us}"),
            target_range: None,
            source_range: Some(range),
            dirty_domain: None,
            dependency_fingerprint: None,
        }
    }

    pub fn source_fingerprint(fingerprint: DependencyFingerprint) -> Self {
        Self::fingerprint(ArtifactDependencyKind::SourceFingerprint, fingerprint)
    }

    pub fn graph_fingerprint(fingerprint: DependencyFingerprint) -> Self {
        Self::fingerprint(ArtifactDependencyKind::GraphFingerprint, fingerprint)
    }

    pub fn runtime_capability_fingerprint(fingerprint: DependencyFingerprint) -> Self {
        Self::fingerprint(
            ArtifactDependencyKind::RuntimeCapabilityFingerprint,
            fingerprint,
        )
    }

    pub fn output_profile_fingerprint(fingerprint: DependencyFingerprint) -> Self {
        Self::fingerprint(
            ArtifactDependencyKind::OutputProfileFingerprint,
            fingerprint,
        )
    }

    pub fn generation_parameters(parameters: Value) -> Self {
        let serialized = serde_json::to_string(&parameters).unwrap_or_else(|_| "null".to_owned());
        Self {
            kind: ArtifactDependencyKind::GenerationParameters,
            dependency_key: format!("generationParameters:{serialized}"),
            target_range: None,
            source_range: None,
            dirty_domain: None,
            dependency_fingerprint: Some(serialized),
        }
    }

    pub fn schema_version(version: u32) -> Self {
        Self::plain(
            ArtifactDependencyKind::SchemaVersion,
            format!("schemaVersion:{version}"),
        )
    }

    pub fn generator_version(version: impl Into<String>) -> Self {
        let version = version.into();
        Self::plain(
            ArtifactDependencyKind::GeneratorVersion,
            format!("generatorVersion:{version}"),
        )
    }

    fn plain(kind: ArtifactDependencyKind, key: impl Into<String>) -> Self {
        Self {
            kind,
            dependency_key: key.into(),
            target_range: None,
            source_range: None,
            dirty_domain: None,
            dependency_fingerprint: None,
        }
    }

    fn fingerprint(kind: ArtifactDependencyKind, fingerprint: DependencyFingerprint) -> Self {
        Self {
            kind,
            dependency_key: fingerprint.key,
            target_range: None,
            source_range: None,
            dirty_domain: None,
            dependency_fingerprint: Some(fingerprint.fingerprint),
        }
    }

    fn validate(&self) -> Result<(), ArtifactStoreError> {
        if self.dependency_key.trim().is_empty() {
            return Err(ArtifactStoreError::InvalidDependency {
                dependency_key: self.dependency_key.clone(),
                reason: "dependency key must not be empty".to_owned(),
            });
        }
        if let Some(range) = self.target_range {
            range.validate_sqlite_integer()?;
        }
        if let Some(range) = self.source_range {
            range.validate_sqlite_integer()?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DependencyUpsert {
    pub dependency: ArtifactDependency,
}

impl DependencyUpsert {
    pub fn new(dependency: ArtifactDependency) -> Self {
        Self { dependency }
    }
}

pub fn upsert_artifact_dependencies(
    store: &mut ArtifactStore,
    artifact_id: &str,
    dependencies: Vec<DependencyUpsert>,
) -> Result<(), ArtifactStoreError> {
    let rows = dependencies
        .into_iter()
        .map(|upsert| {
            upsert.dependency.validate()?;
            Ok(upsert.dependency)
        })
        .collect::<Result<Vec<_>, ArtifactStoreError>>()?;

    let db_path = store.db_path.clone();
    let tx = store
        .connection_mut()
        .transaction()
        .map_err(|source| ArtifactStoreError::Sqlite {
            path: db_path.clone(),
            source,
        })?;
    tx.execute(
        "DELETE FROM artifact_dependency WHERE artifact_id = ?1",
        [artifact_id],
    )
    .map_err(|source| ArtifactStoreError::Sqlite {
        path: db_path.clone(),
        source,
    })?;
    for row in rows {
        tx.execute(
            "INSERT INTO artifact_dependency (
                artifact_id, dependency_kind, dependency_key, target_start_us,
                target_duration_us, source_start_us, source_duration_us, dirty_domain,
                dependency_fingerprint, created_at_unix_ms
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                artifact_id,
                row.kind.as_str(),
                row.dependency_key,
                range_start_i64(row.target_range)?,
                range_duration_i64(row.target_range)?,
                range_start_i64(row.source_range)?,
                range_duration_i64(row.source_range)?,
                row.dirty_domain.map(dirty_domain_to_str),
                row.dependency_fingerprint,
                0_i64,
            ],
        )
        .map_err(|source| ArtifactStoreError::Sqlite {
            path: db_path.clone(),
            source,
        })?;
    }
    tx.commit().map_err(|source| ArtifactStoreError::Sqlite {
        path: db_path,
        source,
    })?;
    Ok(())
}

pub fn dependencies_for_artifact(
    store: &ArtifactStore,
    artifact_id: &str,
) -> Result<Vec<ArtifactDependency>, ArtifactStoreError> {
    let mut statement = store
        .connection()
        .prepare(
            "SELECT dependency_kind, dependency_key, target_start_us, target_duration_us,
                source_start_us, source_duration_us, dirty_domain, dependency_fingerprint
             FROM artifact_dependency
             WHERE artifact_id = ?1
             ORDER BY dependency_kind, dependency_key",
        )
        .map_err(|source| ArtifactStoreError::Sqlite {
            path: store.db_path.clone(),
            source,
        })?;
    statement
        .query_map([artifact_id], dependency_from_row)
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

pub fn artifact_ids_for_dependency(
    store: &ArtifactStore,
    dependency: ArtifactDependency,
) -> Result<Vec<String>, ArtifactStoreError> {
    dependency.validate()?;
    let mut statement = store
        .connection()
        .prepare(
            "SELECT artifact_id
             FROM artifact_dependency
             WHERE dependency_kind = ?1 AND dependency_key = ?2
             ORDER BY artifact_id",
        )
        .map_err(|source| ArtifactStoreError::Sqlite {
            path: store.db_path.clone(),
            source,
        })?;
    statement
        .query_map(
            params![dependency.kind.as_str(), dependency.dependency_key],
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
        })
}

pub fn normalize_dependency_ranges(
    ranges: Vec<DependencyRange>,
) -> Result<Vec<DependencyRange>, ArtifactStoreError> {
    let mut sorted = ranges;
    for range in &sorted {
        range.validate_sqlite_integer()?;
    }
    sorted.sort_by_key(|range| (range.start_us, range.duration_us));

    let mut merged: Vec<DependencyRange> = Vec::new();
    for range in sorted {
        let Some(current) = merged.last_mut() else {
            merged.push(range);
            continue;
        };
        let current_end = current.checked_end()?;
        if current_end >= range.start_us {
            let range_end = range.checked_end()?;
            let end = current_end.max(range_end);
            current.duration_us = end.checked_sub(current.start_us).ok_or_else(|| {
                ArtifactStoreError::RangeOverflow {
                    start_us: current.start_us,
                    duration_us: current.duration_us,
                }
            })?;
        } else {
            merged.push(range);
        }
    }
    Ok(merged)
}

fn dependency_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ArtifactDependency> {
    let kind_string: String = row.get(0)?;
    let kind = ArtifactDependencyKind::from_db(&kind_string).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
    })?;
    let dependency_key: String = row.get(1)?;
    let target_start: Option<i64> = row.get(2)?;
    let target_duration: Option<i64> = row.get(3)?;
    let source_start: Option<i64> = row.get(4)?;
    let source_duration: Option<i64> = row.get(5)?;
    let dirty_domain: Option<String> = row.get(6)?;
    let dependency_fingerprint: Option<String> = row.get(7)?;
    Ok(ArtifactDependency {
        kind,
        dependency_key,
        target_range: optional_range(target_start, target_duration)?,
        source_range: optional_range(source_start, source_duration)?,
        dirty_domain: dirty_domain
            .as_deref()
            .map(dirty_domain_from_str)
            .transpose()
            .map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    6,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?,
        dependency_fingerprint,
    })
}

fn optional_range(
    start: Option<i64>,
    duration: Option<i64>,
) -> rusqlite::Result<Option<DependencyRange>> {
    match (start, duration) {
        (Some(start), Some(duration)) if start >= 0 && duration >= 0 => {
            Ok(Some(DependencyRange::new(start as u64, duration as u64)))
        }
        (None, None) => Ok(None),
        _ => Err(rusqlite::Error::InvalidQuery),
    }
}

fn range_start_i64(range: Option<DependencyRange>) -> Result<Option<i64>, ArtifactStoreError> {
    range
        .map(|range| {
            range.validate_sqlite_integer()?;
            Ok(range.start_us as i64)
        })
        .transpose()
}

fn range_duration_i64(range: Option<DependencyRange>) -> Result<Option<i64>, ArtifactStoreError> {
    range
        .map(|range| {
            range.validate_sqlite_integer()?;
            Ok(range.duration_us as i64)
        })
        .transpose()
}

fn dirty_domain_to_str(domain: DirtyDomain) -> &'static str {
    match domain {
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

fn dirty_domain_from_str(value: &str) -> Result<DirtyDomain, ArtifactStoreError> {
    match value {
        "timing" => Ok(DirtyDomain::Timing),
        "visual" => Ok(DirtyDomain::Visual),
        "text" => Ok(DirtyDomain::Text),
        "audio" => Ok(DirtyDomain::Audio),
        "material" => Ok(DirtyDomain::Material),
        "effect" => Ok(DirtyDomain::Effect),
        "filter" => Ok(DirtyDomain::Filter),
        "transition" => Ok(DirtyDomain::Transition),
        "canvas" => Ok(DirtyDomain::Canvas),
        "outputProfile" => Ok(DirtyDomain::OutputProfile),
        "runtimeCapabilities" => Ok(DirtyDomain::RuntimeCapabilities),
        "preview" => Ok(DirtyDomain::Preview),
        "exportPrep" => Ok(DirtyDomain::ExportPrep),
        "thumbnail" => Ok(DirtyDomain::Thumbnail),
        "waveform" => Ok(DirtyDomain::Waveform),
        "proxy" => Ok(DirtyDomain::Proxy),
        "graphSnapshot" => Ok(DirtyDomain::GraphSnapshot),
        "previewCache" => Ok(DirtyDomain::PreviewCache),
        _ => Err(ArtifactStoreError::InvalidDependency {
            dependency_key: value.to_owned(),
            reason: "unknown dirty domain".to_owned(),
        }),
    }
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
