---
phase: 12-media-io-hardware-decode-and-frame-texture-interop
plan: 02B
subsystem: media-runtime-bindings
tags: [media-io, bindings, schema, source-guards, cargo-dependencies]

requires:
  - phase: 12-01
    provides: shared media IO, frame pool, frame/texture handle, and fallback contracts
  - phase: 12-02
    provides: desktop runtime capability aggregation for FFmpeg and native media IO domains
provides:
  - Binding-safe media IO capability contracts generated from draft_model
  - Runtime capability binding mapping for mediaIo native platform domains
  - Phase 12 source guard for renderer/native-pointer/raw-frame-payload boundaries
  - Approved Windows/macOS platform Cargo dependency declarations
affects: [phase-12, media-runtime-desktop, bindings-node, generated-contracts]

tech-stack:
  added:
    - windows 0.62.2
    - objc2 0.6.4
    - objc2-av-foundation 0.3.2
    - objc2-video-toolbox 0.3.2
    - objc2-core-video 0.3.2
    - objc2-core-media 0.3.2
    - objc2-metal 0.3.2
  patterns:
    - Binding-facing media IO reports are draft_model-generated contracts.
    - Native dependencies are target-specific and approved before platform plans rely on them.

key-files:
  created:
    - scripts/phase12-source-guards.sh
  modified:
    - Cargo.lock
    - crates/media_runtime_desktop/Cargo.toml
    - crates/draft_model/src/lib.rs
    - crates/draft_model/tests/schema_exports.rs
    - crates/bindings_node/src/runtime_capability_service.rs
    - crates/bindings_node/tests/runtime_capabilities.rs
    - schemas/command.schema.json
    - apps/desktop-electron/src/generated/CommandResultEnvelope.ts
    - package.json

key-decisions:
  - "RuntimeCapabilityReport keeps existing FFmpeg readiness fields and adds generated mediaIo contracts for native platform capability data."
  - "Decoded frame and texture binding contracts expose owner session, generation, backend, device, dimensions, and pixel format metadata, not native pointers or raw frame bytes."
  - "Windows/macOS native platform crates are target-specific dependencies approved by the user before 12-04 and 12-05 rely on them."

patterns-established:
  - "Generated media IO contracts: draft_model owns schema and TypeScript exports for binding-visible runtime capability and handle metadata."
  - "Source guard boundary: renderer code cannot own platform media APIs, FFmpeg fallback selection, native pointers, or full-frame byte payload contracts."

requirements-completed: [MEDIAIO-02, MEDIAIO-04, MEDIAIO-05]

duration: 29 min
completed: 2026-06-18
---

# Phase 12 Plan 02B: Binding/Schema And Dependency Checkpoint Summary

**Generated media IO capability and handle metadata contracts with approved target-specific native platform dependencies.**

## Performance

- **Duration:** 29 min
- **Started:** 2026-06-18T19:01:00Z
- **Completed:** 2026-06-18T19:30:33Z
- **Tasks:** 2
- **Files modified:** 10

## Accomplishments

- Added failing and passing binding/schema tests proving `probeRuntimeCapabilities` now returns `mediaIo` with Windows/macOS domains, codec status, texture interop status, and fallback ladder data.
- Added generated `draft_model` contracts for media IO capabilities plus decoded frame and texture handle metadata without native pointers or raw full-frame payloads.
- Added `scripts/phase12-source-guards.sh` and `test:phase12-source-guards` to block renderer ownership of platform media APIs, FFmpeg fallback selection, native pointers, and raw byte/pixel payload contracts.
- Completed the blocking dependency legitimacy checkpoint after user approval and added target-specific dependencies for Windows Media Foundation/D3D and macOS AVFoundation/VideoToolbox/CoreVideo/Metal plans.

## Task Commits

1. **Task 12-02B-01 RED: Binding capability and guard tests** - `424777c` (test)
2. **Task 12-02B-01 GREEN: Binding-safe media IO contracts and source guard** - `add4f1e` (feat)
3. **Task 12-02B-02: Approved platform media dependencies** - `90c2fce` (chore)

## Files Created/Modified

- `scripts/phase12-source-guards.sh` - Phase 12 renderer/binding boundary guard.
- `crates/draft_model/src/lib.rs` - Binding-safe media IO capability and handle metadata contracts.
- `crates/bindings_node/src/runtime_capability_service.rs` - Mapping from desktop runtime capability aggregate to generated command result contracts.
- `schemas/command.schema.json` and `apps/desktop-electron/src/generated/CommandResultEnvelope.ts` - Generated schema/TypeScript contract updates.
- `crates/media_runtime_desktop/Cargo.toml` and `Cargo.lock` - Approved target-specific Windows/macOS platform dependencies.
- `package.json` - `test:phase12-source-guards` script.

## Package Approval Record

User approval: `approved`

Approved dependency set:

| Target | Crate | Version | Registry/source |
|--------|-------|---------|-----------------|
| Windows | `windows` | 0.62.2 | crates.io, `github.com/microsoft/windows-rs` |
| macOS | `objc2` | 0.6.4 | crates.io, `github.com/madsmtm/objc2` |
| macOS | `objc2-av-foundation` | 0.3.2 | crates.io, `github.com/madsmtm/objc2` |
| macOS | `objc2-video-toolbox` | 0.3.2 | crates.io, `github.com/madsmtm/objc2` |
| macOS | `objc2-core-video` | 0.3.2 | crates.io, `github.com/madsmtm/objc2` |
| macOS | `objc2-core-media` | 0.3.2 | crates.io, `github.com/madsmtm/objc2` |
| macOS | `objc2-metal` | 0.3.2 | crates.io, `github.com/madsmtm/objc2` |

Rejected crates: None.

## Decisions Made

- `RuntimeCapabilityReport` remains backward-compatible for current UI/runtime diagnostics by preserving existing FFmpeg fields while adding the generated `mediaIo` capability object.
- Handle-capable contracts use metadata-only `RuntimeDecodedFrameHandleMetadata` and `RuntimeTextureHandleMetadata`; they intentionally omit native pointers, `ArrayBuffer`, `Uint8Array`, bytes, pixels, and color-channel payloads.
- Platform dependencies are declared under target-specific Cargo sections so future 12-04/12-05 native implementations can use approved crates without editing shared dependency declarations.

## Deviations from Plan

None - plan executed exactly as written.

## Issues Encountered

- Direct crates.io API `curl` returned 403 during package re-check, so package legitimacy was verified with `cargo info` plus crates.io/GitHub source pages before presenting the approval checkpoint.
- `rustfmt` reordered imports in unrelated draft_model files during local formatting; those unrelated diffs were reverted before commit.

## Verification

- `cargo test -p bindings_node runtime_capabilities -- --nocapture`
- `cargo test -p draft_model schema_exports -- --nocapture`
- `pnpm run test:phase12-source-guards`
- `git diff --exit-code schemas`
- `cargo metadata --locked --format-version 1 >/tmp/video-editor-phase12-cargo-metadata.json`
- `cargo check --workspace --locked`
- `git diff --check`

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Ready for `12-03`: FFmpeg CPU frame fallback decoder and structured fallback ladder. The binding/schema boundary and platform dependency approvals required by later 12-04/12-05 native implementations are in place.

---
*Phase: 12-media-io-hardware-decode-and-frame-texture-interop*
*Completed: 2026-06-18*
