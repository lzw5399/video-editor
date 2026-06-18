# Phase 12: Media IO, Hardware Decode, And Frame/Texture Interop - Design

**Designed:** 2026-06-18
**Scope:** MEDIAIO-01 through MEDIAIO-05
**Status:** Ready for planning

## Design Goals

Phase 12 creates a Rust media IO layer that can probe, read, decode, and hand off media frames without forcing realtime preview through FFmpeg-generated artifacts or JS-owned frame buffers.

The design keeps ownership boundaries strict:

- `draft_model`, `draft_commands`, `engine_core`, and `render_graph` stay semantic.
- `media_runtime` owns shared media IO contracts and capability types.
- Platform runtime code owns native OS decoder interaction.
- Phase 11 realtime preview owns composition and presentation.
- FFmpeg remains fallback/probe/export/transcode.

## Proposed Crates And Modules

### Shared Contract Surface

```text
crates/media_runtime/src/
├── lib.rs
├── capabilities.rs        # extend current RuntimeCapabilityReport
├── media_io.rs            # MediaProbeService, MediaReader, stream/session traits
├── decoder.rs             # VideoDecoder, AudioDecoder, decode requests/results
├── frame.rs               # FramePool, frame leases, CPU/GPU frame storage
├── texture.rs             # TextureHandle, platform backend enums, device identity
├── color.rs               # color primaries/transfer/matrix/range metadata
└── fallback.rs            # fallback reason enums and selected path diagnostics
```

### Desktop Implementations

Recommended implementation split:

```text
crates/media_runtime_desktop/src/
├── lib.rs                 # desktop factory and existing DesktopFfmpegExecutor
├── ffmpeg_fallback.rs     # process-backed probe/decode fallback wrapper
├── capabilities.rs        # aggregate desktop capability report
├── windows.rs             # cfg(windows), Media Foundation / D3D path
└── macos.rs               # cfg(target_os = "macos"), AVFoundation / VideoToolbox / CoreVideo path
```

If platform dependency graphs become noisy, split later into:

```text
crates/media_runtime_windows/
crates/media_runtime_macos/
crates/media_runtime_ffmpeg/
```

The public trait surface should not change if that split happens.

## Core Type Shape

```rust
pub trait MediaProbeService {
    fn probe_material(&self, request: MediaProbeRequest) -> Result<MediaProbeReport, MediaIoError>;
    fn probe_runtime_capabilities(&self) -> RuntimeCapabilities;
}

pub trait MediaReader {
    fn reader_name(&self) -> &'static str;
    fn open(&self, request: MediaOpenRequest) -> Result<Box<dyn MediaSession>, MediaIoError>;
}

pub trait MediaSession {
    fn session_id(&self) -> MediaSessionId;
    fn streams(&self) -> &[MediaStreamInfo];
    fn video_decoder(&self, stream_id: StreamId) -> Result<Box<dyn VideoDecoder>, MediaIoError>;
    fn audio_decoder(&self, stream_id: StreamId) -> Result<Box<dyn AudioDecoder>, MediaIoError>;
}

pub trait VideoDecoder {
    fn decoder_name(&self) -> &'static str;
    fn decode_at(&mut self, request: VideoDecodeRequest) -> Result<DecodedVideoFrame, DecodeError>;
    fn flush(&mut self) -> Result<(), DecodeError>;
}

pub trait AudioDecoder {
    fn decoder_name(&self) -> &'static str;
    fn read_range(&mut self, request: AudioDecodeRequest) -> Result<DecodedAudioFrame, DecodeError>;
    fn flush(&mut self) -> Result<(), DecodeError>;
}
```

Key structs:

```rust
pub struct DecodedVideoFrame {
    pub handle_id: FrameHandleId,
    pub owner_session: MediaSessionId,
    pub playback_generation: Option<u64>,
    pub source_time_us: u64,
    pub duration_us: Option<u64>,
    pub frame_index: Option<u64>,
    pub dimensions: FrameDimensions,
    pub pixel_format: VideoPixelFormat,
    pub color: VideoColorMetadata,
    pub storage: VideoFrameStorage,
    pub release: FrameLeaseId,
}

pub enum VideoFrameStorage {
    Cpu(CpuFrameHandle),
    Texture(TextureHandle),
    PlatformOpaque(PlatformFrameHandle),
}

pub struct TextureHandle {
    pub handle_id: TextureHandleId,
    pub backend: TextureBackend,
    pub device_id: RuntimeDeviceId,
    pub owner_session: MediaSessionId,
    pub generation: u64,
    pub dimensions: FrameDimensions,
    pub pixel_format: VideoPixelFormat,
    pub color: VideoColorMetadata,
}

pub enum TextureBackend {
    D3d11Texture2D,
    D3d12Resource,
    MetalTexture,
    CoreVideoPixelBuffer,
}
```

Design rule: binding-facing APIs return `handle_id`, metadata, storage kind, and diagnostics. They do not return per-frame pixel bytes for handle-capable paths.

## Runtime Capabilities

Extend the current `RuntimeCapabilityReport` shape into a broader `RuntimeCapabilities` model:

```text
RuntimeCapabilities
├── ffmpeg: existing binary/filter/encoder posture
├── media_io:
│   ├── probe_service: native / ffprobe / hybrid
│   ├── windows: MediaFoundation, DXVA, D3D11/D3D12 texture interop
│   ├── macos: AVFoundation, VideoToolbox, CoreVideo, Metal interop
│   └── fallback_ladder: ordered decode paths
├── codecs:
│   ├── h264: supported/degraded/unsupported + reason
│   ├── hevc: supported/degraded/unsupported + reason
│   └── other codecs as reported, not promised
├── pixel_formats:
│   ├── nv12, bgra8, rgba8, p010, etc.
│   └── color metadata support
└── texture_interop:
    ├── available
    ├── backend
    ├── device_compatibility
    └── fallback reason
```

Capability status values should stay consistent with the current `Ready`, `Warning`, `Unavailable` pattern, but include domain-specific reasons:

- `UnsupportedCodec`
- `UnsupportedContainer`
- `UnsupportedPixelFormat`
- `UnsupportedColorSpace`
- `HardwareDecodeUnavailable`
- `TextureInteropUnavailable`
- `DeviceMismatch`
- `AllocationFailed`
- `PlatformApiFailed`
- `FfmpegUnavailable`
- `UserDisabledHardwareDecode`

## Windows Path

Primary flow:

```text
MediaOpenRequest
  -> Media Foundation Source Reader
  -> configure D3D device manager / DXVA where supported
  -> select video stream and output subtype
  -> decode frame
  -> receive D3D-backed sample or CPU sample
  -> wrap in FramePool lease
  -> return TextureHandle or CpuFrameHandle
```

Implementation notes:

- Use Media Foundation for source reading and transforms.
- Use a D3D device identity compatible with Phase 11 preview where possible.
- Prefer texture-backed output when a device-compatible path is proven.
- If Media Foundation hardware decode works but texture import to the preview device is not proven, return a platform opaque frame or CPU frame with `TextureInteropUnavailable`.
- Keep FFmpeg as fallback for unsupported containers/codecs/pixel formats.

Initial accepted path:

- H.264 MP4/MOV source fixture.
- Native capability report on Windows.
- Fallback reason test when native API is unavailable in CI or on non-Windows.

## macOS Path

Primary flow:

```text
MediaOpenRequest
  -> AVAssetReader / AVAssetReaderTrackOutput
  -> VideoToolbox decompression path where required
  -> CVPixelBuffer output
  -> CVMetalTextureCache creates Metal texture view where compatible
  -> wrap CVPixelBuffer/CVMetalTexture lifetime in FramePool lease
  -> return TextureHandle or CpuFrameHandle
```

Implementation notes:

- Treat `CVPixelBuffer` as the lifetime anchor for decoded image data.
- Treat `CVMetalTexture` / Metal texture views as derived from the pixel buffer and texture cache.
- Keep pixel buffer, texture cache, and texture view alive until the frame lease is released.
- Capture color metadata from sample/pixel-buffer attachments when available; otherwise return `unknown` plus diagnostic.
- Keep FFmpeg as fallback for unsupported formats or platform errors.

Initial accepted path:

- H.264 MP4/MOV source fixture.
- Native capability report on macOS.
- Fallback reason test when texture cache creation fails or is disabled.

## FFmpeg Fallback Path

FFmpeg remains required but should be treated as a fallback implementation of media IO rather than the realtime preview owner.

Fallback levels:

1. `NativeHardwareTexture`: native hardware decode to compatible texture.
2. `NativeHardwareCpuCopy`: native hardware decode but CPU copy is required.
3. `NativeSoftwareCpuFrame`: native software decode to CPU frame.
4. `FfmpegCpuFrame`: FFmpeg decode to CPU frame.
5. `FfmpegPreviewArtifact`: existing preview PNG/MP4 artifact path.

Every level returns a selected path and reason. Phase 11 can then display diagnostics and decide whether the frame is usable for realtime preview.

## Frame Pool And Lifetime Model

Frame pools are per runtime session and have explicit budgets:

- max decoded frames per stream
- max bytes for CPU frames
- max native textures per device
- max outstanding leases
- leak diagnostics on session close

Lease rules:

1. Decoder obtains a frame slot from `FramePool`.
2. Decoder fills native/CPU storage and returns `DecodedVideoFrame`.
3. Preview runtime retains the frame while rendering.
4. Preview runtime releases the frame after GPU submission no longer needs it.
5. Session close releases all remaining handles and records leaks.

Thread-safety rules:

- `FrameHandleId` and `TextureHandleId` are portable IDs.
- Native handles are not exposed to JS.
- Rust code must encode whether a handle is `Send`, main-thread-only, or device-thread-only.
- Platform texture access requires matching `RuntimeDeviceId`.

## No Unnecessary JS/Rust 4K Copies

Binding contract:

```json
{
  "kind": "decodedVideoFrame",
  "handleId": "frame_...",
  "storage": "texture",
  "textureHandleId": "texture_...",
  "width": 3840,
  "height": 2160,
  "pixelFormat": "nv12",
  "sourceTimeUs": 1234567,
  "fallback": null
}
```

Forbidden for handle-capable paths:

```json
{
  "bytes": [ ... millions of values ... ]
}
```

If UI needs an inspectable bitmap, it should request a thumbnail/preview artifact or an explicitly downscaled diagnostic frame, not the full native decode frame.

## Phase 11 Integration Contract

Phase 11 calls Phase 12 from material source-time requests:

```text
Resolved frame state from engine/render graph
  -> material_id + source_time_us + playback_generation
  -> media runtime decode request
  -> decoded frame handle
  -> preview compositor samples texture/CPU frame
  -> renderer reports parity/fallback diagnostics
```

Phase 12 must not:

- normalize drafts
- resolve timeline layer order
- apply transforms/keyframes/effects
- compile FFmpeg commands for export
- own preview presentation
- mutate `.veproj/project.json`

## Rollout Waves

### Wave 0: Contract And Tests

- Add shared media IO modules/types to `media_runtime`.
- Add fake reader/decoder/frame pool tests.
- Add fallback reason enums and serialization tests.
- Add contract guard ensuring handle responses do not include large byte buffers.

Exit gate:

```bash
cargo test -p media_runtime
pnpm run test:contracts
```

### Wave 1: Capability Reporting

- Extend runtime capabilities with media IO domains.
- Add desktop aggregate report.
- Add Windows/macOS stub reports behind `cfg`.
- Preserve current FFmpeg capability fields.

Exit gate:

```bash
cargo test -p media_runtime -p media_runtime_desktop capabilities
pnpm run test:bindings
```

### Wave 2: FFmpeg CPU Frame Fallback

- Implement fallback decoder that can produce a CPU frame handle for small controlled fixtures.
- Keep existing preview artifact fallback intact.
- Add structured fallback path diagnostics.

Exit gate:

```bash
cargo test -p media_runtime_desktop fallback
```

### Wave 3: macOS Native Path

- Implement macOS AVFoundation/VideoToolbox/CoreVideo path behind `cfg(target_os = "macos")`.
- Add CoreVideo pixel-buffer frame leases.
- Add Metal texture handle shape where texture cache succeeds.
- Add skipped tests on non-macOS with explicit reason.

Exit gate on macOS:

```bash
cargo test -p media_runtime_desktop macos
```

### Wave 4: Windows Native Path

- Implement Windows Media Foundation/DXVA/D3D path behind `cfg(windows)`.
- Add D3D texture handle shape where device-compatible texture decode succeeds.
- Add skipped tests on non-Windows with explicit reason.

Exit gate on Windows:

```bash
cargo test -p media_runtime_desktop windows
```

### Wave 5: Phase 11 Handoff

- Add adapter API for `RealtimePreviewRuntime` to request decoded material frames.
- Keep renderer-facing response handle-based.
- Add diagnostics showing selected decode path and fallback reason.

Exit gate:

```bash
cargo test -p media_runtime -p media_runtime_desktop
pnpm run test:bindings
pnpm run test:contracts
```

## Test Plan

Unit tests:

- fake decoder returns CPU frame lease and release clears outstanding count
- fake texture handle is rejected when device ID mismatches
- fallback reason serializes with stable camelCase names
- color metadata round-trips through serde
- session close releases leaked frame leases and reports diagnostics

Platform-gated tests:

- Windows capability report includes Media Foundation/DXVA/D3D fields
- macOS capability report includes AVFoundation/VideoToolbox/CoreVideo/Metal fields
- native path unavailable on wrong OS returns `Unavailable`, not panic

Integration tests:

- H.264 fixture decodes first frame through the best available path
- unsupported codec fixture falls back to FFmpeg or artifact path with reason
- binding handle response does not include raw full-frame bytes

Manual verification:

- On Windows, verify hardware decode capability report and first-frame decode on an H.264 MP4.
- On macOS, verify VideoToolbox/CoreVideo capability report and first-frame decode on an H.264 MOV/MP4.
- In Phase 11 preview, verify handle path does not visibly regress first-frame/seek latency compared with artifact fallback.

## Risks And Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| Native texture cannot be imported into Phase 11 `wgpu` device | Hardware decode works but preview still copies or falls back | Encode device identity and compatibility checks in `TextureHandle`; allow CPU/artifact fallback. |
| macOS Rust binding choice changes | Implementation churn | Keep public media_runtime traits independent from binding crate choice. |
| Windows D3D11/D3D12 interop mismatch | Texture handoff blocked | Start with D3D-backed capability report and CPU fallback; only mark texture path ready after real import test. |
| Hidden CPU copy harms 4K preview | MEDIAIO-04 failure | Telemetry and tests classify storage path and assert no byte payload in binding response. |
| Color metadata incomplete | Preview/export parity drift | Make color metadata explicit and diagnostic-bearing from the first contract wave. |

## Planner Notes

- Add package verification checkpoints before adding any new Cargo dependencies, especially macOS binding crates.
- Do not remove or rewrite existing FFmpeg capability/probe behavior.
- Do not modify Phase 10.1 artifacts.
- If Phase 11 implementation changes the preview device model, update `RuntimeDeviceId` and `TextureHandle` compatibility checks before native texture integration.
