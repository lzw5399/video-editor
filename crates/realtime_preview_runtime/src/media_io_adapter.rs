use std::collections::BTreeMap;
use std::path::PathBuf;

use draft_model::{MaterialId, Microseconds};
use media_runtime::{
    DecodeError, DecodedVideoFrame, MediaIoError, MediaIoFallbackReason, MediaIoFallbackSelection,
    MediaOpenRequest, MediaReader, MediaSession, RuntimeDeviceId, SelectedDecodePath, StreamId,
    NativeTextureLeaseRegistry, VideoDecodeRequest, VideoDecoder, VideoFrameStorage,
};
use serde::{Deserialize, Serialize};

use crate::{
    PlaybackGeneration, PreviewFrameInput, PreviewFrameProvider, PreviewFrameProviderError,
    RealtimePreviewFallbackReason, TextureHandleDescriptor, fallback_reason_from_media_io,
};

const PROVIDER_NAME: &str = "media-io-frame-provider";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PreviewFrameStoragePreference {
    Any,
    Cpu,
    Texture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PreviewFrameStorageKind {
    Cpu,
    Texture,
    PlatformOpaque,
    ArtifactFallback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewDecodeDeviceContext {
    pub preview_device: Option<RuntimeDeviceId>,
    pub texture_compatible: bool,
    pub reason: Option<String>,
}

impl PreviewDecodeDeviceContext {
    pub fn cpu_only() -> Self {
        Self {
            preview_device: None,
            texture_compatible: false,
            reason: Some("preview requested CPU or handle-only decode".to_owned()),
        }
    }

    pub fn compatible(preview_device: RuntimeDeviceId) -> Self {
        Self {
            preview_device: Some(preview_device),
            texture_compatible: true,
            reason: None,
        }
    }

    pub fn unproven(reason: impl Into<String>) -> Self {
        Self {
            preview_device: None,
            texture_compatible: false,
            reason: Some(reason.into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewMaterialDecodeSource {
    pub material_id: MaterialId,
    pub material_uri: PathBuf,
    pub stream_id: StreamId,
    pub selected_path: SelectedDecodePath,
    pub fallback_selection: Option<MediaIoFallbackSelection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewMaterialDecodeRequest {
    pub material_id: MaterialId,
    pub source_position: Microseconds,
    pub playback_generation: PlaybackGeneration,
    pub desired_storage: PreviewFrameStoragePreference,
    pub device: PreviewDecodeDeviceContext,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewDecodeDiagnostic {
    pub material_id: MaterialId,
    pub selected_path: SelectedDecodePath,
    pub fallback_reason: Option<MediaIoFallbackReason>,
    pub storage_kind: PreviewFrameStorageKind,
    pub texture_compatible: bool,
    pub preview_device: Option<RuntimeDeviceId>,
    pub native_device: Option<RuntimeDeviceId>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewMaterialDecodeOutput {
    pub material_id: MaterialId,
    pub source_position: Microseconds,
    pub playback_generation: PlaybackGeneration,
    pub decoded_frame: DecodedVideoFrame,
    pub storage_kind: PreviewFrameStorageKind,
    pub selected_path: SelectedDecodePath,
    pub fallback: Option<RealtimePreviewFallbackReason>,
    pub stale_rejected: bool,
    pub diagnostics: Vec<PreviewDecodeDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewMediaIoTelemetry {
    pub decode_request_count: u64,
    pub presentable_frame_count: u64,
    pub stale_rejected_count: u64,
    pub fallback_count: u64,
}

impl PreviewMediaIoTelemetry {
    fn new() -> Self {
        Self {
            decode_request_count: 0,
            presentable_frame_count: 0,
            stale_rejected_count: 0,
            fallback_count: 0,
        }
    }
}

pub struct MediaIoFrameProvider {
    reader: Box<dyn MediaReader>,
    materials: BTreeMap<MaterialId, RegisteredMaterial>,
    telemetry: PreviewMediaIoTelemetry,
    desired_storage: PreviewFrameStoragePreference,
    device: PreviewDecodeDeviceContext,
    native_texture_registry: Option<NativeTextureLeaseRegistry>,
}

impl MediaIoFrameProvider {
    pub fn new(reader: Box<dyn MediaReader>) -> Self {
        Self {
            reader,
            materials: BTreeMap::new(),
            telemetry: PreviewMediaIoTelemetry::new(),
            desired_storage: PreviewFrameStoragePreference::Texture,
            device: PreviewDecodeDeviceContext::unproven(
                "preview GPU device context has not been attached to media IO",
            ),
            native_texture_registry: None,
        }
    }

    pub fn with_desired_storage(mut self, desired_storage: PreviewFrameStoragePreference) -> Self {
        self.desired_storage = desired_storage;
        self
    }

    pub fn with_preview_device_context(mut self, device: PreviewDecodeDeviceContext) -> Self {
        self.device = device;
        self
    }

    pub fn with_native_texture_registry(mut self, registry: NativeTextureLeaseRegistry) -> Self {
        self.native_texture_registry = Some(registry);
        self
    }

    pub fn desired_storage(&self) -> PreviewFrameStoragePreference {
        self.desired_storage
    }

    pub fn preview_device_context(&self) -> &PreviewDecodeDeviceContext {
        &self.device
    }

    pub fn register_material(
        &mut self,
        source: PreviewMaterialDecodeSource,
    ) -> Result<(), MediaIoHandoffError> {
        let session = self
            .reader
            .open(MediaOpenRequest {
                material_uri: source.material_uri.clone(),
                requested_streams: vec![source.stream_id],
            })
            .map_err(|source_error| MediaIoHandoffError::Open {
                material_id: source.material_id.clone(),
                source: source_error,
            })?;
        let decoder = session
            .video_decoder(source.stream_id)
            .map_err(|source_error| MediaIoHandoffError::Decoder {
                material_id: source.material_id.clone(),
                source: source_error,
            })?;

        self.materials.insert(
            source.material_id.clone(),
            RegisteredMaterial {
                source,
                _session: session,
                decoder,
            },
        );
        Ok(())
    }

    pub fn decode_material_frame(
        &mut self,
        request: PreviewMaterialDecodeRequest,
        active_generation: PlaybackGeneration,
    ) -> Result<PreviewMaterialDecodeOutput, MediaIoHandoffError> {
        self.telemetry.decode_request_count = self.telemetry.decode_request_count.saturating_add(1);

        let material = self
            .materials
            .get_mut(&request.material_id)
            .ok_or_else(|| MediaIoHandoffError::MaterialNotRegistered {
                material_id: request.material_id.clone(),
            })?;
        let frame = material
            .decoder
            .decode_at(VideoDecodeRequest {
                source_time_us: request.source_position.get(),
                playback_generation: Some(request.playback_generation.get()),
            })
            .map_err(|source| MediaIoHandoffError::Decode {
                material_id: request.material_id.clone(),
                source,
            })?;

        let selected_path = material
            .source
            .fallback_selection
            .as_ref()
            .map(|selection| selection.selected_path)
            .unwrap_or(material.source.selected_path);
        let fallback_reason = material
            .source
            .fallback_selection
            .as_ref()
            .and_then(|selection| selection.reason);
        let storage_kind = storage_kind_for(&frame.storage);
        let native_device = native_device_for(&frame.storage);
        let texture_compatible = texture_compatible(&request.device, native_device.as_ref());
        let stale_rejected = request.playback_generation != active_generation
            || frame.playback_generation != Some(active_generation.get());
        let fallback = if stale_rejected {
            Some(RealtimePreviewFallbackReason::StaleGeneration)
        } else {
            fallback_reason_from_media_io(selected_path, fallback_reason)
        };
        if stale_rejected {
            self.telemetry.stale_rejected_count =
                self.telemetry.stale_rejected_count.saturating_add(1);
        }
        if fallback.is_some() {
            self.telemetry.fallback_count = self.telemetry.fallback_count.saturating_add(1);
        }

        let diagnostics = vec![PreviewDecodeDiagnostic {
            material_id: request.material_id.clone(),
            selected_path,
            fallback_reason,
            storage_kind,
            texture_compatible,
            preview_device: request.device.preview_device.clone(),
            native_device,
            message: diagnostic_message(
                selected_path,
                fallback_reason,
                storage_kind,
                texture_compatible,
                stale_rejected,
            ),
        }];

        Ok(PreviewMaterialDecodeOutput {
            material_id: request.material_id,
            source_position: request.source_position,
            playback_generation: request.playback_generation,
            decoded_frame: frame,
            storage_kind,
            selected_path,
            fallback,
            stale_rejected,
            diagnostics,
        })
    }

    pub fn telemetry(&self) -> &PreviewMediaIoTelemetry {
        &self.telemetry
    }
}

impl PreviewFrameProvider for MediaIoFrameProvider {
    fn provider_name(&self) -> &'static str {
        PROVIDER_NAME
    }

    fn frame_for(
        &mut self,
        material_id: &MaterialId,
        source_position: Microseconds,
        playback_generation: PlaybackGeneration,
    ) -> Result<PreviewFrameInput, PreviewFrameProviderError> {
        let output = self
            .decode_material_frame(
                PreviewMaterialDecodeRequest {
                    material_id: material_id.clone(),
                    source_position,
                    playback_generation,
                    desired_storage: self.desired_storage,
                    device: self.device.clone(),
                },
                playback_generation,
            )
            .map_err(|error| {
                PreviewFrameProviderError::unavailable(
                    self.provider_name(),
                    material_id.clone(),
                    source_position,
                    playback_generation,
                    format!("media IO decode handoff failed: {error}"),
                )
            })?;

        let input = preview_input_from_media_io_output(
            self.provider_name(),
            output,
            self.native_texture_registry.as_ref(),
        )?;
        self.telemetry.presentable_frame_count =
            self.telemetry.presentable_frame_count.saturating_add(1);
        Ok(input)
    }
}

struct RegisteredMaterial {
    source: PreviewMaterialDecodeSource,
    _session: Box<dyn MediaSession>,
    decoder: Box<dyn VideoDecoder>,
}

#[derive(Debug)]
pub enum MediaIoHandoffError {
    MaterialNotRegistered {
        material_id: MaterialId,
    },
    Open {
        material_id: MaterialId,
        source: MediaIoError,
    },
    Decoder {
        material_id: MaterialId,
        source: MediaIoError,
    },
    Decode {
        material_id: MaterialId,
        source: DecodeError,
    },
}

impl std::fmt::Display for MediaIoHandoffError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MaterialNotRegistered { material_id } => {
                write!(
                    formatter,
                    "material {} is not registered for media IO",
                    material_id.as_str()
                )
            }
            Self::Open {
                material_id,
                source,
            } => {
                write!(
                    formatter,
                    "failed to open media IO material {}: {source}",
                    material_id.as_str()
                )
            }
            Self::Decoder {
                material_id,
                source,
            } => {
                write!(
                    formatter,
                    "failed to create media IO decoder for {}: {source}",
                    material_id.as_str()
                )
            }
            Self::Decode {
                material_id,
                source,
            } => {
                write!(
                    formatter,
                    "failed to decode media IO frame for {}: {source}",
                    material_id.as_str()
                )
            }
        }
    }
}

impl std::error::Error for MediaIoHandoffError {}

fn storage_kind_for(storage: &VideoFrameStorage) -> PreviewFrameStorageKind {
    match storage {
        VideoFrameStorage::Cpu(_) => PreviewFrameStorageKind::Cpu,
        VideoFrameStorage::Texture(_) => PreviewFrameStorageKind::Texture,
        VideoFrameStorage::PlatformOpaque(_) => PreviewFrameStorageKind::PlatformOpaque,
    }
}

fn preview_input_from_media_io_output(
    provider_name: &'static str,
    output: PreviewMaterialDecodeOutput,
    native_texture_registry: Option<&NativeTextureLeaseRegistry>,
) -> Result<PreviewFrameInput, PreviewFrameProviderError> {
    if output.stale_rejected {
        return Err(PreviewFrameProviderError::unavailable(
            provider_name,
            output.material_id,
            output.source_position,
            output.playback_generation,
            "decoded frame belongs to a stale playback generation",
        ));
    }

    if let Some(fallback) = output.fallback {
        return Err(PreviewFrameProviderError::unavailable(
            provider_name,
            output.material_id,
            output.source_position,
            output.playback_generation,
            format!("media IO fallback {fallback:?} cannot be used as product compositor input"),
        ));
    }

    match output.storage_kind {
        PreviewFrameStorageKind::Texture => {
            let texture_compatible = output
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.texture_compatible);
            if !texture_compatible {
                return Err(PreviewFrameProviderError::unavailable(
                    provider_name,
                    output.material_id,
                    output.source_position,
                    output.playback_generation,
                    "native texture frame is not proven compatible with the preview GPU device",
                ));
            }

            let material_id = output.material_id;
            let source_position = output.source_position;
            let playback_generation = output.playback_generation;
            let descriptor = TextureHandleDescriptor::from_decoded_frame(
                material_id.clone(),
                source_position,
                &output.decoded_frame,
            )
            .map_err(|error| {
                PreviewFrameProviderError::invalid_frame(
                    provider_name,
                    Some(material_id.clone()),
                    Some(source_position),
                    Some(playback_generation),
                    error,
                )
            })?
            .ok_or_else(|| {
                PreviewFrameProviderError::unavailable(
                    provider_name,
                    material_id,
                    source_position,
                    playback_generation,
                    "media IO reported texture storage without a texture handle",
                )
            })?;
            let expected_handle = descriptor.to_texture_handle().map_err(|error| {
                PreviewFrameProviderError::invalid_frame(
                    provider_name,
                    Some(descriptor.material_id.clone()),
                    Some(descriptor.source_position),
                    Some(descriptor.playback_generation),
                    error,
                )
            })?;
            let registry = native_texture_registry.ok_or_else(|| {
                PreviewFrameProviderError::unavailable(
                    provider_name,
                    descriptor.material_id.clone(),
                    descriptor.source_position,
                    descriptor.playback_generation,
                    "native texture lease registry is not attached to media IO",
                )
            })?;
            registry.resolve(&expected_handle).map_err(|error| {
                PreviewFrameProviderError::unavailable(
                    provider_name,
                    descriptor.material_id.clone(),
                    descriptor.source_position,
                    descriptor.playback_generation,
                    format!("native texture lease unavailable: {error}"),
                )
            })?;
            Ok(PreviewFrameInput::TextureHandle(descriptor))
        }
        PreviewFrameStorageKind::Cpu => Err(PreviewFrameProviderError::unavailable(
            provider_name,
            output.material_id,
            output.source_position,
            output.playback_generation,
            "media IO CPU frame handle does not include compositor-ready RGBA pixels",
        )),
        PreviewFrameStorageKind::PlatformOpaque => Err(PreviewFrameProviderError::unavailable(
            provider_name,
            output.material_id,
            output.source_position,
            output.playback_generation,
            "platform-opaque media IO frame cannot be sampled by the realtime GPU compositor",
        )),
        PreviewFrameStorageKind::ArtifactFallback => Err(PreviewFrameProviderError::unavailable(
            provider_name,
            output.material_id,
            output.source_position,
            output.playback_generation,
            "FFmpeg preview artifacts cannot be used as product realtime compositor input",
        )),
    }
}

fn native_device_for(storage: &VideoFrameStorage) -> Option<RuntimeDeviceId> {
    match storage {
        VideoFrameStorage::Texture(texture) => Some(texture.device_id.clone()),
        VideoFrameStorage::Cpu(_) | VideoFrameStorage::PlatformOpaque(_) => None,
    }
}

fn texture_compatible(
    device: &PreviewDecodeDeviceContext,
    native_device: Option<&RuntimeDeviceId>,
) -> bool {
    device.texture_compatible
        && device
            .preview_device
            .as_ref()
            .zip(native_device)
            .map(|(preview, native)| preview == native)
            .unwrap_or(false)
}

fn diagnostic_message(
    selected_path: SelectedDecodePath,
    fallback_reason: Option<MediaIoFallbackReason>,
    storage_kind: PreviewFrameStorageKind,
    texture_compatible: bool,
    stale_rejected: bool,
) -> String {
    if stale_rejected {
        return "decoded frame rejected because playback generation is stale".to_owned();
    }
    if let Some(reason) = fallback_reason {
        return format!(
            "media IO selected {selected_path:?} with {reason:?}; storage={storage_kind:?}; textureCompatible={texture_compatible}"
        );
    }
    format!(
        "media IO selected {selected_path:?}; storage={storage_kind:?}; textureCompatible={texture_compatible}"
    )
}
