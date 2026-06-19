---
phase: 15-audio-engine-and-dsp-timeline-pipeline
reviewed: 2026-06-19T12:04:42Z
depth: standard
files_reviewed: 59
files_reviewed_list:
  - Cargo.toml
  - apps/desktop-electron/src/generated/CommandEnvelope.ts
  - apps/desktop-electron/src/generated/CommandResultEnvelope.ts
  - apps/desktop-electron/src/generated/Draft.ts
  - apps/desktop-electron/src/main/index.ts
  - apps/desktop-electron/src/renderer/App.tsx
  - apps/desktop-electron/src/renderer/commandHelpers.ts
  - apps/desktop-electron/src/renderer/viewModel.ts
  - apps/desktop-electron/src/renderer/workspace/FeaturePanel.tsx
  - apps/desktop-electron/src/renderer/workspace/Inspector.tsx
  - apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx
  - apps/desktop-electron/src/renderer/workspace/Timeline.tsx
  - apps/desktop-electron/src/renderer/workspace/preview-inspector.css
  - apps/desktop-electron/src/renderer/workspace/timeline.css
  - apps/desktop-electron/tests/workspace.spec.ts
  - crates/audio_engine/Cargo.toml
  - crates/audio_engine/src/dsp_timeline.rs
  - crates/audio_engine/src/lib.rs
  - crates/audio_engine/src/mix_intent.rs
  - crates/audio_engine/src/output.rs
  - crates/audio_engine/src/session.rs
  - crates/audio_engine/src/telemetry.rs
  - crates/audio_engine/tests/audio_session_generation.rs
  - crates/audio_engine/tests/dsp_timeline.rs
  - crates/audio_output_desktop/Cargo.toml
  - crates/audio_output_desktop/src/cpal_output.rs
  - crates/audio_output_desktop/src/lib.rs
  - crates/audio_output_desktop/src/mock_output.rs
  - crates/audio_output_desktop/tests/audio_output_capabilities.rs
  - crates/audio_output_desktop/tests/native_audio.rs
  - crates/bindings_node/Cargo.toml
  - crates/bindings_node/src/audio_service.rs
  - crates/bindings_node/src/lib.rs
  - crates/bindings_node/tests/audio_service.rs
  - crates/draft_commands/src/audio.rs
  - crates/draft_commands/src/timeline.rs
  - crates/draft_commands/tests/text_audio_commands.rs
  - crates/draft_model/src/lib.rs
  - crates/draft_model/src/timeline.rs
  - crates/draft_model/src/validation.rs
  - crates/draft_model/tests/draft_schema.rs
  - crates/draft_model/tests/schema_exports.rs
  - crates/engine_core/src/normalize.rs
  - crates/ffmpeg_compiler/src/filters.rs
  - crates/ffmpeg_compiler/src/job.rs
  - crates/ffmpeg_compiler/tests/common/mod.rs
  - crates/ffmpeg_compiler/tests/ffmpeg_job_snapshots.rs
  - crates/render_graph/src/fingerprint.rs
  - crates/render_graph/src/graph.rs
  - crates/render_graph/src/lib.rs
  - crates/render_graph/tests/render_graph_snapshots.rs
  - crates/testkit/Cargo.toml
  - crates/testkit/src/audio_parity.rs
  - crates/testkit/src/lib.rs
  - crates/testkit/tests/audio_preview_export_parity.rs
  - package.json
  - schemas/command.schema.json
  - schemas/draft.schema.json
  - scripts/phase15-source-guards.sh
findings:
  critical: 5
  warning: 2
  info: 0
  total: 7
status: issues_found
---

# Phase 15: Code Review Report

**Reviewed:** 2026-06-19T12:04:42Z
**Depth:** standard
**Files Reviewed:** 59
**Status:** issues_found

## Summary

Reviewed the Phase 15 Rust audio semantics/runtime/output/bindings/export/testkit changes, Electron audio UI/workspace tests, generated contracts, and source guard script. The main failures are correctness issues in the production audio preview and export paths: UI playback is rejected by the Rust binding generation check, accepted play would still not seek the Rust clock, export audio loses timeline placement and volume automation, and waveform binding reports synthetic ready data.

## Critical Issues

### CR-01: Playback UI sends a stale generation [BLOCKER]

**File:** `apps/desktop-electron/src/renderer/App.tsx:1688`
**Issue:** `handlePlayAudioPreview` sends `current.audioPreview.generation + 1`, but `AudioPreviewBindingRegistry::play` rejects any generation that is not exactly equal to current Rust session status (`crates/bindings_node/src/audio_service.rs:63-74`). After session creation the UI has generation 0, sends 1, and the real binding returns `staleRejected` instead of `playing`. The Playwright test misses this because the Electron main test mock accepts every play command.
**Fix:**
```ts
buildPlayAudioPreviewCommand({
  draft: current.draft,
  sessionId,
  targetTime: playheadRef.current,
  playbackGeneration: current.audioPreview.generation
});
```
Add a binding-backed test or Rust integration test that creates a session, sends the exact UI play payload, and asserts `accepted == true`.

### CR-02: Play accepts targetTime but never moves the Rust clock [BLOCKER]

**File:** `crates/bindings_node/src/audio_service.rs:57`
**Issue:** `play(session_id, target_time, playback_generation)` returns `target_time` in the response, but it only calls `self.runtime.resume(runtime_id)` at lines 75-79. `TimelineClock::resume` changes state to playing and advances generation without updating position. After a user seeks the UI playhead and presses play, the response says the requested time is playing while `getAudioPreviewStatus` still reports the old Rust clock position.
**Fix:**
```rust
let runtime_id = self.runtime_session_id(session_id)?;
let generation = self
    .runtime
    .seek(runtime_id, target_time)
    .and_then(|_| self.runtime.resume(runtime_id))
    .map_err(AudioPreviewBindingError::runtime)?;
```
Then assert that `registry.play(..., Microseconds::new(600_000), current_generation)` followed by `registry.status(...)` reports `target_time == 600_000`.

### CR-03: Export audio ignores target timeline placement [BLOCKER]

**File:** `crates/ffmpeg_compiler/src/filters.rs:188`
**Issue:** `compile_audio_mix_filters` trims the source and resets PTS to zero (`asetpts=PTS-STARTPTS`) for every audio segment, then `generate_filter_script` immediately feeds all labels into `amix` at lines 166-176. There is no `adelay`, `asetpts` offset, or silence pad based on `RenderAudioMix.target_timerange.start` relative to the output range. In a full export, audio segments at 10s and 20s both start at 0s.
**Fix:** Pass the output timerange into `compile_audio_mix_filters`, compute `delay = active_target_start - output_start`, and add an FFmpeg-owned delay/PTS step before mixing, for example `adelay={delay_ms}|{delay_ms}` after trimming. Add a snapshot with an export range starting at zero and an audio segment whose `targetTimerange.start > 0`.

### CR-04: Export silently drops volume automation [BLOCKER]

**File:** `crates/ffmpeg_compiler/src/filters.rs:195`
**Issue:** Export only emits a constant `volume={gain}` plus pan/fades; `audio.volume_keyframes` is never compiled. Worse, `render_graph::volume_keyframes_for` writes every `RenderAudioVolumeKeyframe.target_sample` as the constant offset argument (`crates/render_graph/src/graph.rs:1016-1038`), and the parity helper normalizes preview samples to `0` (`crates/testkit/src/audio_parity.rs:339-347`), so tests cannot catch the mismatch. Keyframed volume previews and exports diverge.
**Fix:** Either compile `volume_keyframes` into an FFmpeg volume expression over timeline time, or explicitly classify keyframed volume as unsupported and fail/export a diagnostic until supported. Compute/export the real sample/time for each keyframe and update parity tests to compare it instead of forcing preview samples to zero.

### CR-05: Waveform binding returns fake ready peaks for any request [BLOCKER]

**File:** `crates/bindings_node/src/audio_service.rs:258`
**Issue:** `waveform_display_peaks` does not consult artifact status, material metadata, or generated waveform data. For any positive `max_peak_bins`, it synthesizes a repeating peak pattern and returns `WaveformDisplayStatus::Ready` at lines 277-296. The renderer can show "波形就绪" for an audio material whose waveform was never generated or has failed, which violates the derived-artifact boundary and misleads production UI.
**Fix:** Route this command through the artifact/resource status service or a Rust waveform display DTO backed by real generated peak data. Return `Pending`, `Missing`, or `Failed` unless a ready waveform artifact exists; keep the 512-bin cap when serializing display peaks.

## Warnings

### WR-01: Native output ignores requested capabilities and miscounts callbacks [WARNING]

**File:** `crates/audio_output_desktop/src/cpal_output.rs:72`
**Issue:** `CpalAudioOutputDevice::open_stream` only checks that requested capabilities are nonzero, then opens `self.default_config` regardless of the requested sample rate/channel count. `CpalAudioOutputSink` also shares `presented_result_count` with the CPAL data callback, so `presented_result_count()` is incremented both by `present()` and by each silent callback (lines 148-163 and 336-352). Capability mismatches can be hidden and telemetry/count assertions become unreliable.
**Fix:** Validate requested capabilities against `self.capabilities()` or build the stream from the requested supported config. Use separate counters for `present()` calls and CPAL callbacks, and expose a name that matches the metric.

### WR-02: Source guard excludes the production copy file [WARNING]

**File:** `scripts/phase15-source-guards.sh:219`
**Issue:** The production forbidden-copy scan excludes `viewModel.ts`, but `viewModel.ts` is where audio preview/device/waveform status labels and error copy are produced. A future regression could add user-visible "FFmpeg", "native backend", "cacheKey", or similar internal wording there and still pass `test:phase15-source-guards`.
**Fix:** Scan production string literals in `viewModel.ts`, or split internal transport formatting from user-facing labels and include the user-facing label module in `PRODUCTION_FORBIDDEN_COPY_PATTERN`.

---

_Reviewed: 2026-06-19T12:04:42Z_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_
