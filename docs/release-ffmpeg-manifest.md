# Release FFmpeg Manifest

This document records the desktop runtime packaging posture for FFmpeg and ffprobe.

## Runtime Posture

FFmpeg and ffprobe are bundled application resources for the desktop package.

Runtime discovery is intentionally single-source:

- Electron configures the native binding with the app-local bundled runtime directory.
- `apps/desktop-electron/runtime/ffmpeg/<platform>-<arch>` during local development
- `process.resourcesPath/ffmpeg/<platform>-<arch>` in packaged Electron builds

The app runtime must not discover FFmpeg or ffprobe through `PATH` or separate
per-binary environment variables, and product startup must not honor external
runtime directory overrides.

## Bundled Runtime Layout

The build script `apps/desktop-electron/scripts/provision-ffmpeg-runtime.mjs`
validates the already-bundled FFmpeg and ffprobe binaries in:

```text
apps/desktop-electron/runtime/ffmpeg/<platform>-<arch>/
  ffmpeg
  ffprobe
  manifest.local.json
```

`electron-builder.yml` packages that directory as:

```text
resources/ffmpeg/<platform>-<arch>/
```

The generated `manifest.local.json` records the binary file names, `-version`
first lines, configure lines, SHA-256 checksums, and review status.

## Licensing Status

The current engineering status is `legalReviewPending`.

The runtime capability report must expose:

- `source: "bundled"` for both binaries
- `licensePosture.source: "bundledRuntime"`
- `licensePosture.externalRuntime: false`
- `licensePosture.redistributableBuild: false`

This records that the packaged app has a deterministic bundled runtime. It does
not claim public redistribution approval. A public redistributable build still
requires legal review of the exact FFmpeg build, enabled libraries, notices,
source-offer/relinking obligations where applicable, and release approvals.
