#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactFingerprint {
    value: String,
}

impl ArtifactFingerprint {
    pub fn as_str(&self) -> &str {
        &self.value
    }
}
