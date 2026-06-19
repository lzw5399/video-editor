---
phase: 15-audio-engine-and-dsp-timeline-pipeline
reviewed: 2026-06-19T12:23:31Z
depth: standard
files_reviewed: 13
files_reviewed_list:
  - apps/desktop-electron/src/renderer/App.tsx
  - crates/audio_output_desktop/src/cpal_output.rs
  - crates/bindings_node/src/audio_service.rs
  - crates/bindings_node/tests/audio_service.rs
  - crates/engine_core/src/normalize.rs
  - crates/ffmpeg_compiler/src/filters.rs
  - crates/ffmpeg_compiler/tests/common/mod.rs
  - crates/ffmpeg_compiler/tests/ffmpeg_job_snapshots.rs
  - crates/render_graph/src/graph.rs
  - crates/render_graph/tests/render_graph_snapshots.rs
  - crates/testkit/src/audio_parity.rs
  - package.json
  - scripts/phase15-source-guards.sh
findings:
  critical: 2
  warning: 1
  info: 0
  total: 3
status: issues_found
---

# Phase 15: Code Review Report

**Reviewed:** 2026-06-19T12:23:31Z
**Depth:** standard
**Files Reviewed:** 13
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

Reviewed HEAD `4ecc0cb` after `cd3dc84` recorded the prior review and `4ecc0cb` attempted to close the seven Phase 15 findings. `reference/` was left untouched.

Prior finding status: CR-01, CR-02, CR-03, CR-05, and WR-02 are fixed in the current worktree. WR-01 is only partially fixed. CR-04 is not fixed to production grade: keyframed audio still compiles to a successful export job without FFmpeg volume automation. One additional export blocker was found in the same audio compiler path.

## Critical Issues

### CR-01: Silent audio ranges compile an invalid `amix=inputs=0` graph [BLOCKER]

**File:** `crates/ffmpeg_compiler/src/filters.rs:142`

**Issue:** `has_audio_output` is set from `!plan.graph.audio_mixes.is_empty()` before clipping audio mixes to the requested output range. If a draft has audio elsewhere on the timeline but the current preview/export range has no overlapping audio, the loop skips every mix, `audio_labels` remains empty, and lines 168-178 still emit `amix=inputs=0` plus `[aout]`. `job.rs` then maps `[aout]` and marks `expect_audio_stream = true`, so a valid silent export range can fail compilation/execution or validate against a nonexistent audio stream.

**Fix:**
```rust
let mut audio_labels = Vec::new();
// populate labels from clipped mixes

let has_audio_output = !matches!(plan.output_profile, RenderOutputProfile::PreviewFrame { .. })
    && !audio_labels.is_empty();

if audio_labels.len() == 1 {
    lines.push(format!("[{}]anull[aout]", audio_labels[0]));
} else if audio_labels.len() > 1 {
    let inputs = audio_labels.iter().map(|label| format!("[{label}]")).collect::<String>();
    lines.push(format!("{inputs}amix=inputs={}:duration=longest:normalize=0[aout]", audio_labels.len()));
}
```
Add a snapshot where the graph contains an audio segment outside the export range and assert no `[aout]`, no `amix=inputs=0`, and `expect_audio_stream == false`.

### CR-02: Keyframed volume exports still succeed with wrong audio [BLOCKER]

**File:** `crates/ffmpeg_compiler/src/filters.rs:205`

**Issue:** The fix for the prior CR-04 adds `audio.volumeKeyframes` diagnostics at lines 280-292, but the compiled filter chain still only emits constant `volume={gain}` at line 205. `compile_ffmpeg_job` copies those diagnostics into `FfmpegJob` and still returns `Ok` at `crates/ffmpeg_compiler/src/job.rs:291`, so a draft with keyframed volume can export successfully while the produced audio ignores automation. A diagnostic attached to a successful render job is not a production-grade unsupported-feature boundary.

**Fix:** Either compile `audio.volume_keyframes` into an FFmpeg `volume` expression over timeline time, or make unsupported audio automation a hard compile error before job creation, for example an `UnsupportedAudioAutomation` `FfmpegCompileErrorKind`. Add a test that a draft with volume keyframes cannot produce a successful FFmpeg job until automation is actually compiled.

## Warnings

### WR-01: Native output still ignores part of the requested capability contract [WARNING]

**File:** `crates/audio_output_desktop/src/cpal_output.rs:144`

**Issue:** The fix for the prior WR-01 validates device id, sample rate, and maximum channels, but it only checks `requested.max_frame_count != 0` and never rejects `requested.max_frame_count > available.max_frame_count`. `open_stream` still opens `self.default_config` at lines 78-79 regardless of that requested frame bound. Callers can request a frame size the output contract does not support and still receive a stream, hiding mismatched preview/output buffering assumptions.

**Fix:** Extend `validate_native_output_capabilities` to reject frame counts above `available.max_frame_count`, and add a unit assertion mirroring the sample-rate/channel/device mismatch checks:
```rust
if requested.max_frame_count > available.max_frame_count {
    return Err(AudioOutputError::InvalidCapabilities {
        reason: format!(
            "native output frame count {} exceeds available {}",
            requested.max_frame_count, available.max_frame_count
        ),
    });
}
```

---

_Reviewed: 2026-06-19T12:23:31Z_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_
