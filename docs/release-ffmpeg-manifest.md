# Release FFmpeg Manifest

This document records the Phase 6 MVP runtime posture for FFmpeg and ffprobe.

## Phase 6 Posture

FFmpeg is external/user-provided for the MVP.

Runtime discovery uses the existing Rust-owned media runtime boundary:

- `VE_FFMPEG_PATH`
- `VE_FFPROBE_PATH`
- `PATH`

No FFmpeg binary is bundled by Phase 6.

Homebrew --enable-gpl is development/test only.

The local development machine may use Homebrew, system packages, or another
user-installed FFmpeg build to satisfy no-mock preview/export tests. That local
runtime is not a Video Editor redistributable binary and is not evidence that
the project can ship the same build.

## What Phase 6 Ships

- Electron desktop directory package support.
- Rust-owned FFmpeg/ffprobe discovery and capability diagnostics.
- Dev and packaged no-mock workflow gates that require an external runtime.
- Documentation and guards that prevent accidental bundled-runtime claims.

## What Phase 6 Does Not Ship

- Downloaded FFmpeg binaries.
- Bundled FFmpeg or ffprobe resources.
- A selected redistributable FFmpeg build.
- LGPL/GPL/nonfree redistribution review for a project-shipped FFmpeg binary.
- Source-offer, object-file, or build-script obligations for a shipped FFmpeg
  binary.
- Packaged resource resolver tests for bundled FFmpeg.

## Future Bundled FFmpeg Checklist

Any later plan that bundles FFmpeg must add all of these artifacts together:

1. Exact binary source and version.
2. Full configure line and enabled external libraries.
3. LGPL/GPL/nonfree review based on that exact build.
4. Source-offer and relinking obligation review where applicable.
5. Third-party notices for FFmpeg and enabled libraries.
6. Packaged resource resolver implementation and tests.
7. Runtime capability tests that prove packaged-resource discovery works.
8. Release notes that distinguish bundled, system, and user-configured runtimes.

Until that work exists, Video Editor releases must continue to describe FFmpeg
as an external/user-provided runtime for the MVP.
