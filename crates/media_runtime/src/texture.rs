use std::any::Any;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::rc::Rc;

use serde::{Deserialize, Serialize};

use crate::{FrameDimensions, MediaSessionId, VideoColorMetadata, VideoPixelFormat};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TextureBackend {
    D3d11Texture2D,
    D3d12Resource,
    MetalTexture,
    CoreVideoPixelBuffer,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TextureHandleId(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeDeviceId {
    pub backend: TextureBackend,
    pub adapter_id: String,
    pub device_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextureHandle {
    pub handle_id: TextureHandleId,
    pub owner_session: MediaSessionId,
    pub generation: u64,
    pub backend: TextureBackend,
    pub device_id: RuntimeDeviceId,
    pub dimensions: FrameDimensions,
    pub pixel_format: VideoPixelFormat,
    pub color: VideoColorMetadata,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NativeTextureLeaseResourceKind {
    WgpuTexture,
    MacosCoreVideoMetalTexture,
    WindowsD3dTexture,
    PlatformOpaque,
}

#[derive(Clone)]
pub struct NativeTextureLease {
    handle: TextureHandle,
    resource_kind: NativeTextureLeaseResourceKind,
    resource: Rc<dyn Any>,
}

impl NativeTextureLease {
    pub fn handle(&self) -> &TextureHandle {
        &self.handle
    }

    pub fn resource_kind(&self) -> NativeTextureLeaseResourceKind {
        self.resource_kind
    }

    pub fn resource_as<T: 'static>(&self) -> Option<Rc<T>> {
        Rc::clone(&self.resource).downcast::<T>().ok()
    }
}

impl fmt::Debug for NativeTextureLease {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("NativeTextureLease")
            .field("handle", &self.handle)
            .field("resource_kind", &self.resource_kind)
            .finish_non_exhaustive()
    }
}

impl PartialEq for NativeTextureLease {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle && self.resource_kind == other.resource_kind
    }
}

impl Eq for NativeTextureLease {}

#[derive(Debug, Clone)]
pub struct NativeTextureLeaseRegistry {
    leases: Rc<RefCell<BTreeMap<TextureHandleId, NativeTextureLease>>>,
}

impl Default for NativeTextureLeaseRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl NativeTextureLeaseRegistry {
    pub fn new() -> Self {
        Self {
            leases: Rc::new(RefCell::new(BTreeMap::new())),
        }
    }

    pub fn register_resource<T: 'static>(
        &self,
        handle: TextureHandle,
        resource_kind: NativeTextureLeaseResourceKind,
        resource: T,
    ) -> Result<NativeTextureLease, NativeTextureLeaseError> {
        if handle.handle_id.0.trim().is_empty() {
            return Err(NativeTextureLeaseError::new(
                NativeTextureLeaseErrorKind::InvalidHandle,
                "native texture lease handle id must be present",
            ));
        }
        let lease = NativeTextureLease {
            handle,
            resource_kind,
            resource: Rc::new(resource),
        };
        self.leases
            .borrow_mut()
            .insert(lease.handle.handle_id.clone(), lease.clone());
        Ok(lease)
    }

    pub fn resolve(
        &self,
        expected: &TextureHandle,
    ) -> Result<NativeTextureLease, NativeTextureLeaseError> {
        let leases = self.leases.borrow();
        let lease = leases.get(&expected.handle_id).ok_or_else(|| {
            NativeTextureLeaseError::new(
                NativeTextureLeaseErrorKind::NotRegistered,
                format!(
                    "native texture lease {} is not registered",
                    expected.handle_id.0
                ),
            )
        })?;
        validate_expected_handle(expected, &lease.handle)?;
        Ok(lease.clone())
    }

    pub fn unregister(&self, handle_id: &TextureHandleId) -> Option<NativeTextureLease> {
        self.leases.borrow_mut().remove(handle_id)
    }

    pub fn contains(&self, handle_id: &TextureHandleId) -> bool {
        self.leases.borrow().contains_key(handle_id)
    }

    pub fn len(&self) -> usize {
        self.leases.borrow().len()
    }

    pub fn is_empty(&self) -> bool {
        self.leases.borrow().is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NativeTextureLeaseErrorKind {
    InvalidHandle,
    NotRegistered,
    OwnerSessionMismatch,
    StaleGeneration,
    BackendMismatch,
    DeviceMismatch,
    DimensionMismatch,
    PixelFormatMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeTextureLeaseError {
    pub kind: NativeTextureLeaseErrorKind,
    pub message: String,
}

impl NativeTextureLeaseError {
    fn new(kind: NativeTextureLeaseErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for NativeTextureLeaseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for NativeTextureLeaseError {}

fn validate_expected_handle(
    expected: &TextureHandle,
    registered: &TextureHandle,
) -> Result<(), NativeTextureLeaseError> {
    if expected.owner_session != registered.owner_session {
        return Err(NativeTextureLeaseError::new(
            NativeTextureLeaseErrorKind::OwnerSessionMismatch,
            "native texture lease owner session does not match the decoded frame",
        ));
    }
    if expected.generation != registered.generation {
        return Err(NativeTextureLeaseError::new(
            NativeTextureLeaseErrorKind::StaleGeneration,
            "native texture lease generation does not match the decoded frame",
        ));
    }
    if expected.backend != registered.backend {
        return Err(NativeTextureLeaseError::new(
            NativeTextureLeaseErrorKind::BackendMismatch,
            "native texture lease backend does not match the decoded frame",
        ));
    }
    if expected.device_id != registered.device_id {
        return Err(NativeTextureLeaseError::new(
            NativeTextureLeaseErrorKind::DeviceMismatch,
            "native texture lease device identity does not match the decoded frame",
        ));
    }
    if expected.dimensions != registered.dimensions {
        return Err(NativeTextureLeaseError::new(
            NativeTextureLeaseErrorKind::DimensionMismatch,
            "native texture lease dimensions do not match the decoded frame",
        ));
    }
    if expected.pixel_format != registered.pixel_format {
        return Err(NativeTextureLeaseError::new(
            NativeTextureLeaseErrorKind::PixelFormatMismatch,
            "native texture lease pixel format does not match the decoded frame",
        ));
    }
    Ok(())
}
