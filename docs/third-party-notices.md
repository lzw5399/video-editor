# Third-Party Notices

This notice file summarizes the Phase 6 MVP dependency posture. It is not a
complete legal review for a future bundled-media-runtime release.

## Video Editor

Video Editor is licensed under the MIT License. See `LICENSE`.

## Electron Desktop Dependencies

The desktop app is built with Electron, React, React DOM, Vite, Playwright, and
related JavaScript tooling declared in the repository package manifests. Release
operators should regenerate dependency notices from the locked package graph
before any public binary release.

## Rust Dependencies

The editing, render, media runtime, project store, and Node-API binding layers
are Rust workspace crates. Release operators should generate Rust dependency
license notices from `Cargo.lock` before any public binary release.

## FFmpeg And ffprobe

FFmpeg is external/user-provided for the MVP.

No FFmpeg binary is bundled by Phase 6. Users or CI provide FFmpeg and ffprobe
through `VE_FFMPEG_PATH`, `VE_FFPROBE_PATH`, or `PATH`.

Because Phase 6 does not redistribute FFmpeg binaries, this notice does not
claim a redistributable FFmpeg build, configure line, source offer, or LGPL/GPL
compliance package. Any later bundled FFmpeg release must add a concrete build
manifest and corresponding notices before shipping.

## External References

Kdenlive, MLT, Jianying, CapCut, Kaipai, and pyJianYingDraft are references for
concepts, terminology, and compatibility research only. This project does not
copy their code, assets, XML definitions, presets, or UI implementation.
