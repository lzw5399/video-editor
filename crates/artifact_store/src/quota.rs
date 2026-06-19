use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::ArtifactStoreError;
use crate::gc::plan_garbage_collection;
use crate::schema::ArtifactStore;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuotaPolicy {
    pub byte_limit: Option<u64>,
    pub warning_ratio_per_mille: u16,
}

impl Default for QuotaPolicy {
    fn default() -> Self {
        Self {
            byte_limit: None,
            warning_ratio_per_mille: 850,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum QuotaSeverity {
    Normal,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuotaSnapshot {
    pub used_bytes: u64,
    pub reclaimable_bytes: u64,
    pub released_bytes: u64,
    pub source_media_bytes: u64,
    pub untracked_blob_bytes: u64,
    pub artifact_count: u64,
    pub tombstone_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuotaDisplayLabels {
    pub used_label: String,
    pub reclaimable_label: String,
    pub released_label: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct QuotaState {
    pub snapshot: QuotaSnapshot,
    pub policy: QuotaPolicy,
    pub severity: QuotaSeverity,
    pub quota_severity: String,
    pub status_label: String,
    pub labels: QuotaDisplayLabels,
    pub cleanup_available: bool,
}

pub fn set_quota_policy(
    store: &mut ArtifactStore,
    policy: QuotaPolicy,
) -> Result<(), ArtifactStoreError> {
    validate_policy(policy)?;
    store
        .connection()
        .execute(
            "INSERT INTO quota_state (
                quota_id, byte_limit, current_bytes, last_gc_at_unix_ms, updated_at_unix_ms
             ) VALUES ('default', ?1, 0, NULL, ?2)
             ON CONFLICT(quota_id) DO UPDATE SET
                byte_limit = excluded.byte_limit,
                updated_at_unix_ms = excluded.updated_at_unix_ms",
            params![optional_u64_i64(policy.byte_limit)?, now_unix_ms()],
        )
        .map_err(|source| sqlite_error(store, source))?;
    store
        .connection()
        .execute(
            "INSERT INTO store_metadata (key, value, updated_at_unix_ms)
             VALUES ('quota_warning_ratio_per_mille', ?1, ?2)
             ON CONFLICT(key) DO UPDATE SET
                value = excluded.value,
                updated_at_unix_ms = excluded.updated_at_unix_ms",
            params![policy.warning_ratio_per_mille.to_string(), now_unix_ms()],
        )
        .map_err(|source| sqlite_error(store, source))?;
    Ok(())
}

pub fn compute_quota_state(store: &ArtifactStore) -> Result<QuotaState, ArtifactStoreError> {
    let policy = quota_policy(store)?;
    let used_bytes = sum_i64(
        store,
        "SELECT COALESCE(SUM(byte_count), 0)
         FROM artifact
         WHERE status != 'tombstoned'",
    )?;
    let artifact_count = count_i64(
        store,
        "SELECT COUNT(*)
         FROM artifact
         WHERE status != 'tombstoned'",
    )?;
    let released_bytes = sum_i64(
        store,
        "SELECT COALESCE(SUM(byte_count), 0)
         FROM artifact_tombstone",
    )?;
    let tombstone_count = count_i64(store, "SELECT COUNT(*) FROM artifact_tombstone")?;
    let gc_plan = plan_garbage_collection(store)?;
    let snapshot = QuotaSnapshot {
        used_bytes,
        reclaimable_bytes: gc_plan.reclaimable_bytes,
        released_bytes,
        source_media_bytes: 0,
        untracked_blob_bytes: 0,
        artifact_count,
        tombstone_count,
    };
    let severity = quota_severity(&snapshot, policy);
    let status_label = if snapshot.released_bytes > 0 && snapshot.reclaimable_bytes == 0 {
        "缓存清理完成"
    } else if severity == QuotaSeverity::Warning {
        "缓存空间偏高"
    } else {
        "缓存空间正常"
    }
    .to_owned();
    let labels = format_quota_display_labels(&snapshot);
    let cleanup_available = snapshot.reclaimable_bytes > 0 || severity == QuotaSeverity::Warning;

    store
        .connection()
        .execute(
            "INSERT INTO quota_state (
                quota_id, byte_limit, current_bytes, last_gc_at_unix_ms, updated_at_unix_ms
             ) VALUES ('default', ?1, ?2, NULL, ?3)
             ON CONFLICT(quota_id) DO UPDATE SET
                current_bytes = excluded.current_bytes,
                updated_at_unix_ms = excluded.updated_at_unix_ms",
            params![
                optional_u64_i64(policy.byte_limit)?,
                i64::try_from(snapshot.used_bytes).map_err(|_| {
                    ArtifactStoreError::RangeOverflow {
                        start_us: snapshot.used_bytes,
                        duration_us: 0,
                    }
                })?,
                now_unix_ms(),
            ],
        )
        .map_err(|source| sqlite_error(store, source))?;

    Ok(QuotaState {
        snapshot,
        policy,
        severity,
        quota_severity: match severity {
            QuotaSeverity::Normal => "normal",
            QuotaSeverity::Warning => "warning",
        }
        .to_owned(),
        status_label,
        labels,
        cleanup_available,
    })
}

pub fn format_quota_display_labels(snapshot: &QuotaSnapshot) -> QuotaDisplayLabels {
    QuotaDisplayLabels {
        used_label: format_bytes(snapshot.used_bytes),
        reclaimable_label: format_bytes(snapshot.reclaimable_bytes),
        released_label: format_bytes(snapshot.released_bytes),
    }
}

fn quota_policy(store: &ArtifactStore) -> Result<QuotaPolicy, ArtifactStoreError> {
    let byte_limit = store
        .connection()
        .query_row(
            "SELECT byte_limit FROM quota_state WHERE quota_id = 'default'",
            [],
            |row| row.get::<_, Option<i64>>(0),
        )
        .optional()
        .map_err(|source| sqlite_error(store, source))?
        .flatten()
        .map(u64::try_from)
        .transpose()
        .map_err(|_| ArtifactStoreError::RangeOverflow {
            start_us: i64::MAX as u64 + 1,
            duration_us: 0,
        })?;
    let warning_ratio_per_mille = store
        .connection()
        .query_row(
            "SELECT value FROM store_metadata WHERE key = 'quota_warning_ratio_per_mille'",
            [],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|source| sqlite_error(store, source))?
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or_else(|| QuotaPolicy::default().warning_ratio_per_mille);
    let policy = QuotaPolicy {
        byte_limit,
        warning_ratio_per_mille,
    };
    validate_policy(policy)?;
    Ok(policy)
}

fn quota_severity(snapshot: &QuotaSnapshot, policy: QuotaPolicy) -> QuotaSeverity {
    let Some(byte_limit) = policy.byte_limit else {
        return QuotaSeverity::Normal;
    };
    if byte_limit == 0 {
        return QuotaSeverity::Warning;
    }
    let threshold = byte_limit.saturating_mul(u64::from(policy.warning_ratio_per_mille)) / 1000;
    if snapshot.used_bytes >= threshold {
        QuotaSeverity::Warning
    } else {
        QuotaSeverity::Normal
    }
}

fn validate_policy(policy: QuotaPolicy) -> Result<(), ArtifactStoreError> {
    if policy.warning_ratio_per_mille == 0 || policy.warning_ratio_per_mille > 1000 {
        return Err(ArtifactStoreError::InvalidDerivedPath {
            path: "quotaPolicy".to_owned(),
            reason: "warning ratio must be in 1..=1000 per-mille".to_owned(),
        });
    }
    Ok(())
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

fn sum_i64(store: &ArtifactStore, sql: &str) -> Result<u64, ArtifactStoreError> {
    let value = store
        .connection()
        .query_row(sql, [], |row| row.get::<_, i64>(0))
        .map_err(|source| sqlite_error(store, source))?;
    u64::try_from(value).map_err(|_| ArtifactStoreError::RangeOverflow {
        start_us: value.unsigned_abs(),
        duration_us: 0,
    })
}

fn count_i64(store: &ArtifactStore, sql: &str) -> Result<u64, ArtifactStoreError> {
    sum_i64(store, sql)
}

fn optional_u64_i64(value: Option<u64>) -> Result<Option<i64>, ArtifactStoreError> {
    value
        .map(|value| {
            i64::try_from(value).map_err(|_| ArtifactStoreError::RangeOverflow {
                start_us: value,
                duration_us: 0,
            })
        })
        .transpose()
}

fn now_unix_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or(0)
}

fn sqlite_error(store: &ArtifactStore, source: rusqlite::Error) -> ArtifactStoreError {
    ArtifactStoreError::Sqlite {
        path: store.db_path.clone(),
        source,
    }
}
