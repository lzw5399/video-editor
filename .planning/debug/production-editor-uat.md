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
- next_action: "Continue broader production UAT gaps after this committed slice: combination video+audio+text+SRT parity, Rust canonical draft session, and product workbench hierarchy."
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

## Eliminated

## Resolution

- root_cause: "The preview surface boundary mixed BrowserWindow content-local DOM coordinates, Electron top-left screen coordinates, and raw AppKit bottom-left screen coordinates. Tests then self-validated by choosing the direct/flipped formula closest to native output."
- fix: "Removed direct/flipped heuristic; declared content-local logical bounds and Electron top-left screen telemetry; converted raw AppKit screen rect deterministically; changed macOS NSView frame placement to use parent_view.isFlipped(); removed scale-based frame math from the legacy AVPlayer bridge; hid product-mode preview/audio diagnostic chips."
- verification: "cargo fmt --all; cargo test -p realtime_preview_runtime platform::macos::tests -- --nocapture; cargo test -p bindings_node native_preview_presenter -- --nocapture; corepack pnpm --dir apps/desktop-electron run build; package:dir; product surface, audio, cadence, UI reference, and full product-user-journey Playwright gates passed."
- files_changed: "apps/desktop-electron/src/main/realtimePreviewHost.ts; apps/desktop-electron/src/renderer/workspace/PreviewMonitor.tsx; apps/desktop-electron/tests/product-user-journey.spec.ts; apps/desktop-electron/tests/helpers/userJourney.ts; crates/realtime_preview_runtime/src/platform/macos.rs; crates/bindings_node/src/native_preview_presenter.rs; apps/desktop-electron/package.json; pnpm-lock.yaml"
