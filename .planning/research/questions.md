# Research Questions

## Production Desktop Editor Architecture

- How should a Rust `wgpu` preview surface be embedded into the Electron desktop workspace on Windows and macOS: native child window, platform view handle, shared texture, or another composition strategy?
- What are the exact constraints for `wgpu` D3D12 and Metal surfaces when the host window is owned by Electron, including resize, DPI scaling, focus, z-order, and fullscreen behavior?
- What is the lowest-copy path from Windows Media Foundation / DXVA decoded frames into a `wgpu` texture, and which D3D11/D3D12 interop layer is required?
- What is the lowest-copy path from macOS VideoToolbox / CoreVideo decoded frames into a `wgpu` Metal texture, and how should CVPixelBuffer lifetimes be represented?
- What fallback ladder should desktop preview use when native hardware decode fails: platform software decode, FFmpeg decode to CPU frame, or FFmpeg-generated preview artifact?
- What should the initial `TimelineClock` drift budget be for WASAPI/CoreAudio audio master sync with `wgpu` video rendering?
- How should `PlaybackGeneration` be represented so stale preview frames, audio buffers, waveform jobs, and cache jobs cannot overwrite current state after seek or edit?
- What SQLite schema and WAL/concurrency settings should the project-local artifact store use for thumbnails, waveforms, preview frames, proxies, render graph snapshots, and generation status?
- What stable render graph node identity scheme best covers segment, text overlay, effect, transition, audio mix, material, and canvas nodes without relying on content hashes as identity?
- How should the cross-language handle registry expose retain/release semantics through Node-API first, then C ABI, JNI, Swift/ObjC, and server runtime entrypoints?
- Which FFmpeg export/filter outputs must be parity-tested against GPU preview before production effects, retiming, and transitions are considered safe?
- What FFmpeg distribution, hardware acceleration, LGPL/GPL/nonfree, and third-party notice posture is required once desktop packaging ships production export/runtime binaries?
