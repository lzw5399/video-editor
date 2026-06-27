# Phase 12: Media IO, Hardware Decode, And Frame/Texture Interop - Research

**Researched:** 2026-06-18
**Domain:** Desktop media IO, hardware decode, platform texture interop, Rust runtime boundaries
**Confidence:** MEDIUM-HIGH

## User Constraints

### Locked Decisions

- UI emits commands; Rust core owns project and timeline semantics. [VERIFIED: AGENTS.md]
- No UI or binding code may directly construct FFmpeg commands. [VERIFIED: AGENTS.md]
- `.veproj/project.json` is canonical semantic state; render graphs, scripts, thumbnails, waveforms, proxy files, preview caches, decoded frames, and texture handles are derived artifacts. [VERIFIED: AGENTS.md]
- Core time math uses integer microseconds, frame indices, or rational frame rates. [VERIFIED: AGENTS.md]
- Phase 12 must introduce `MediaProbeService`, `MediaReader`, `VideoDecoder`, `AudioDecoder`, `FramePool`, `DecodedVideoFrame`, `TextureHandle`, and `RuntimeCapabilities` style abstractions. [VERIFIED: user request]
- Windows path is Media Foundation / DXVA / D3D texture interop, with software/FFmpeg fallback. [VERIFIED: user request]
- macOS path is AVFoundation / VideoToolbox / CoreVideo / Metal texture interop, with software/FFmpeg fallback. [VERIFIED: user request]
- Phase 12 feeds Phase 11 realtime preview without taking over rendering semantics. [VERIFIED: user request]
- Do not modify product source files, Phase 10.1 files, or commit. [VERIFIED: user request]

### the agent's Discretion

- Start Phase 12 platform paths as `cfg` modules under `media_runtime_desktop`; keep shared trait/type surface in `media_runtime` and leave a clean split seam for later `media_runtime_windows`, `media_runtime_macos`, and `media_runtime_ffmpeg` crates. [RESOLVED]
- First native hardware-decode acceptance target is H.264 MP4/MOV on Windows and macOS. HEVC, ProRes, AV1, and other codecs are capability-reported and degraded unless a later task explicitly proves support. [RESOLVED]

### Deferred Ideas (OUT OF SCOPE)

- Full mobile runtime implementation. [VERIFIED: .planning/notes/production-editor-architecture-decisions.md]
- Full task scheduler, audio output engine, artifact store, incremental graph, effects registry, and hardware encoder selection. [VERIFIED: .planning/ROADMAP.md]

## Summary

Phase 12 should create a media IO layer that is narrower than the preview renderer and broader than the existing FFmpeg process executor. The current codebase exposes `FfmpegExecutor`, ffprobe-based material probe metadata, and FFmpeg runtime capability reports; preview currently prepares render graph output and uses FFmpeg to materialize cached PNG/MP4 artifacts. [VERIFIED: `crates/media_runtime/src/lib.rs`; VERIFIED: `crates/media_runtime/src/probe.rs`; VERIFIED: `crates/preview_service/src/service.rs`]

The primary recommendation is to put shared contracts in `media_runtime`, put platform implementations behind desktop `cfg` boundaries, and make every decoded frame a lease with explicit color metadata and either CPU storage or a platform texture handle. Windows should use Media Foundation plus a D3D device-manager-backed decode path; macOS should use AVFoundation/VideoToolbox producing CoreVideo pixel buffers and Metal texture cache handles. [CITED: https://learn.microsoft.com/en-us/windows/win32/medfound/source-reader-attributes; CITED: https://developer.apple.com/documentation/videotoolbox/vtdecompressionsession; CITED: https://developer.apple.com/documentation/corevideo/cvmetaltexturecache]

**Primary recommendation:** Use native hardware texture decode as an optimization behind the same `DecodedVideoFrame` contract, with FFmpeg preserved as fallback/probe/export/transcode rather than replacing render graph semantics. [VERIFIED: .planning/notes/production-editor-architecture-decisions.md]

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|--------------|----------------|-----------|
| Media probing | Rust media runtime | FFmpeg fallback | Probe results are runtime facts, not UI state. Existing ffprobe probe already lives in `media_runtime`. [VERIFIED: `crates/media_runtime/src/probe.rs`] |
| Media reading | Desktop runtime | OS media frameworks | Reading container streams depends on platform/FFmpeg capabilities and must not enter semantic crates. [VERIFIED: `docs/runtime-boundaries.md`] |
| Video decode | Desktop runtime | GPU device layer | Decode selects native hardware/software/FFmpeg path and returns frame handles. [VERIFIED: .planning/REQUIREMENTS.md MEDIAIO-01] |
| Frame/texture lifetime | Rust handle registry/runtime | Binding layer release API | Native surfaces are device/session-bound and should cross JS only as opaque handles. [VERIFIED: .planning/REQUIREMENTS.md BIND-02/BIND-03] |
| Realtime composition | Phase 11 preview runtime | wgpu backend | Phase 12 supplies decoded material frames; Phase 11 owns composition and presentation. [VERIFIED: `.planning/phases/11-realtime-preview-runtime-and-gpu-render-backend/11-CONTEXT.md`] |
| Export semantics | Render graph + FFmpeg compiler/runtime | FFmpeg fallback | Export remains on the existing semantic render graph/FFmpeg compiler path. [VERIFIED: `docs/runtime-boundaries.md`] |

## Standard Stack

### Core

| Library / API | Version | Purpose | Why Standard |
|---------------|---------|---------|--------------|
| Rust workspace `media_runtime` | local | Shared media IO traits, frame metadata, handle types, capability reports | Existing project boundary for FFmpeg/ffprobe execution and runtime capabilities. [VERIFIED: codebase grep] |
| Windows Media Foundation Source Reader | OS API | Container/stream read and platform decode entry point | Source Reader exposes media-source attributes including D3D manager and DXVA-related settings. [CITED: https://learn.microsoft.com/en-us/windows/win32/medfound/source-reader-attributes] |
| Windows DXVA / Direct3D video decode | OS API | Hardware decode into D3D resources | Microsoft documents Direct3D 11 video decode support in Media Foundation and `ID3D11VideoDecoder`. [CITED: https://learn.microsoft.com/en-us/windows/win32/medfound/supporting-direct3d-11-video-decoding-in-media-foundation; CITED: https://learn.microsoft.com/en-us/windows/win32/api/d3d11/nn-d3d11-id3d11videodecoder] |
| AVFoundation / AVAssetReader | OS API | macOS media read path | Apple documents AVAssetReader and track outputs as the media sample reading path. [CITED: https://developer.apple.com/documentation/avfoundation/avassetreader; CITED: https://developer.apple.com/documentation/avfoundation/avassetreadertrackoutput] |
| VideoToolbox `VTDecompressionSession` | OS API | macOS hardware/software video decompression | Apple documents VideoToolbox decompression sessions for decoded video output. [CITED: https://developer.apple.com/documentation/videotoolbox/vtdecompressionsession] |
| CoreVideo `CVPixelBuffer` / `CVMetalTextureCache` | OS API | Pixel-buffer lifetime and Metal texture interop | Apple documents pixel buffers and Metal texture cache objects for CoreVideo-to-Metal texture creation. [CITED: https://developer.apple.com/documentation/corevideo/cvpixelbuffer; CITED: https://developer.apple.com/documentation/corevideo/cvmetaltexturecache] |
| FFmpeg / ffprobe | local binary 8.1 in current environment | Fallback decode/probe/export/transcode | Existing runtime already discovers and executes local FFmpeg/ffprobe; environment has FFmpeg 8.1. [VERIFIED: `ffmpeg -version`; VERIFIED: `crates/media_runtime/src/lib.rs`] |
| `wgpu` | 29.0.3 | Phase 11 consumer GPU layer | Cargo reports dx12 and metal features; Phase 11 locked `wgpu` as preview backend. [VERIFIED: cargo registry; VERIFIED: Phase 11 context] |

### Rust Binding Candidates

| Crate | Version | Purpose | Recommendation |
|-------|---------|---------|----------------|
| `windows` | 0.62.2 | Windows API bindings for Media Foundation and D3D | Use for Windows implementation if planner accepts crate addition; Microsoft owns the repository. [VERIFIED: cargo registry] |
| `objc2` | 0.6.4 | Objective-C runtime bindings | Candidate foundation for macOS framework calls; legitimacy not slopcheck-verified for Cargo. [ASSUMED] |
| `objc2-av-foundation` | 0.3.2 | AVFoundation bindings | Candidate for `AVAssetReader`; requires planner verification. [ASSUMED] |
| `objc2-video-toolbox` | 0.3.2 | VideoToolbox bindings | Candidate for `VTDecompressionSession`; requires planner verification. [ASSUMED] |
| `objc2-core-video` | 0.3.2 | CoreVideo bindings | Candidate for `CVPixelBuffer` and `CVMetalTextureCache`; requires planner verification. [ASSUMED] |
| `objc2-core-media` | 0.3.2 | CoreMedia bindings | Candidate for sample buffer/timing metadata; requires planner verification. [ASSUMED] |
| `objc2-metal` | 0.3.2 | Metal framework bindings | Candidate for native Metal handles; requires planner verification. [ASSUMED] |

**Installation:** Do not install during research. Planner should gate new crate additions behind a human/package verification checkpoint because slopcheck in this environment checks npm names, not Cargo crates. [VERIFIED: slopcheck command output]

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| `wgpu` | crates.io | since 2019-01-24 | 24,928,968 total | github.com/gfx-rs/wgpu | N/A, npm-only slopcheck unavailable for Cargo | Existing locked Phase 11 dependency candidate; planner verify before add. [VERIFIED: crates.io API] |
| `windows` | crates.io | since 2019-01-15 | 249,297,999 total | github.com/microsoft/windows-rs | N/A, npm-only slopcheck unavailable for Cargo | Approved candidate with planner checkpoint. [VERIFIED: crates.io API] |
| `objc2` | crates.io | since 2021-11-19 | 69,861,130 total | github.com/madsmtm/objc2 | N/A, npm-only slopcheck unavailable for Cargo | Candidate, checkpoint required. [VERIFIED: crates.io API] |
| `objc2-av-foundation` | crates.io | since 2024-09-08 | 560,274 total | github.com/madsmtm/objc2 | N/A, npm-only slopcheck unavailable for Cargo | Candidate, checkpoint required. [VERIFIED: crates.io API] |
| `objc2-video-toolbox` | crates.io | since 2025-01-12 | 15,211 total | github.com/madsmtm/objc2 | N/A, npm-only slopcheck unavailable for Cargo | Candidate, checkpoint required. [VERIFIED: crates.io API] |
| `objc2-core-video` | crates.io | since 2024-12-09 | 5,088,045 total | github.com/madsmtm/objc2 | N/A, npm-only slopcheck unavailable for Cargo | Candidate, checkpoint required. [VERIFIED: crates.io API] |
| `objc2-core-media` | crates.io | since 2024-12-15 | 558,926 total | github.com/madsmtm/objc2 | N/A, npm-only slopcheck unavailable for Cargo | Candidate, checkpoint required. [VERIFIED: crates.io API] |
| `objc2-metal` | crates.io | since 2024-04-17 | 20,397,925 total | github.com/madsmtm/objc2 | N/A, npm-only slopcheck unavailable for Cargo | Candidate, checkpoint required. [VERIFIED: crates.io API] |

**Packages removed due to slopcheck [SLOP] verdict:** none for Cargo; npm-only slopcheck results were invalid for these Rust crates. [VERIFIED: slopcheck command output]

**Packages flagged as suspicious [SUS]:** none from a Cargo-aware slopcheck because no Cargo-aware slopcheck was available. [VERIFIED: slopcheck command output]

## Architecture Patterns

### System Architecture Diagram

```text
Material URI / material ID
  -> MediaProbeService
      -> platform probe if supported
      -> ffprobe fallback
      -> RuntimeCapabilities + MaterialProbeMetadata + fallback reasons

Render graph material intent from Phase 11
  -> MediaReader session
      -> VideoDecoder / AudioDecoder
          -> FramePool lease
              -> DecodedVideoFrame
                  -> CpuFrameHandle OR TextureHandle
                      -> RealtimePreviewRuntime compositor
                          -> wgpu/D3D12 or wgpu/Metal presentation

Unsupported path
  -> classified fallback reason
  -> native software decode OR FFmpeg CPU decode OR existing preview artifact
```

### Pattern 1: Shared Traits, Platform Implementations

**What:** Put trait/type contracts in `media_runtime`; put OS-specific code behind `cfg(windows)` and `cfg(target_os = "macos")` implementation modules or crates. [VERIFIED: `docs/runtime-boundaries.md`]

**When to use:** Use this for probe, reader, decoder, frame pool, capability, and handle contracts. [VERIFIED: .planning/REQUIREMENTS.md MEDIAIO-01]

**Example:**

```rust
pub trait VideoDecoder {
    fn decoder_name(&self) -> &'static str;
    fn decode_at(&mut self, request: VideoDecodeRequest) -> Result<DecodedVideoFrame, DecodeError>;
}

pub struct DecodedVideoFrame {
    pub frame_id: FrameHandleId,
    pub source_time_us: u64,
    pub duration_us: Option<u64>,
    pub dimensions: FrameDimensions,
    pub pixel_format: VideoPixelFormat,
    pub color: VideoColorMetadata,
    pub storage: VideoFrameStorage,
}
```

### Pattern 2: Opaque Handle Boundary

**What:** Bindings return IDs plus metadata, not raw D3D/Metal/CV pointers or large byte arrays. [VERIFIED: .planning/REQUIREMENTS.md MEDIAIO-04; VERIFIED: .planning/REQUIREMENTS.md BIND-02]

**When to use:** Use this whenever frames are intended for Phase 11 preview consumption or native lifetime management. [VERIFIED: Phase 11 context]

### Anti-Patterns to Avoid

- **JS-owned frame bytes:** Passing 4K RGBA buffers through Node for every frame creates avoidable copies and breaks MEDIAIO-04. [VERIFIED: .planning/REQUIREMENTS.md MEDIAIO-04]
- **Decoder-owned rendering semantics:** Decoders choose decode paths; they must not evaluate layers, effects, transforms, or timeline state. [VERIFIED: Phase 11 context]
- **Implicit fallback:** If a native path fails, return a structured reason and selected fallback path. [VERIFIED: .planning/REQUIREMENTS.md MEDIAIO-05]
- **Leaking platform pointers to UI:** Native resource lifetime belongs to Rust sessions and device/context ownership, not React state. [VERIFIED: .planning/REQUIREMENTS.md BIND-02]

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Windows container demux/decode | Custom MP4/MOV reader and DXVA scheduler | Media Foundation Source Reader + D3D device manager | OS stack already integrates codecs, transforms, and device-backed decode. [CITED: https://learn.microsoft.com/en-us/windows/win32/medfound/source-reader-attributes] |
| Windows hardware video decode | Custom H.264/H.265 bitstream parser | Media Foundation / Direct3D video decode APIs | Direct3D 11 video decode is documented by Microsoft. [CITED: https://learn.microsoft.com/en-us/windows/win32/api/d3d11/nn-d3d11-id3d11videodecoder] |
| macOS media reading | Custom QuickTime/MOV parser | AVFoundation `AVAssetReader` | Apple supplies sample reading through AVFoundation. [CITED: https://developer.apple.com/documentation/avfoundation/avassetreader] |
| macOS hardware decode | Custom codec implementation | VideoToolbox `VTDecompressionSession` | Apple supplies system decompression sessions. [CITED: https://developer.apple.com/documentation/videotoolbox/vtdecompressionsession] |
| CoreVideo to Metal texture bridge | Manual CPU copy into texture by default | `CVMetalTextureCache` | Apple provides a CoreVideo-to-Metal texture cache. [CITED: https://developer.apple.com/documentation/corevideo/cvmetaltexturecache] |
| Export fallback | New exporter | Existing render graph -> FFmpeg compiler/runtime | Export semantics already belong to render graph and FFmpeg compiler. [VERIFIED: `docs/runtime-boundaries.md`] |

## Common Pitfalls

### Device Mismatch Between Decode And Preview

**What goes wrong:** Hardware decode succeeds but the decoded D3D/Metal texture cannot be consumed by the preview device. [ASSUMED]

**Why it happens:** Decode devices and `wgpu` presentation/composition devices may not be the same underlying platform device or may require explicit interop/import support. [ASSUMED]

**How to avoid:** Include device identity in `TextureHandle` and require a compatibility check before advertising zero-copy texture interop. [ASSUMED]

### Hidden CPU Copies

**What goes wrong:** The code advertises a texture path but maps/copies each frame to CPU first. [ASSUMED]

**How to avoid:** Runtime telemetry should classify storage as `nativeTexture`, `cpuFrame`, or `artifactFallback`, and tests should assert no byte payload in binding responses for handle paths. [VERIFIED: .planning/REQUIREMENTS.md MEDIAIO-04]

### Color Metadata Loss

**What goes wrong:** Preview/export parity diverges because color range, matrix, transfer, or primaries are dropped during decode. [ASSUMED]

**How to avoid:** Make color metadata mandatory on `DecodedVideoFrame`, even if the value is `unknown` with source diagnostics. [VERIFIED: .planning/REQUIREMENTS.md MEDIAIO-03]

### Fallback Without Diagnostics

**What goes wrong:** Unsupported codecs silently take a slower path, making performance and parity issues hard to debug. [VERIFIED: .planning/REQUIREMENTS.md MEDIAIO-05]

**How to avoid:** Capability reports and decode responses should carry fallback reason enums. [VERIFIED: .planning/REQUIREMENTS.md MEDIAIO-02]

## State Of The Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Spawn FFmpeg for each preview frame/artifact | Realtime Rust preview consumes decoded frames/texture handles | Post Phase 10.1 roadmap | Phase 12 must provide material decode without owning composition. [VERIFIED: .planning/ROADMAP.md] |
| Raw image artifacts as preview handoff | Opaque CPU/GPU frame handles with explicit leases | Phase 12 target | Avoids unnecessary JS/Rust 4K frame copies. [VERIFIED: .planning/REQUIREMENTS.md MEDIAIO-04] |
| FFmpeg-only capability report | Platform decode + FFmpeg fallback capability report | Phase 12 target | Reports native hardware decode and texture interop separately. [VERIFIED: .planning/REQUIREMENTS.md MEDIAIO-02] |

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| Rust | Rust crates/tests | yes | rustc 1.95.0 | none |
| Cargo | Registry verification/tests | yes | cargo 1.95.0 | none |
| Node.js | Existing Electron bindings/tests | yes | v24.12.0 | none |
| npm | Existing frontend tooling | yes | 11.6.2 | pnpm if configured by project |
| FFmpeg | Fallback/probe/export/transcode | yes | 8.1 | Platform decode for preview, but export fallback still needs FFmpeg |
| ffprobe | Probe fallback | yes | 8.1 | Platform probe plus degraded metadata |
| ctx7 | Documentation lookup | no | - | Official docs and cargo registry used |
| slopcheck | Package legitimacy | partial | 0.6.1, npm-only behavior observed | Planner must checkpoint Cargo crate additions |

**Missing dependencies with no fallback:** Cargo-aware slopcheck package legitimacy verification. [VERIFIED: slopcheck command output]

**Missing dependencies with fallback:** ctx7, replaced by official docs and registry checks. [VERIFIED: shell command]

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust `cargo test` plus existing Electron/Node contract tests. [VERIFIED: package.json; VERIFIED: Cargo.toml files] |
| Config file | Cargo workspace and package scripts. [VERIFIED: package.json] |
| Quick run command | `cargo test -p media_runtime -p media_runtime_desktop` |
| Full suite command | `pnpm run test:runtime && pnpm run test:bindings && pnpm run test:contracts` |

### Phase Requirements -> Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| MEDIAIO-01 | Traits decouple media read/decode from FFmpeg process execution | unit/API | `cargo test -p media_runtime media_io` | No, Wave 0 |
| MEDIAIO-02 | Platform capability report includes native decode/texture/fallback reasons | unit/platform-gated | `cargo test -p media_runtime_desktop capabilities` | No, Wave 0 |
| MEDIAIO-03 | Frame pool, lifetime, color metadata, CPU/GPU storage contracts | unit | `cargo test -p media_runtime frame_pool` | No, Wave 0 |
| MEDIAIO-04 | Binding responses use opaque handles, not 4K byte buffers | contract | `pnpm run test:contracts` | No, Wave 0 |
| MEDIAIO-05 | Native failure falls back to FFmpeg with structured reason | unit/integration | `cargo test -p media_runtime_desktop fallback` | No, Wave 0 |

### Wave 0 Gaps

- [ ] Add media IO trait/type unit tests in `crates/media_runtime`.
- [ ] Add fake frame pool leak/release tests.
- [ ] Add platform capability report tests with `cfg(windows)` and `cfg(target_os = "macos")`.
- [ ] Add contract guard that no large frame byte payload appears in handle-based binding responses.

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | no | Desktop local runtime phase; no auth surface. [ASSUMED] |
| V3 Session Management | yes | Runtime session IDs, generations, retain/release, cascading close. [VERIFIED: .planning/REQUIREMENTS.md BIND-02] |
| V4 Access Control | limited | Bindings must only access handles owned by the active runtime session. [ASSUMED] |
| V5 Input Validation | yes | Validate media paths, stream metadata, codec/pixel-format enums, dimensions, and handle IDs. [ASSUMED] |
| V6 Cryptography | no | No crypto implementation in scope. [ASSUMED] |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Stale or forged handle ID | Elevation of privilege / tampering | Session-owned opaque IDs with generation checks. [VERIFIED: .planning/REQUIREMENTS.md BIND-02] |
| Malformed media file triggers crash | Denial of service | Run decode through OS/FFmpeg APIs with classified errors, cancellation, and test fixtures. [ASSUMED] |
| Path traversal through media URI | Tampering / information disclosure | Reuse project store path normalization and checked material URI policy. [VERIFIED: `docs/runtime-boundaries.md`] |
| Unbounded native frame pool | Denial of service | Fixed budgets, backpressure, explicit release, leak diagnostics. [ASSUMED] |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Platform code starts as `cfg` modules under `media_runtime_desktop` while keeping `media_runtime` traits stable and preserving a future split seam. | User Constraints | If module size exceeds context budget, split planning should create later platform crate extraction plans without changing the trait surface. |
| A2 | Initial hardware decode acceptance targets H.264 MP4/MOV; HEVC, ProRes, AV1, and other codecs remain capability-reported/degraded unless explicitly proven later. | User Constraints | Codec status must not be overstated in capability reports or acceptance criteria. |
| A3 | `objc2-*` crates are acceptable macOS binding candidates. | Standard Stack | Planner may choose generated bindings, `core-*` crates, or handwritten FFI instead. |
| A4 | Device mismatch is a primary interop risk. | Common Pitfalls | Tests may miss the actual failing interop mode on one platform. |
| A5 | Color metadata must include unknown-with-diagnostic states. | Common Pitfalls | Schema may need refinement after real media fixtures. |

## Resolved Questions

1. **[RESOLVED] How will Phase 11 expose its `wgpu` device identity to Phase 12?**
   - What we know: Phase 11 uses `wgpu` targeting D3D12 and Metal. [VERIFIED: Phase 11 context]
   - Resolution: Phase 11 exposes device identity through `RuntimeDeviceId`, backend, and capability compatibility fields. Phase 12 `TextureHandle` values must include device identity and compatibility state. CPU fallback remains required until native import is proven for the active backend/device pair. [RESOLVED]

2. **[RESOLVED] Which codecs are Phase 12's first hardware-decode acceptance targets?**
   - What we know: Requirements mention unsupported codecs degrade predictably. [VERIFIED: MEDIAIO-05]
   - Resolution: Plan H.264 MP4/MOV on Windows and macOS as the first hardware-decode acceptance target. HEVC, ProRes, AV1, and other codecs are capability-reported and degraded unless explicitly proven by later implementation and tests. [RESOLVED]

3. **[RESOLVED] Should platform implementations be separate crates immediately?**
   - What we know: Architecture shape names `media_runtime_windows` / `media_runtime_macos` / `media_runtime_ffmpeg`. [VERIFIED: production architecture note]
   - Resolution: Start with `cfg` modules under `media_runtime_desktop`, keep the trait surface in `media_runtime`, and preserve clean module boundaries so later `media_runtime_windows`, `media_runtime_macos`, and `media_runtime_ffmpeg` crates can be extracted without changing callers. [RESOLVED]

## Sources

### Primary

- Microsoft Source Reader Attributes - https://learn.microsoft.com/en-us/windows/win32/medfound/source-reader-attributes
- Microsoft Direct3D 11 video decoding in Media Foundation - https://learn.microsoft.com/en-us/windows/win32/medfound/supporting-direct3d-11-video-decoding-in-media-foundation
- Microsoft `ID3D11VideoDecoder` - https://learn.microsoft.com/en-us/windows/win32/api/d3d11/nn-d3d11-id3d11videodecoder
- Microsoft `IMFDXGIDeviceManager` - https://learn.microsoft.com/en-us/windows/win32/api/mfobjects/nn-mfobjects-imfdxgidevicemanager
- Apple `AVAssetReader` - https://developer.apple.com/documentation/avfoundation/avassetreader
- Apple `AVAssetReaderTrackOutput` - https://developer.apple.com/documentation/avfoundation/avassetreadertrackoutput
- Apple `VTDecompressionSession` - https://developer.apple.com/documentation/videotoolbox/vtdecompressionsession
- Apple `CVPixelBuffer` - https://developer.apple.com/documentation/corevideo/cvpixelbuffer
- Apple `CVMetalTextureCache` - https://developer.apple.com/documentation/corevideo/cvmetaltexturecache
- Apple `CVMetalTexture` - https://developer.apple.com/documentation/corevideo/cvmetaltexture
- wgpu docs.rs 29.0.3 - https://docs.rs/wgpu/29.0.3/wgpu/
- FFmpeg documentation - https://ffmpeg.org/ffmpeg.html
- ffprobe documentation - https://ffmpeg.org/ffprobe.html

### Codebase

- `AGENTS.md`
- `.planning/PROJECT.md`
- `.planning/ROADMAP.md`
- `.planning/REQUIREMENTS.md`
- `.planning/notes/production-editor-architecture-decisions.md`
- `docs/runtime-boundaries.md`
- `crates/media_runtime/src/lib.rs`
- `crates/media_runtime/src/probe.rs`
- `crates/media_runtime/src/capabilities.rs`
- `crates/media_runtime_desktop/src/lib.rs`
- `crates/preview_service/src/service.rs`
- `.planning/phases/11-realtime-preview-runtime-and-gpu-render-backend/11-CONTEXT.md`

## Metadata

**Confidence breakdown:**
- Architecture: HIGH - grounded in project constraints and Phase 11/12 roadmap.
- Windows native path: MEDIUM-HIGH - official Microsoft API docs confirm the building blocks; exact `wgpu` interop depends on Phase 11 device implementation.
- macOS native path: MEDIUM-HIGH - official Apple API docs confirm the building blocks; Rust crate binding choice remains assumed.
- Package choices: MEDIUM - Cargo registry metadata verified; Cargo-aware slopcheck unavailable.

**Research date:** 2026-06-18
**Valid until:** 2026-07-18
