use crate::fingerprint::ArtifactFingerprint;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobWriteIntent {
    pub artifact_id: String,
    pub artifact_kind: String,
    pub stable_key: String,
    pub schema_fingerprint: String,
    pub generator_fingerprint: String,
    pub runtime_capability_fingerprint: Option<String>,
    pub source_fingerprint: Option<String>,
    pub graph_fingerprint: Option<String>,
    pub output_profile_fingerprint: Option<String>,
    pub generation_parameters_json: serde_json::Value,
    pub expected_fingerprint: Option<ArtifactFingerprint>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobRecord {
    pub artifact_id: String,
    pub blob_relative_path: String,
    pub blob_fingerprint: ArtifactFingerprint,
    pub byte_count: u64,
}

#[derive(Debug)]
pub struct BlobStore;
