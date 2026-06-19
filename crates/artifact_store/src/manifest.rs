use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ArtifactStoreError;
use crate::blob_store::{BlobRecord, BlobStore, BlobWriteIntent};
use crate::dependencies::{ArtifactDependency, dependencies_for_artifact};
use crate::fingerprint::fingerprint_bytes;
use crate::paths::{derived_root_path, validate_derived_relative_path};
use crate::schema::ArtifactStore;

pub const SYNC_MANIFEST_SCHEMA_VERSION: u32 = 1;
pub const SYNC_MANIFEST_GENERATOR_VERSION: &str = "artifact-store-sync-manifest-v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManifestGenerationOptions {
    pub include_dirty_artifacts: bool,
    pub include_tombstones: bool,
}

impl Default for ManifestGenerationOptions {
    fn default() -> Self {
        Self {
            include_dirty_artifacts: true,
            include_tombstones: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncManifest {
    pub schema_version: u32,
    pub generator_version: String,
    pub entries: Vec<SyncManifestEntry>,
    pub tombstones: Vec<ManifestTombstone>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncManifestEntry {
    pub artifact_id: String,
    pub artifact_kind: String,
    pub stable_key: String,
    pub status: String,
    pub blob_relative_path: String,
    pub blob_fingerprint: String,
    pub byte_count: u64,
    pub source_fingerprint: Option<String>,
    pub graph_fingerprint: Option<String>,
    pub runtime_capability_fingerprint: Option<String>,
    pub output_profile_fingerprint: Option<String>,
    pub schema_fingerprint: String,
    pub generator_fingerprint: String,
    pub generation_parameters: Value,
    pub dependencies: Vec<ManifestDependency>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManifestDependency {
    pub kind: String,
    pub key: String,
    pub target_start_us: Option<u64>,
    pub target_duration_us: Option<u64>,
    pub source_start_us: Option<u64>,
    pub source_duration_us: Option<u64>,
    pub dirty_domain: Option<String>,
    pub dependency_fingerprint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ManifestTombstone {
    pub artifact_id: String,
    pub blob_relative_path: String,
    pub blob_fingerprint: String,
    pub byte_count: u64,
    pub reason: String,
}

pub fn generate_sync_manifest(
    store: &ArtifactStore,
    options: ManifestGenerationOptions,
) -> Result<SyncManifest, ArtifactStoreError> {
    let derived_root = derived_root_path(&store.config.bundle_path);
    let mut statement = store
        .connection()
        .prepare(
            "SELECT artifact_id, artifact_kind, stable_key, status, blob_relative_path,
                blob_fingerprint, byte_count, source_fingerprint, graph_fingerprint,
                runtime_capability_fingerprint, output_profile_fingerprint, schema_fingerprint,
                generator_fingerprint, generation_parameters_json
             FROM artifact
             WHERE blob_relative_path IS NOT NULL
                AND blob_fingerprint IS NOT NULL
                AND status != 'tombstoned'
             ORDER BY stable_key, blob_relative_path, artifact_id",
        )
        .map_err(|source| sqlite_error(store, source))?;
    let rows = statement
        .query_map([], |row| {
            Ok(ArtifactManifestRow {
                artifact_id: row.get(0)?,
                artifact_kind: row.get(1)?,
                stable_key: row.get(2)?,
                status: row.get(3)?,
                blob_relative_path: row.get(4)?,
                blob_fingerprint: row.get(5)?,
                byte_count: row.get(6)?,
                source_fingerprint: row.get(7)?,
                graph_fingerprint: row.get(8)?,
                runtime_capability_fingerprint: row.get(9)?,
                output_profile_fingerprint: row.get(10)?,
                schema_fingerprint: row.get(11)?,
                generator_fingerprint: row.get(12)?,
                generation_parameters_json: row.get(13)?,
            })
        })
        .map_err(|source| sqlite_error(store, source))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| sqlite_error(store, source))?;

    let mut entries = Vec::new();
    for row in rows {
        if !options.include_dirty_artifacts && row.status == "dirty" {
            continue;
        }
        validate_derived_relative_path(&derived_root, &row.blob_relative_path)?;
        if !row
            .blob_relative_path
            .replace('\\', "/")
            .starts_with("blobs/")
        {
            return Err(ArtifactStoreError::InvalidDerivedPath {
                path: row.blob_relative_path,
                reason: "manifest entries must reference derived blob paths".to_owned(),
            });
        }
        entries.push(SyncManifestEntry {
            artifact_id: row.artifact_id.clone(),
            artifact_kind: row.artifact_kind,
            stable_key: row.stable_key,
            status: row.status,
            blob_relative_path: row.blob_relative_path,
            blob_fingerprint: row.blob_fingerprint,
            byte_count: u64::try_from(row.byte_count).map_err(|_| {
                ArtifactStoreError::RangeOverflow {
                    start_us: row.byte_count.unsigned_abs(),
                    duration_us: 0,
                }
            })?,
            source_fingerprint: row.source_fingerprint,
            graph_fingerprint: row.graph_fingerprint,
            runtime_capability_fingerprint: row.runtime_capability_fingerprint,
            output_profile_fingerprint: row.output_profile_fingerprint,
            schema_fingerprint: row.schema_fingerprint,
            generator_fingerprint: row.generator_fingerprint,
            generation_parameters: serde_json::from_str(&row.generation_parameters_json).map_err(
                |error| ArtifactStoreError::InvalidDerivedPath {
                    path: row.artifact_id.clone(),
                    reason: format!("generation parameters must parse: {error}"),
                },
            )?,
            dependencies: manifest_dependencies(store, &row.artifact_id)?,
        });
    }

    let tombstones = if options.include_tombstones {
        manifest_tombstones(store)?
    } else {
        Vec::new()
    };

    Ok(SyncManifest {
        schema_version: SYNC_MANIFEST_SCHEMA_VERSION,
        generator_version: SYNC_MANIFEST_GENERATOR_VERSION.to_owned(),
        entries,
        tombstones,
    })
}

pub fn manifest_fingerprint(manifest: &SyncManifest) -> Result<String, ArtifactStoreError> {
    let bytes =
        serde_json::to_vec(manifest).map_err(|error| ArtifactStoreError::InvalidDerivedPath {
            path: "syncManifest".to_owned(),
            reason: format!("manifest must serialize: {error}"),
        })?;
    Ok(fingerprint_bytes(&bytes).to_string())
}

pub fn write_sync_manifest_artifact(
    bundle_path: impl AsRef<Path>,
    manifest: &SyncManifest,
) -> Result<BlobRecord, ArtifactStoreError> {
    let bytes =
        serde_json::to_vec(manifest).map_err(|error| ArtifactStoreError::InvalidDerivedPath {
            path: "syncManifest".to_owned(),
            reason: format!("manifest must serialize: {error}"),
        })?;
    let fingerprint = fingerprint_bytes(&bytes).to_string();
    let mut blobs = BlobStore::open(bundle_path)?;
    blobs.write_blob_atomic(
        BlobWriteIntent {
            artifact_id: "sync-manifest-current".to_owned(),
            artifact_kind: "syncManifest".to_owned(),
            stable_key: "syncManifest:local:current".to_owned(),
            schema_fingerprint: format!("sync-manifest-schema:v{SYNC_MANIFEST_SCHEMA_VERSION}"),
            generator_fingerprint: SYNC_MANIFEST_GENERATOR_VERSION.to_owned(),
            runtime_capability_fingerprint: None,
            source_fingerprint: None,
            graph_fingerprint: Some(fingerprint),
            output_profile_fingerprint: None,
            generation_parameters_json: serde_json::json!({
                "schemaVersion": SYNC_MANIFEST_SCHEMA_VERSION,
                "generatorVersion": SYNC_MANIFEST_GENERATOR_VERSION
            }),
            expected_fingerprint: None,
        },
        &bytes,
    )
}

struct ArtifactManifestRow {
    artifact_id: String,
    artifact_kind: String,
    stable_key: String,
    status: String,
    blob_relative_path: String,
    blob_fingerprint: String,
    byte_count: i64,
    source_fingerprint: Option<String>,
    graph_fingerprint: Option<String>,
    runtime_capability_fingerprint: Option<String>,
    output_profile_fingerprint: Option<String>,
    schema_fingerprint: String,
    generator_fingerprint: String,
    generation_parameters_json: String,
}

fn manifest_dependencies(
    store: &ArtifactStore,
    artifact_id: &str,
) -> Result<Vec<ManifestDependency>, ArtifactStoreError> {
    let mut dependencies = dependencies_for_artifact(store, artifact_id)?
        .into_iter()
        .map(dependency_summary)
        .collect::<Vec<_>>();
    dependencies.sort_by(|first, second| {
        (
            &first.kind,
            &first.key,
            first.target_start_us,
            first.target_duration_us,
            first.source_start_us,
            first.source_duration_us,
            &first.dirty_domain,
        )
            .cmp(&(
                &second.kind,
                &second.key,
                second.target_start_us,
                second.target_duration_us,
                second.source_start_us,
                second.source_duration_us,
                &second.dirty_domain,
            ))
    });
    Ok(dependencies)
}

fn dependency_summary(dependency: ArtifactDependency) -> ManifestDependency {
    ManifestDependency {
        kind: dependency.kind.as_str().to_owned(),
        key: dependency.dependency_key,
        target_start_us: dependency.target_range.map(|range| range.start_us),
        target_duration_us: dependency.target_range.map(|range| range.duration_us),
        source_start_us: dependency.source_range.map(|range| range.start_us),
        source_duration_us: dependency.source_range.map(|range| range.duration_us),
        dirty_domain: dependency.dirty_domain.map(|domain| format!("{domain:?}")),
        dependency_fingerprint: dependency.dependency_fingerprint,
    }
}

fn manifest_tombstones(
    store: &ArtifactStore,
) -> Result<Vec<ManifestTombstone>, ArtifactStoreError> {
    let derived_root = derived_root_path(&store.config.bundle_path);
    let mut statement = store
        .connection()
        .prepare(
            "SELECT artifact_id, blob_relative_path, blob_fingerprint, byte_count, reason
             FROM artifact_tombstone
             WHERE blob_relative_path IS NOT NULL
                AND blob_fingerprint IS NOT NULL
             ORDER BY artifact_id, blob_relative_path",
        )
        .map_err(|source| sqlite_error(store, source))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
            ))
        })
        .map_err(|source| sqlite_error(store, source))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|source| sqlite_error(store, source))?;
    rows.into_iter()
        .map(
            |(artifact_id, blob_relative_path, blob_fingerprint, byte_count, reason)| {
                validate_derived_relative_path(&derived_root, &blob_relative_path)?;
                Ok(ManifestTombstone {
                    artifact_id,
                    blob_relative_path,
                    blob_fingerprint,
                    byte_count: u64::try_from(byte_count).map_err(|_| {
                        ArtifactStoreError::RangeOverflow {
                            start_us: byte_count.unsigned_abs(),
                            duration_us: 0,
                        }
                    })?,
                    reason,
                })
            },
        )
        .collect()
}

fn sqlite_error(store: &ArtifactStore, source: rusqlite::Error) -> ArtifactStoreError {
    ArtifactStoreError::Sqlite {
        path: store.db_path.clone(),
        source,
    }
}
