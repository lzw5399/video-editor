---
status: resolved
trigger: "Production-grade desktop video editor usability audit: use the real app as a normal Jianying/CapCut-style user, find broken basic editing flows and repair production issues, especially preview stutter and synchronous UI-driven frame pumping."
created: 2026-06-21
updated: 2026-06-21
---

# Debug Session: basic-editing-flow-audit

## Symptoms

- expected_behavior: "A user can import media, place clips on a Jianying-style timeline, preview smoothly, seek/scrub, split/trim/move/delete, use text/transform controls, save/open, and export with Rust core owning playback/render semantics."
- actual_behavior: "Previous evidence showed the product could pass weak gates while still presenting only about 21 frames over 3 seconds and blocking snapshot calls. Recent fixes moved cadence to a Rust worker, but the whole normal-user editing flow still needs product evidence and production-grade review."
- error_messages: "No single crash signature yet. Need real app telemetry, screenshots, and UI observations."
- timeline: "Started during preview cadence and basic editing verification; current repo includes a Rust worker preview fix that must be re-audited in context."
- reproduction: "Run the actual Electron desktop app with real fixture media and exercise import, timeline placement, playback, seek/scrub, editing commands, text/transform where available, save/open, and export."

## Current Focus

- hypothesis: "The known bad synchronous UI-driven preview cadence and low-level renderer edit command path have been replaced for the current product gates, but full production decode-ahead/session-owned architecture remains staged future work."
- test: "Run Rust command/schema tests, source guards, desktop build, product cadence/user journey, dev real workflow, package:dir, packaged smoke, and packaged real workflow."
- expecting: "3s 30fps cadence accounts for 90 frames with media clock progression >=2.9s, presentation snapshot calls stay lightweight, renderer main flow uses Rust-owned intent commands, and packaged app gates pass."
- next_action: "Open a follow-up architecture phase for decode-ahead queues, Rust canonical project sessions, broader edit/export parity, and UI information hierarchy cleanup."
- reasoning_checkpoint: "Use production-architecture-review output shape: Decision, Current chain, Production target, Gap, Required action, Verification gates."
- tdd_checkpoint: "Strengthened cadence and source guards now fail the previous known-bad states; product and packaged gates passed on 2026-06-21."

## Evidence

- timestamp: 2026-06-21 04:04:04 +0800
  observation: "Official Rust toolchain is active from ~/.cargo/bin after removing Homebrew rust wrappers."
  command: "which cargo && which rustup && rustc --version"
  result: "cargo/rustup resolve under /Users/zhiwen/.cargo/bin; rustc 1.95.0-aarch64-apple-darwin."
- timestamp: 2026-06-21 04:04:04 +0800
  observation: "Renderer product edit flow now uses Rust-owned intent commands for segment add/move/split/trim plus text/audio/track creation."
  command: "cargo test -p draft_commands timeline_edits -- --nocapture"
  result: "2 passed, including intent_timeline_edits_are_rust_owned."
- timestamp: 2026-06-21 04:04:04 +0800
  observation: "Command schema/generated contracts include the new intent commands and remain generated from Rust."
  command: "cargo test -p draft_model schema_exports -- --nocapture"
  result: "19 passed."
- timestamp: 2026-06-21 04:04:04 +0800
  observation: "Phase 3 source guard is a real failing gate for renderer/main boundary violations instead of a warning-only script."
  command: "corepack pnpm run test:phase3-source-guards"
  result: "passed."
- timestamp: 2026-06-21 04:04:04 +0800
  observation: "Desktop native binding and Electron renderer build with the official Rust toolchain."
  command: "corepack pnpm --dir apps/desktop-electron run build"
  result: "passed; only existing Node engine and AVAsset deprecated API warnings."
- timestamp: 2026-06-21 04:04:04 +0800
  observation: "Product preview cadence no longer uses artifact fallback or synchronous snapshot frame pump."
  command: "corepack pnpm --dir apps/desktop-electron exec playwright test tests/product-preview-cadence.spec.ts tests/product-user-journey.spec.ts --reporter=line --workers=1"
  result: "10 passed. Cadence metrics: presentedDelta=89, droppedDelta=1, accountedFrameDelta=90, targetDeltaMicroseconds=2966637, p50/p95 getPresentationState=0ms, requestPreviewFrame delta=0."
- timestamp: 2026-06-21 04:04:04 +0800
  observation: "Real no-mock workflow covers import, preview, save/reopen, and export after text material display-name fix."
  command: "corepack pnpm --dir apps/desktop-electron exec playwright test tests/real-workflow.spec.ts --grep \"dev\" --reporter=line --workers=1"
  result: "1 passed."
- timestamp: 2026-06-21 04:04:04 +0800
  observation: "package:dir no longer reproduces the earlier hang; electron-builder completes and generates out/mac-arm64/Video Editor.app."
  command: "DEBUG=electron-builder ELECTRON_BUILDER_DEBUG=1 corepack pnpm --dir apps/desktop-electron run package:dir"
  result: "passed in about 4 seconds; debug log showed Electron zip extracted, node modules collected, signing skipped because identity=null."
- timestamp: 2026-06-21 04:04:04 +0800
  observation: "Packaged app loads file renderer/preload/native binding and real packaged workflow passes."
  command: "corepack pnpm --dir apps/desktop-electron exec playwright test tests/packaged-smoke.spec.ts tests/real-workflow.spec.ts --grep \"packaged\" --reporter=line --workers=1"
  result: "3 passed."

## Eliminated

- hypothesis: "The remaining packaged failure is caused by electron-builder not producing an app."
  evidence: "package:dir completed successfully and packaged real workflow passed. The smoke failure was an outdated expectation that launch enters the workspace directly; current product correctly starts at the project entry."
- hypothesis: "The product preview cadence still depends on requestPreviewFrame artifact fallback."
  evidence: "Product cadence telemetry reported frameRequestsBefore=0 and frameRequestsAfter=0 while renderGraphActive=true."
- hypothesis: "Text save/reopen failure is a Rust text command persistence bug."
  evidence: "After fixing the test selector to the user-entered text display name, dev real workflow passed."

## Resolution

- root_cause: "The preview gate previously accepted a known-bad synchronous cadence, and renderer edit commands still leaked low-level timeline construction. Several tests also assumed old UI/default text behavior."
- fix: "Moved product playback evidence to Rust-owned worker cadence with wall-clock/drop accounting, added Rust intent commands and renderer helpers for product edit flows, strengthened source/cadence gates, fixed text material display names, aligned packaged smoke with the project entry, and verified package:dir on a real generated app."
- verification: "cargo fmt --all; cargo test -p draft_commands timeline_edits -- --nocapture; cargo test -p draft_model schema_exports -- --nocapture; corepack pnpm run test:phase3-source-guards; corepack pnpm --dir apps/desktop-electron run build; product cadence/user journey 10 passed twice; dev real workflow 1 passed; package:dir passed; packaged smoke + packaged real workflow 3 passed."
- files_changed: "crates/bindings_node, crates/realtime_preview_runtime, crates/draft_model, crates/draft_commands, schemas/command.schema.json, generated CommandEnvelope, renderer App/commandHelpers/tests, package source guard, and this debug session note."
