use draft_model::{MaterialId, Microseconds};
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
        PlaybackGeneration::new(7),
        "metalTexture",
        1920,
        1080,
        "nv12",
    )
    .expect("external texture descriptor is valid");

    let texture = cache
        .upload_frame(&device, PreviewFrameInput::TextureHandle(descriptor.clone()))
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
