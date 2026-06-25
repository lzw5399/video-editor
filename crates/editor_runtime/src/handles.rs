use std::collections::BTreeMap;

use media_runtime::{
    FrameDimensions, RuntimeDeviceId, TextureBackend, VideoColorMetadata, VideoPixelFormat,
};
use serde::{Deserialize, Serialize};

use crate::{RuntimeError, RuntimeErrorKind, RuntimeSessionId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HandleKind {
    RuntimeSession,
    ProjectSession,
    Media,
    Frame,
    Texture,
    Artifact,
}

impl HandleKind {
    fn token_prefix(self) -> &'static str {
        match self {
            Self::RuntimeSession => "runtimeHandle",
            Self::ProjectSession => "projectSessionHandle",
            Self::Media => "mediaHandle",
            Self::Frame => "frameHandle",
            Self::Texture => "textureHandle",
            Self::Artifact => "artifactHandle",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HandleToken {
    kind: HandleKind,
    token: String,
    owner_session: RuntimeSessionId,
    generation: u64,
}

impl HandleToken {
    pub fn from_raw_parts(
        kind: HandleKind,
        token: impl Into<String>,
        owner_session: RuntimeSessionId,
        generation: u64,
    ) -> Self {
        Self {
            kind,
            token: token.into(),
            owner_session,
            generation,
        }
    }

    pub fn kind(&self) -> HandleKind {
        self.kind
    }

    pub fn as_str(&self) -> &str {
        &self.token
    }

    pub fn owner_session(&self) -> &RuntimeSessionId {
        &self.owner_session
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn with_generation(&self, generation: u64) -> Self {
        let mut token = self.clone();
        token.generation = generation;
        token
    }

    fn key(&self) -> HandleRecordKey {
        HandleRecordKey {
            kind: self.kind,
            token: self.token.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextureHandleDescriptor {
    pub backend: TextureBackend,
    pub device: RuntimeDeviceId,
    pub dimensions: FrameDimensions,
    pub pixel_format: VideoPixelFormat,
    pub color: VideoColorMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextureResolveExpectation {
    pub descriptor: TextureHandleDescriptor,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandleAcquireRequest {
    kind: HandleKind,
    owner_session: RuntimeSessionId,
    texture: Option<TextureHandleDescriptor>,
    lease_expires_at_us: Option<u64>,
}

impl HandleAcquireRequest {
    pub fn runtime_session(owner_session: RuntimeSessionId) -> Self {
        Self::new(HandleKind::RuntimeSession, owner_session)
    }

    pub fn project_session(owner_session: RuntimeSessionId) -> Self {
        Self::new(HandleKind::ProjectSession, owner_session)
    }

    pub fn media(owner_session: RuntimeSessionId) -> Self {
        Self::new(HandleKind::Media, owner_session)
    }

    pub fn frame(owner_session: RuntimeSessionId) -> Self {
        Self::new(HandleKind::Frame, owner_session)
    }

    pub fn texture(owner_session: RuntimeSessionId, descriptor: TextureHandleDescriptor) -> Self {
        Self::new(HandleKind::Texture, owner_session).with_texture_descriptor(descriptor)
    }

    pub fn artifact(owner_session: RuntimeSessionId) -> Self {
        Self::new(HandleKind::Artifact, owner_session)
    }

    pub fn with_lease_expires_at_us(mut self, expires_at_us: u64) -> Self {
        self.lease_expires_at_us = Some(expires_at_us);
        self
    }

    fn new(kind: HandleKind, owner_session: RuntimeSessionId) -> Self {
        Self {
            kind,
            owner_session,
            texture: None,
            lease_expires_at_us: None,
        }
    }

    fn with_texture_descriptor(mut self, descriptor: TextureHandleDescriptor) -> Self {
        self.texture = Some(descriptor);
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HandleReleaseState {
    Explicit,
    CascadeClose,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HandleReleaseReport {
    pub token: HandleToken,
    pub kind: HandleKind,
    pub owner_session: RuntimeSessionId,
    pub generation: u64,
    pub outstanding_count: usize,
    pub release_state: HandleReleaseState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HandleResolution {
    pub token: HandleToken,
    pub kind: HandleKind,
    pub owner_session: RuntimeSessionId,
    pub generation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeLeakDiagnostic {
    pub token: HandleToken,
    pub kind: HandleKind,
    pub owner_session: RuntimeSessionId,
    pub generation: u64,
    pub outstanding_count: usize,
    pub release_state: HandleReleaseState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeCloseReport {
    pub owner_session: RuntimeSessionId,
    pub leak_diagnostics: Vec<RuntimeLeakDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct HandleRecordKey {
    kind: HandleKind,
    token: String,
}

#[derive(Debug, Clone)]
struct HandleRecord {
    token: HandleToken,
    texture: Option<TextureHandleDescriptor>,
    outstanding_count: usize,
    lease_expires_at_us: Option<u64>,
    release_state: Option<HandleReleaseState>,
}

#[derive(Debug, Default)]
pub struct HandleRegistry {
    next_id: u64,
    records: BTreeMap<HandleRecordKey, HandleRecord>,
}

impl HandleRegistry {
    pub fn acquire(&mut self, request: HandleAcquireRequest) -> Result<HandleToken, RuntimeError> {
        if request.kind == HandleKind::Texture && request.texture.is_none() {
            return Err(RuntimeError::new(
                RuntimeErrorKind::InvalidRequest,
                "texture handles require texture metadata",
            ));
        }
        let id_number = self.next_id.saturating_add(1);
        self.next_id = id_number;
        let token = HandleToken::from_raw_parts(
            request.kind,
            format!("{}-{id_number}", request.kind.token_prefix()),
            request.owner_session,
            1,
        );
        let record = HandleRecord {
            token: token.clone(),
            texture: request.texture,
            outstanding_count: 1,
            lease_expires_at_us: request.lease_expires_at_us,
            release_state: None,
        };
        self.records.insert(token.key(), record);
        Ok(token)
    }

    pub fn outstanding_count(&self, owner_session: &RuntimeSessionId, kind: HandleKind) -> usize {
        self.records
            .values()
            .filter(|record| {
                record.token.kind == kind
                    && record.token.owner_session == *owner_session
                    && record.release_state.is_none()
            })
            .count()
    }

    pub fn resolve(
        &self,
        owner_session: &RuntimeSessionId,
        token: &HandleToken,
        now_us: u64,
    ) -> Result<HandleResolution, RuntimeError> {
        let record = self.validated_record(owner_session, token, now_us)?;
        Ok(HandleResolution {
            token: token.clone(),
            kind: record.token.kind,
            owner_session: record.token.owner_session.clone(),
            generation: record.token.generation,
        })
    }

    pub fn resolve_texture(
        &self,
        owner_session: &RuntimeSessionId,
        token: &HandleToken,
        expectation: &TextureResolveExpectation,
        now_us: u64,
    ) -> Result<HandleResolution, RuntimeError> {
        let record = self.validated_record(owner_session, token, now_us)?;
        if record.token.kind != HandleKind::Texture {
            return Err(RuntimeError::new(
                RuntimeErrorKind::WrongKind,
                format!("handle {} is not a texture handle", token.as_str()),
            ));
        }
        let Some(descriptor) = record.texture.as_ref() else {
            return Err(RuntimeError::new(
                RuntimeErrorKind::WrongKind,
                format!("texture handle {} has no texture metadata", token.as_str()),
            ));
        };
        validate_texture_descriptor(descriptor, &expectation.descriptor)?;
        Ok(HandleResolution {
            token: token.clone(),
            kind: record.token.kind,
            owner_session: record.token.owner_session.clone(),
            generation: record.token.generation,
        })
    }

    pub fn retain(
        &mut self,
        owner_session: &RuntimeSessionId,
        token: &HandleToken,
        now_us: u64,
    ) -> Result<HandleResolution, RuntimeError> {
        let key = {
            let record = self.validated_record(owner_session, token, now_us)?;
            record.token.key()
        };
        let record = self
            .records
            .get_mut(&key)
            .expect("validated handle record must still exist");
        record.outstanding_count = record.outstanding_count.saturating_add(1);
        Ok(HandleResolution {
            token: token.clone(),
            kind: record.token.kind,
            owner_session: record.token.owner_session.clone(),
            generation: record.token.generation,
        })
    }

    pub fn release(
        &mut self,
        owner_session: &RuntimeSessionId,
        token: &HandleToken,
    ) -> Result<HandleReleaseReport, RuntimeError> {
        {
            let record = self.validated_record(owner_session, token, 0)?;
            if record.release_state.is_some() {
                return Err(double_release(token));
            }
        }
        let key = token.key();
        let kind;
        {
            let record = self
                .records
                .get_mut(&key)
                .expect("validated handle record must still exist");
            kind = record.token.kind;
            if record.outstanding_count > 1 {
                record.outstanding_count -= 1;
            } else {
                record.release_state = Some(HandleReleaseState::Explicit);
                record.outstanding_count = 0;
            }
        }
        let record = self
            .records
            .get(&key)
            .expect("released handle record must still exist");
        Ok(HandleReleaseReport {
            token: token.clone(),
            kind: record.token.kind,
            owner_session: record.token.owner_session.clone(),
            generation: record.token.generation,
            outstanding_count: self.outstanding_count(owner_session, kind),
            release_state: HandleReleaseState::Explicit,
        })
    }

    pub fn close_runtime_session(
        &mut self,
        owner_session: &RuntimeSessionId,
    ) -> RuntimeCloseReport {
        let keys = self
            .records
            .iter()
            .filter_map(|(key, record)| {
                if record.token.owner_session == *owner_session && record.release_state.is_none() {
                    Some(key.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let mut leak_diagnostics = Vec::with_capacity(keys.len());
        for key in keys {
            if let Some(record) = self.records.get_mut(&key) {
                let outstanding_count = record.outstanding_count;
                record.release_state = Some(HandleReleaseState::CascadeClose);
                record.outstanding_count = 0;
                leak_diagnostics.push(RuntimeLeakDiagnostic {
                    token: record.token.clone(),
                    kind: record.token.kind,
                    owner_session: record.token.owner_session.clone(),
                    generation: record.token.generation,
                    outstanding_count,
                    release_state: HandleReleaseState::CascadeClose,
                });
            }
        }
        RuntimeCloseReport {
            owner_session: owner_session.clone(),
            leak_diagnostics,
        }
    }

    fn validated_record(
        &self,
        owner_session: &RuntimeSessionId,
        token: &HandleToken,
        now_us: u64,
    ) -> Result<&HandleRecord, RuntimeError> {
        let record = self.records.get(&token.key()).ok_or_else(|| {
            RuntimeError::new(
                RuntimeErrorKind::UnknownHandle,
                format!("handle not found: {}", token.as_str()),
            )
        })?;
        if record.token.owner_session != *owner_session
            || token.owner_session != record.token.owner_session
        {
            return Err(RuntimeError::new(
                RuntimeErrorKind::WrongOwner,
                format!("handle {} owner session mismatch", token.as_str()),
            ));
        }
        if token.generation != record.token.generation {
            return Err(RuntimeError::new(
                RuntimeErrorKind::StaleGeneration,
                format!("handle {} generation is stale", token.as_str()),
            ));
        }
        if record.release_state.is_some() {
            return Err(double_release(token));
        }
        if record
            .lease_expires_at_us
            .is_some_and(|expires_at_us| now_us > expires_at_us)
        {
            return Err(RuntimeError::new(
                RuntimeErrorKind::LeaseExpired,
                format!("handle {} lease expired", token.as_str()),
            ));
        }
        Ok(record)
    }
}

fn validate_texture_descriptor(
    registered: &TextureHandleDescriptor,
    expected: &TextureHandleDescriptor,
) -> Result<(), RuntimeError> {
    if registered.backend != expected.backend || registered.device != expected.device {
        return Err(RuntimeError::new(
            RuntimeErrorKind::WrongDevice,
            "texture handle device identity does not match",
        ));
    }
    if registered.dimensions != expected.dimensions
        || registered.pixel_format != expected.pixel_format
        || registered.color != expected.color
    {
        return Err(RuntimeError::new(
            RuntimeErrorKind::TextureMetadataMismatch,
            "texture handle frame metadata does not match",
        ));
    }
    Ok(())
}

fn double_release(token: &HandleToken) -> RuntimeError {
    RuntimeError::new(
        RuntimeErrorKind::DoubleRelease,
        format!("handle {} was already released", token.as_str()),
    )
}
