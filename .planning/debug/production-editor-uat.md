---
status: investigating
trigger: "Production editor UAT from ordinary Jianying-style user perspective, based on pasted review requirements: UI reference screenshots, combination playback, Rust-owned semantics/session, preview scheduler, preview/export parity, FFmpeg reliability, and hard verification gates."
created: 2026-06-21
updated: 2026-06-21
---

# Debug Session: production-editor-uat

## Symptoms

- expected_behavior: "A normal user can launch the desktop editor, create/open a project, import video/audio/subtitle materials, add video + external audio + text + SRT subtitles, play smoothly with centered real GPU preview and native audio, see subtitles change over time, save/reopen/export, and get a Jianying-like production workbench without diagnostic/fallback noise."
- actual_behavior: "Previous fixes improved preview cadence and intent commands, but pasted review calls out remaining product gaps: UI hierarchy, combination playback coverage, Rust session ownership, preview/export parity, FFmpeg distribution, and screenshot gates are incomplete."
- error_messages: "No single crash signature yet. Need foreground product screenshots, telemetry, code-path review, and failing/product gates."
- timeline: "After c41a975 fixed preview cadence and initial intent editing flow."
- reproduction: "Use the actual Electron app as a normal user. Compare against docs/ui-reference/jianying-pro/screenshots, exercise video/audio/text/SRT playback and export, and inspect architecture boundaries."

## Current Focus

- hypothesis: "The immediate P0 surface issue was a native preview placement contract bug: renderer/main used Electron top-left logical pixels while Rust/AppKit reported bottom-left screen coordinates, and old tests accepted heuristic direct/flipped alignment."
- test: "Verify WGPU native child view placement during real playback by comparing DOM preview host screen rect, Electron-converted native child rect, and visible preview pixel motion."
- expecting: "Native surface placement telemetry reports a single coordinate contract and Playwright fails if the child view is shifted off the DOM preview host."
- next_action: "Continue broader production UAT gaps after this slice: Rust canonical draft session, product workbench hierarchy, and permanent preview worker architecture."
- reasoning_checkpoint: "Use production-architecture-review output shape: Decision, Current chain, Production target, Gap, Required action, Verification gates."
- tdd_checkpoint: "Before product code edits, add or strengthen gates that would fail the observed bad UI/playback/parity state."

## Evidence

- timestamp: 2026-06-21
  observation: "External audit flagged preview video shifting left/bottom as a DOM/Electron/AppKit/native surface coordinate-contract bug, not a CSS/y-offset issue."
  data: "Repo inspection confirmed renderer sends .preview-native-host getBoundingClientRect() content-local logical pixels; Electron main previously selected direct/flipped screen rect based on whichever was closer to native; Rust macOS WGPU and legacy AVPlayer paths both used parent_height - dom_y - height guessing."
- timestamp: 2026-06-21
  observation: "First strengthened Playwright run failed with native AppKit rect y=588 vs expected Electron top-left y=324, proving the gate catches the known bad state and that AppKit screen rects must be converted to Electron top-left logical screen coordinates."
  data: "Failure delta y=264 before deterministic AppKit bottom-left -> Electron top-left conversion."
- timestamp: 2026-06-21
  observation: "After fix, native surface placement UAT passed in packaged macOS app."
  data: "corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --grep \"native surface aligned\" -> 1 passed."
- timestamp: 2026-06-21
  observation: "Product playback cadence remained at production gate after placement changes."
  data: "product-preview-cadence.spec.ts -> accountedFrameDelta=90, presentedDelta=89, droppedDelta=1, targetDeltaMicroseconds=2966637, p50/p95 presentationDurationMs=0."
- timestamp: 2026-06-21
  observation: "Full product user journey passed after hiding product-mode diagnostics and updating audio UAT away from center preview chips."
  data: "corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts -> 9 passed."
- timestamp: 2026-06-21
  observation: "Standalone TypeScript type-check remains blocked by pre-existing broad type debt and is not a usable gate for this slice yet."
  data: "corepack pnpm --dir apps/desktop-electron exec tsc --noEmit initially cannot find node types; adding node types exposed generated schema missing symbols, React/global Window declaration conflicts, and existing main/test narrowing errors, so the dependency experiment was not kept in this slice."
- timestamp: 2026-06-21
  observation: "Combination video + external audio + normal text + two-cue SRT preview was genuinely stuttery before the fix."
  data: "New combo cadence gate initially measured presentedDelta=6, droppedDelta=71, accountedFrameDelta=77, targetDeltaMicroseconds=2533308. After text texture cache it improved to presentedDelta=82/droppedDelta=8, proving per-frame text raster/upload was a bottleneck but first-frame cold start still consumed playback window time."
- timestamp: 2026-06-21
  observation: "Text preview now caches static GPU text textures and bundled font parsing, and playback worker prewarms the first frame in its own thread-local media pipeline before audio starts."
  data: "Final combo cadence: presentedDelta=90, droppedDelta=0, accountedFrameDelta=90, targetDeltaMicroseconds=2966637, visibleChanged=true, renderDurationMs=7. Single-video cadence: presentedDelta=90, droppedDelta=0."
- timestamp: 2026-06-21
  observation: "Preview/export subtitle parity found and fixed a real ASS timing bug."
  data: "ASS event times now use centiseconds instead of three-digit milliseconds; preview_export_parity_burns_two_cue_srt_text_into_frames validates two SRT cues burn different pixels and preview/export RGB parity for cue one."
- timestamp: 2026-06-21
  observation: "Combination preview no longer relies only on total presented-frame count; Rust worker telemetry now exposes frame pacing samples and the product cadence gate fails if interval p95/max or scheduler lateness regresses."
  data: "corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-preview-cadence.spec.ts --reporter=line -> 2 passed. Single video: presentedDelta=90, droppedDelta=0, intervalP50Ms=34, intervalP95Ms=35, intervalMaxMs=38. Video+external audio+text+two-cue SRT: presentedDelta=90, droppedDelta=0, intervalP50Ms=34, intervalP95Ms=36, intervalMaxMs=45, scheduleLatenessP95Ms=4."
- timestamp: 2026-06-21
  observation: "User-reported video+text preview stutter was rechecked on the current worktree. Sustained playback did not reproduce dropped frames, but combo first-frame latency remains visibly higher than the single-video baseline."
  data: "Targeted combo cadence: presentedDelta=90, droppedDelta=0, accountedFrameDelta=90, targetDeltaMicroseconds=2966637, intervalP50Ms=33, intervalP95Ms=39, intervalMaxMs=51, renderDurationMs=6, firstFrameLatencyMs=387. Repeat-each=3: single video firstFrameLatencyMs=78/86/88; combo firstFrameLatencyMs=343/414/463 with all combo runs presentedDelta=90 and droppedDelta=0. Visible combo UAT passed: corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --grep \"composites video external audio text\" --reporter=line -> 1 passed."
- timestamp: 2026-06-21
  observation: "Product SRT import was migrated from renderer-owned structural command data to a Rust-owned intent command. During verification the first intent implementation exposed a real semantic bug: it reused the selected normal text/title track and caused subtitle overlap with the existing text segment. Rust now only reuses subtitle tracks and otherwise creates a dedicated subtitle track."
  data: "Contract and command gates: cargo test -p draft_model schema_exports -- --nocapture -> 19 passed; cargo test -p draft_commands --test subtitle_commands -- --nocapture -> 8 passed; cargo test -p bindings_node --test text_commands -- --nocapture -> 4 passed. Product gates after package:dir: workspace SRT intent path -> 1 passed; product combo visible UAT -> 1 passed; product-preview-cadence.spec.ts -> 2 passed. Final combo cadence: presentedDelta=90, droppedDelta=0, accountedFrameDelta=90, targetDeltaMicroseconds=2966637, intervalP50Ms=34, intervalP95Ms=36, intervalMaxMs=46, scheduleLatenessP95Ms=5, renderDurationMs=8, firstFrameLatencyMs=335."
- timestamp: 2026-06-21
  observation: "The top-level 字幕 category was a false deferred entry while SRT import was hidden under the 文字 panel. The product route now exposes SRT import directly in 字幕, removes 字幕 from deferred category gates, and product helpers import subtitles from the 字幕 tab."
  data: "corepack pnpm --dir apps/desktop-electron run build -> passed; workspace captions gates -> 3 passed; package:dir -> passed; product combo visible UAT -> 1 passed; product-preview-cadence.spec.ts -> 2 passed with single video presentedDelta=90/droppedDelta=0 and combo presentedDelta=90/droppedDelta=0, combo intervalP95Ms=37, intervalMaxMs=55, firstFrameLatencyMs=323; test:phase3-source-guards -> passed; test:phase10-1-source-guards -> passed."
- timestamp: 2026-06-21
  observation: "First Rust-owned project session boundary landed behind bindings_node. openProjectSession loads canonical .veproj/project.json into a Rust registry; executeProjectIntent accepts sessionId + expectedRevision + intent only, rejects renderer draft fields, applies timeline semantics against session state, persists project.json, and increments revision."
  data: "cargo fmt --all --check -> passed; cargo test -p bindings_node --test project_session -- --nocapture -> 4 passed; cargo test -p bindings_node -- --nocapture -> passed; corepack pnpm --dir apps/desktop-electron run build -> passed; test:phase3-source-guards -> passed; test:phase10-1-source-guards -> passed; package:dir -> passed. The slice includes addTimelineSegmentIntent plus undo/redo across calls using Rust-owned CommandState; renderer main wrapper is available but product UI is not migrated yet."

## Eliminated

## Resolution

- root_cause: "The preview surface boundary mixed BrowserWindow content-local DOM coordinates, Electron top-left screen coordinates, and raw AppKit bottom-left screen coordinates. Tests then self-validated by choosing the direct/flipped formula closest to native output."
- fix: "Removed direct/flipped heuristic; declared content-local logical bounds and Electron top-left screen telemetry; converted raw AppKit screen rect deterministically; changed macOS NSView frame placement to use parent_view.isFlipped(); removed scale-based frame math from the legacy AVPlayer bridge; hid product-mode preview/audio diagnostic chips; added bounded realtime text texture cache, bundled font parse cache, worker-thread first-frame prewarm, SRT parity fixes, and combination cadence gates."
- verification: "cargo fmt --all --check; cargo test -p realtime_preview_runtime --lib -- --nocapture; cargo test -p bindings_node -- --nocapture; cargo test -p testkit --test preview_export_parity -- --nocapture; cargo test -p ffmpeg_compiler -- --nocapture; cargo test -p draft_commands --test subtitle_commands -- --nocapture; corepack pnpm --dir apps/desktop-electron run build; corepack pnpm --dir apps/desktop-electron run package:dir; corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-preview-cadence.spec.ts --reporter=line; corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-user-journey.spec.ts --reporter=line."
- files_changed: "apps/desktop-electron/src/main/realtimePreviewHost.ts; apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx; apps/desktop-electron/tests/product-preview-cadence.spec.ts; apps/desktop-electron/tests/product-user-journey.spec.ts; apps/desktop-electron/tests/helpers/userJourney.ts; crates/realtime_preview_runtime/src/platform/macos.rs; crates/realtime_preview_runtime/src/gpu/compositor.rs; crates/realtime_preview_runtime/src/gpu/text.rs; crates/realtime_preview_runtime/src/gpu/texture_cache.rs; crates/bindings_node/src/native_preview_presenter.rs; crates/bindings_node/src/realtime_preview_service.rs; crates/bindings_node/tests/canvas_commands.rs; crates/ffmpeg_compiler/src/ass.rs; crates/ffmpeg_compiler/src/filters.rs; crates/testkit/src/render_compare.rs; crates/testkit/tests/preview_export_parity.rs; scripts/phase15-3-desktop-gate.sh"
