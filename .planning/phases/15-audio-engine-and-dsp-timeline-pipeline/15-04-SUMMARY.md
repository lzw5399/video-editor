---
phase: 15-audio-engine-and-dsp-timeline-pipeline
plan: "04"
subsystem: audio-bindings
tags:
  - rust
  - bindings-node
  - generated-contracts
  - audio-preview
  - waveform
dependency_graph:
  requires:
    - 15-02 audio_engine session runtime
    - 15-03 audio_output_desktop safe capability summaries
  provides:
    - Generated audio preview/device/waveform command contracts
    - Opaque AudioPreviewBindingRegistry
    - Bounded waveform display payload transport
  affects:
    - draft_model
    - bindings_node
    - desktop-electron generated contracts
    - audio_engine
tech_stack:
  added:
    - bindings_node path dependency on audio_engine
    - bindings_node path dependency on audio_output_desktop
  patterns:
    - Rust-owned DTOs generated to TypeScript
    - Opaque binding session registry
    - Bounded renderer-safe display payloads
key_files:
  created:
    - crates/bindings_node/src/audio_service.rs
    - crates/bindings_node/tests/audio_service.rs
  modified:
    - Cargo.lock
    - crates/audio_engine/src/session.rs
    - crates/bindings_node/Cargo.toml
    - crates/bindings_node/src/lib.rs
    - crates/draft_model/src/lib.rs
    - crates/draft_model/tests/schema_exports.rs
    - schemas/command.schema.json
    - apps/desktop-electron/src/generated/CommandEnvelope.ts
    - apps/desktop-electron/src/generated/CommandResultEnvelope.ts
decisions:
  - Audio preview command transport shares AudioPreviewCommandPayload; handlers validate command-specific required fields.
  - Bindings expose audio-session-* IDs and keep audio_engine session IDs private.
  - Waveform display responses are bounded display peaks, not canonical draft state or artifact-store internals.
  - Stop commands route to AudioPreviewRuntime::stop so binding responses reflect Rust clock semantics.
metrics:
  started_at: 2026-06-19T10:46:51Z
  completed_at: 2026-06-19T11:03:06Z
  duration_seconds: 975
  tasks_completed: 2
  files_changed: 11
requirements:
  completed:
    - AUDIO2-01
    - AUDIO2-03
---

# Phase 15 Plan 04: Audio Binding Contracts Summary

Generated audio preview command contracts and implemented an opaque Node binding service for preview sessions, output device summaries, and bounded waveform display payloads.

## What Changed

Task 15-04-01 added generated Rust-owned DTOs for audio preview command transport. The command contract now includes create/play/pause/stop/seek/cancel/status commands, output device list/select commands, and waveform display status/peak commands. The schema export tests verify the commands, command/payload pairings, generated TypeScript artifacts, and forbidden internal field names for the new audio DTO surface.

Task 15-04-02 added `AudioPreviewBindingRegistry` and routed the new commands through `bindings_node::execute_command`. The registry validates `audio-session-*` IDs, keeps runtime session IDs private, generation-gates stale playback requests, maps device capability probes to safe display summaries, and returns bounded waveform display peak arrays without artifact paths or cache internals.

## Task Commits

| Task | Commit | Message |
|------|--------|---------|
| 15-04-01 RED | 0543f40 | test(15-04): add failing audio binding contract tests |
| 15-04-01 GREEN | 60c5299 | feat(15-04): export audio preview binding contracts |
| 15-04-02 RED | b4a243a | test(15-04): add failing audio binding service tests |
| 15-04-02 GREEN | 3688ee6 | feat(15-04): implement opaque audio binding service |

## Verification

| Command | Result |
|---------|--------|
| `cargo test -p draft_model schema_exports -- --nocapture` | Passed: 19 schema export tests |
| `cargo test -p bindings_node audio_service -- --nocapture` | Passed: 3 audio service tests |
| `cargo test -p bindings_node -- --nocapture` | Passed: full bindings_node test package |
| `git diff --exit-code schemas apps/desktop-electron/src/generated` | Passed: generated artifacts have no drift |
| `rg -n "audio-session-|MalformedSessionId|WaveformDisplayPeaks" crates/bindings_node/src crates/bindings_node/tests` | Passed: implementation and tests contain required anchors |
| `rg -n "native.*handle|raw.*buffer|artifactRoot|SQLite|blobPath|fingerprint|dirtyRange|cacheKey|ffmpegFilter|filter_complex" crates/bindings_node/src/audio_service.rs` | Passed: no forbidden source exposure |

## Requirements Covered

- `AUDIO2-01`: Electron command transport can create and control audio preview sessions while Rust owns sessions, generation checks, and playback state.
- `AUDIO2-03`: Output device and waveform payloads cross the binding as safe summaries and bounded display peaks only.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Narrowed forbidden-field contract test scope**
- **Found during:** Task 15-04-01 GREEN
- **Issue:** The RED test initially scanned all of `CommandResultEnvelope.ts`, which already contains unrelated export and preview dirty-fact fields from earlier phases.
- **Fix:** Scoped the forbidden-field assertion to the new audio DTO schema/generated text instead of unrelated command surfaces.
- **Files modified:** `crates/draft_model/tests/schema_exports.rs`
- **Commit:** 60c5299

**2. [Rule 3 - Blocking Issue] Raised schema export macro recursion limit**
- **Found during:** Task 15-04-01 GREEN
- **Issue:** The expanded audio command/payload pairing assertions exceeded the default `json!` macro recursion limit.
- **Fix:** Added a local recursion limit to the schema export test module.
- **Files modified:** `crates/draft_model/tests/schema_exports.rs`
- **Commit:** 60c5299

**3. [Rule 2 - Missing Critical Functionality] Added Rust-owned stop semantics**
- **Found during:** Task 15-04-02 GREEN
- **Issue:** The binding needed to route stop commands through Rust session state instead of fabricating a stopped response at the binding layer.
- **Fix:** Added `AudioPreviewRuntime::stop`, delegating to `TimelineClock::stop` and synchronizing generation state.
- **Files modified:** `crates/audio_engine/src/session.rs`
- **Commit:** 3688ee6

## Threat Model

| Threat | Mitigation |
|--------|------------|
| Spoofed/tampered audio session IDs | `audio-session-` prefix, fixed 16-hex suffix validation, private runtime session map, classified malformed/unknown errors |
| Device or waveform information disclosure | Safe labels/status/diagnostics only; no native handles, raw buffers, artifact roots, SQLite paths, fingerprints, dirty ranges, cache keys, or FFmpeg filters in audio service output |
| Oversized waveform display payloads | Peak bin requests are capped at 512 bins in Rust before serialization |
| Ambiguous command outcomes | Command responses classify accepted, stale rejected, canceled, stopped, missing, degraded, and unavailable states |

## Known Stubs

None. The waveform display helper intentionally returns bounded display-ready peaks for the binding contract boundary; canonical waveform artifact extraction remains outside this plan's scope.

## Auth Gates

None.

## Deferred Issues

None.

## Self-Check: PASSED

- Verified all created/modified files exist.
- Verified task commits exist: `0543f40`, `60c5299`, `b4a243a`, `3688ee6`.
- Verified worktree status contains only ignored-by-instruction untracked `reference/` after task commits.
