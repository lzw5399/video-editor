use draft_model::{MaterialId, Microseconds};
use media_runtime::{
    MediaSessionId, RuntimeDeviceId, TextureBackend, VideoColorMetadata,
};
use realtime_preview_runtime::gpu::{
    RealtimePreviewGpuBackend, RealtimePreviewGpuDevice, RealtimePreviewGpuDeviceDescriptor,
    RealtimePreviewTextureCache,
};
use realtime_preview_runtime::{PlaybackGeneration, PreviewFrameInput, TextureHandleDescriptor};

#[test]
fn texture_cache_accepts_external_texture_handles_without_cpu_pixels() {
    let device = RealtimePreviewGpuDevice::bootstrap(RealtimePreviewGpuDeviceDescriptor {
        backend: RealtimePreviewGpuBackend::Mock,
        label: Some("external-texture-cache-test".to_owned()),
    })
    .expect("mock device is enough for handle bookkeeping");
    let mut cache = RealtimePreviewTextureCache::new();
    let descriptor = TextureHandleDescriptor::new(
        MaterialId::new("dashcam-video"),
        Microseconds::new(33_333),
        "macos-metal-texture-42",
        MediaSessionId("session-texture-1".to_owned()),
        PlaybackGeneration::new(7),
        "metalTexture",
        RuntimeDeviceId {
            backend: TextureBackend::MetalTexture,
            adapter_id: "metal-adapter".to_owned(),
            device_id: "metal-device".to_owned(),
        },
        1920,
        1080,
        "nv12",
        VideoColorMetadata::unknown_with_diagnostic("test texture color"),
    )
    .expect("external texture descriptor is valid");

    let texture = cache
        .upload_frame(
            &device,
            PreviewFrameInput::TextureHandle(descriptor.clone()),
        )
        .expect("external texture handles should be retained for the GPU compositor");

    assert_eq!(texture.material_id, MaterialId::new("dashcam-video"));
    assert_eq!(texture.source_position, Microseconds::new(33_333));
    assert_eq!(texture.playback_generation, PlaybackGeneration::new(7));
    assert_eq!(texture.width, 1920);
    assert_eq!(texture.height, 1080);
    assert_eq!(texture.cpu_pixels(), None);
    assert_eq!(texture.external_handle(), Some(&descriptor));
    assert_eq!(cache.get(texture.id), Some(&texture));
}
