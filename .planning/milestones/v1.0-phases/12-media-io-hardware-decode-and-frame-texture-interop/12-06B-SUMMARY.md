---
phase: 12-media-io-hardware-decode-and-frame-texture-interop
plan: 06B
subsystem: bindings-node
tags: [media-io, preview-decode, opaque-handles, release-api, generated-contracts]

requires:
  - phase: 12-06
    provides: realtime preview media IO handoff adapter and fallback diagnostics
provides:
  - handle-based preview decode command contract
  - preview frame release command contract with owner session and generation validation
  - binding-visible decode path, storage kind, texture compatibility, fallback diagnostics, and color metadata
  - generated command schema and TypeScript contracts for preview decode/release
  - schema/source guards against native pointer and full-frame JS payload exposure
affects: [phase-12, bindings-node, draft-model, desktop-generated-contracts]

key-files:
  created:
    - .planning/phases/12-media-io-hardware-decode-and-frame-texture-interop/12-06B-SUMMARY.md
  modified:
    - crates/draft_model/src/lib.rs
    - crates/draft_model/tests/schema_exports.rs
    - crates/bindings_node/src/lib.rs
    - crates/bindings_node/src/preview_export_service.rs
    - crates/bindings_node/tests/preview_commands.rs
    - schemas/command.schema.json
    - apps/desktop-electron/src/generated/CommandEnvelope.ts
    - apps/desktop-electron/src/generated/CommandResultEnvelope.ts

key-decisions:
  - "Preview decode binding responses expose opaque frame/texture handle IDs and metadata only; native pointers, platform objects, ArrayBuffer/Uint8Array values, and pixel payloads do not cross to JS."
  - "Frame release validates frame handle ID, owner session, and playback generation before removing a retained handle."
  - "Generated schema command/payload pairing constraints now include preview decode/release and existing keyframe command pairs."
  - "12-06B establishes binding contracts and a Rust-side handle registry; desktop playback controls and continuous presentation remain later UI/preview wiring work."

patterns-established:
  - "Binding handle registry pattern: retain metadata by opaque frame handle ID, reject wrong-session and stale-generation releases, then remove on successful release."
  - "Preview decode diagnostics include selected path, fallback reason, storage kind, preview/native device metadata, texture compatibility, and bounded color metadata."

requirements-completed: [MEDIAIO-02, MEDIAIO-03, MEDIAIO-04, MEDIAIO-05]

duration: 20 min
completed: 2026-06-19
---

# Phase 12 Plan 06B: Preview Decode Handle Contracts Summary

**Electron can now request preview decode metadata through Rust-owned command contracts and release retained frame handles without receiving native pointers or full-frame pixel payloads.**

## Performance

- **Duration:** 20 min
- **Completed:** 2026-06-19
- **Tasks:** 1
- **Files modified:** 8 code/generated files plus this summary

## Accomplishments

- Added `requestPreviewDecode` and `releasePreviewFrame` to `CommandName`, `CommandPayload`, generated schema, and generated TypeScript contracts.
- Added `PreviewDecodeRequest`, `ReleasePreviewFrameCommandPayload`, `DecodedPreviewFrameResponse`, `PreviewFrameReleaseResponse`, `PreviewFrameStoragePreference`, `PreviewFrameStorageKind`, and `PreviewDecodeDiagnostic`.
- Added binding-safe runtime color metadata contracts and attached color metadata to decoded frame and texture handle metadata.
- Added a Rust-side `PreviewFrameHandleRegistry` in `bindings_node` that creates opaque frame/texture handle metadata and validates release by session and playback generation.
- Preserved existing `requestPreviewFrame` artifact command behavior as fallback/compatibility path.
- Tightened schema export assertions and generated schema command/payload pairing constraints for preview decode/release and existing keyframe command pairs.

## Task Commits

1. **Task 12-06B-01 RED: preview decode binding tests** - `bcfda18` (test)
2. **Task 12-06B-01 GREEN: preview decode handle contracts** - `34c70ff` (feat)

## Files Created/Modified

- `crates/draft_model/src/lib.rs` - command/response contracts, storage enums, preview diagnostics, color metadata, and handle metadata additions.
- `crates/bindings_node/src/lib.rs` - command allow-list and dispatch for preview decode/release.
- `crates/bindings_node/src/preview_export_service.rs` - handle registry, decode metadata response builder, session/generation-checked release.
- `crates/bindings_node/tests/preview_commands.rs` - handle response, no full-frame/native payload, wrong-session/stale/unknown release tests.
- `crates/draft_model/tests/schema_exports.rs` - schema/TS export coverage and forbidden payload guard strings.
- `schemas/command.schema.json` and `apps/desktop-electron/src/generated/*.ts` - generated contract updates.

## Decisions Made

- Binding decode currently returns safe handle metadata and diagnostics from the Rust registry boundary. It does not claim that a platform texture object is already mapped into Electron's renderer.
- Handle IDs are opaque binding-owned identifiers; tests assert ownership/generation semantics instead of relying on a fixed ID.
- Color metadata is explicit but currently unknown/diagnostic at this binding layer until the concrete media IO decoder supplies source color attachments through the full handoff path.

## Deviations from Plan

### Contract Guard Tightening

**1. Added keyframe command/payload schema pairing while touching the same hardcoded pairing table**

- **Found during:** generated schema review
- **Issue:** The hardcoded JSON Schema `oneOf` pairing table already missed `setSegmentKeyframe` and `removeSegmentKeyframe`.
- **Fix:** Added those existing command pairs along with `requestPreviewDecode` and `releasePreviewFrame`.
- **Files modified:** `crates/draft_model/tests/schema_exports.rs`, `schemas/command.schema.json`
- **Verification:** `cargo test -p draft_model schema_exports -- --nocapture`
- **Committed in:** `34c70ff`

---

**Total deviations:** 1 guard tightening.
**Impact on plan:** Positive. The generated schema now rejects mismatched command/payload pairs for the new commands and closes an adjacent existing schema gap.

## Issues Encountered

- `cargo fmt --check --package draft_model --package bindings_node` would also report existing unrelated formatting differences in binding runtime capability files. To avoid unrelated churn, verification used `rustfmt --edition 2024 --check --config skip_children=true` on this plan's touched Rust files.

## Verification

- `cargo test -p bindings_node preview_decode -- --nocapture`
- `cargo test -p draft_model schema_exports -- --nocapture`
- `cargo check --workspace --locked`
- `git diff --exit-code schemas apps/desktop-electron/src/generated`
- `rustfmt --edition 2024 --check --config skip_children=true crates/draft_model/src/lib.rs crates/draft_model/tests/schema_exports.rs crates/bindings_node/src/lib.rs crates/bindings_node/src/preview_export_service.rs crates/bindings_node/tests/preview_commands.rs`
- `rg -n "nativePointer|rawHandle|ArrayBuffer|Uint8Array|\\bbytes\\b|\\bpixels\\b" schemas/command.schema.json apps/desktop-electron/src/generated/CommandEnvelope.ts apps/desktop-electron/src/generated/CommandResultEnvelope.ts` returned no matches.
- `git diff --check`

## User Setup Required

None.

## Next Phase Readiness

Ready for `12-06C`: the next plan can connect these binding-safe handles to concrete native preview decode/release internals and eventually the desktop playback presentation loop. This plan does not itself implement the grey play button or continuous preview playback.

---
*Phase: 12-media-io-hardware-decode-and-frame-texture-interop*
*Completed: 2026-06-19*
