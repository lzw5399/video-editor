use std::fs::File;
use std::path::Path;

use crate::ArtifactStoreError;

pub const ARTIFACT_FINGERPRINT_PREFIX: &str = "blake3:v1:";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactFingerprint {
    value: String,
}

impl ArtifactFingerprint {
    pub fn from_prefixed(value: impl Into<String>) -> Result<Self, ArtifactStoreError> {
        let value = value.into();
        if !value.starts_with(ARTIFACT_FINGERPRINT_PREFIX)
            || value.len() == ARTIFACT_FINGERPRINT_PREFIX.len()
        {
            return Err(ArtifactStoreError::InvalidDerivedPath {
                path: value,
                reason: "fingerprint must use blake3:v1 prefix".to_owned(),
            });
        }
        Ok(Self { value })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }

    pub fn content_hex(&self) -> &str {
        &self.value[ARTIFACT_FINGERPRINT_PREFIX.len()..]
    }
}

impl std::fmt::Display for ArtifactFingerprint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.value)
    }
}

pub fn fingerprint_bytes(bytes: &[u8]) -> ArtifactFingerprint {
    ArtifactFingerprint {
        value: format!(
            "{ARTIFACT_FINGERPRINT_PREFIX}{}",
            blake3::hash(bytes).to_hex()
        ),
    }
}

pub fn fingerprint_file(path: impl AsRef<Path>) -> Result<ArtifactFingerprint, ArtifactStoreError> {
    let path = path.as_ref();
    let file = File::open(path).map_err(|source| ArtifactStoreError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    let mut hasher = blake3::Hasher::new();
    hasher
        .update_reader(file)
        .map_err(|source| ArtifactStoreError::Io {
            path: path.to_path_buf(),
            source,
        })?;
    ArtifactFingerprint::from_prefixed(format!(
        "{ARTIFACT_FINGERPRINT_PREFIX}{}",
        hasher.finalize().to_hex()
    ))
}
