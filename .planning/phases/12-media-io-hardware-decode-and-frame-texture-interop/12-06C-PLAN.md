---
phase: "12-media-io-hardware-decode-and-frame-texture-interop"
plan: "06C"
type: execute
wave: 8
depends_on:
  - "12-06B"
files_modified:
  - "crates/media_runtime_desktop/tests/session_leaks.rs"
  - "scripts/phase12-source-guards.sh"
  - "package.json"
autonomous: true
requirements:
  - MEDIAIO-03
  - MEDIAIO-04
  - MEDIAIO-05
user_setup: []
must_haves:
  truths:
    - "MEDIAIO-03: release/session-close tests prove CPU frame, platform-opaque frame, and texture leases are released or reported as leaks."
    - "MEDIAIO-04: final source guards reject renderer-owned native media APIs, raw pointers, and full-frame handle payloads."
    - "MEDIAIO-05: final `test:phase12` runs the focused Rust, binding, source guard, generated contract, and manual-verification-note gates."
  artifacts:
    - path: "crates/media_runtime_desktop/tests/session_leaks.rs"
      provides: "release/session-close leak diagnostics tests"
    - path: "scripts/phase12-source-guards.sh"
      provides: "final source boundary, raw-byte, native-pointer, and ownership guard"
    - path: "package.json"
      provides: "`test:phase12` and source guard script wiring"
  key_links:
    - from: "crates/media_runtime_desktop/tests/session_leaks.rs"
      to: "crates/media_runtime/src/frame.rs"
      via: "frame pool release/session-close behavior"
      pattern: "close.*session|release"
    - from: "package.json"
      to: "scripts/phase12-source-guards.sh"
      via: "`test:phase12` script"
      pattern: "test:phase12"
---

<objective>
Add final Phase 12 release/session-close leak tests, tighten source guards, wire the focused `test:phase12` script, and require manual platform verification notes in the summary.

Purpose: MEDIAIO-03 through MEDIAIO-05 need a final feedback gate proving handle lifetime, source-boundary enforcement, fallback diagnostics, and platform acceptance checks are covered before verification.
Output: Session leak tests, final source guard checks, `test:phase12` script, and Windows/macOS manual verification note requirements.
</objective>

<execution_context>
@/Users/zhiwen/.codex/get-shit-done/workflows/execute-plan.md
@/Users/zhiwen/.codex/get-shit-done/templates/summary.md
</execution_context>

<context>
@AGENTS.md
@.planning/ROADMAP.md
@.planning/REQUIREMENTS.md
@.planning/STATE.md
@.planning/phases/12-media-io-hardware-decode-and-frame-texture-interop/12-CONTEXT.md
@.planning/phases/12-media-io-hardware-decode-and-frame-texture-interop/12-RESEARCH.md
@.planning/phases/12-media-io-hardware-decode-and-frame-texture-interop/12-DESIGN.md
@.planning/phases/12-media-io-hardware-decode-and-frame-texture-interop/12-VALIDATION.md
@.planning/phases/12-media-io-hardware-decode-and-frame-texture-interop/12-06B-PLAN.md
@scripts/phase12-source-guards.sh
@package.json
</context>

## Artifacts this plan produces

- session close cascading release behavior tests
- CPU/platform-opaque/texture storage leak diagnostics tests
- final `test:phase12` script
- comment-filtered source guard count checks
- Windows/macOS manual verification notes in `12-06C-SUMMARY.md`

<tasks>

<task type="auto" tdd="true">
  <name>Task 12-06C-01: Add release/session-close leak diagnostics tests</name>
  <files>crates/media_runtime_desktop/tests/session_leaks.rs</files>
  <read_first>
    - `crates/media_runtime/src/frame.rs`
    - `crates/media_runtime/src/texture.rs`
    - `crates/media_runtime_desktop/src/lib.rs`
    - `.planning/phases/12-media-io-hardware-decode-and-frame-texture-interop/12-DESIGN.md`
  </read_first>
  <behavior>
    - Test 1: closing a media session releases unreleased CPU frame leases and records leak diagnostics with owner session and generation.
    - Test 2: closing a media session releases unreleased platform-opaque frame leases and records leak diagnostics.
    - Test 3: closing a media session releases unreleased texture handle leases and records backend, `RuntimeDeviceId`, compatibility state, owner session, and generation in diagnostics.
  </behavior>
  <action>Per MEDIAIO-03 and MEDIAIO-05, add desktop runtime tests that exercise explicit release and cascading session close for CPU, platform-opaque, and texture storage. The tests should use fake or in-memory runtime sessions when native platforms are unavailable, and they must assert leak diagnostics rather than silently dropping unreleased handles. Keep native pointer access opaque and keep all timing values in integer microseconds, frame indices, or rational frame rates.</action>
  <acceptance_criteria>
    Release/session-close behavior is tested for every Phase 12 storage kind, leak diagnostics include session/generation/device context, and tests pass without requiring native platform hardware in CI.
  </acceptance_criteria>
  <verify>
    <automated>cargo test -p media_runtime_desktop session_leaks -- --nocapture</automated>
    <automated>cargo test -p media_runtime frame_pool -- --nocapture</automated>
  </verify>
  <done>Task complete when session leak diagnostics tests pass and cover CPU, platform-opaque, and texture storage.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 12-06C-02: Tighten source guards, wire final phase gate, and record platform verification notes</name>
  <files>scripts/phase12-source-guards.sh, package.json</files>
  <read_first>
    - `scripts/phase12-source-guards.sh`
    - `package.json`
    - `.planning/phases/12-media-io-hardware-decode-and-frame-texture-interop/12-VALIDATION.md`
  </read_first>
  <behavior>
    - Test 1: source guards reject renderer-owned FFmpeg/native media API usage, raw native pointer exposure, full-frame handle response byte payloads, and direct render graph/timeline ownership in Phase 12 paths.
    - Test 2: count-based grep gates filter comments with `grep -v '^#'` or equivalent before evaluating forbidden tokens.
    - Test 3: final `test:phase12` runs Rust contract, desktop fallback, binding, source guard, and generated contract drift gates.
    - Test 4: `12-06C-SUMMARY.md` records Windows and macOS manual verification notes for native capability, H.264 MP4/MOV first-frame decode, fallback diagnostics, texture compatibility, and session-close leak diagnostics.
  </behavior>
  <action>Per MEDIAIO-04 and MEDIAIO-05, extend `scripts/phase12-source-guards.sh` and add a root `test:phase12` script that runs the focused Phase 12 gates from `12-VALIDATION.md`. The source guard must use `rg` checks and avoid bare comment-counted `grep -c token == 0` gates; where counts matter, filter comments before counting. Include manual verification instructions in `12-06C-SUMMARY.md` for Windows and macOS native capability/decode checks, fallback diagnostics, texture compatibility, and leak diagnostics. Do not edit product behavior outside the listed files.</action>
  <acceptance_criteria>
    Final Phase 12 gate script passes, source guards are comment-safe and enforce ownership boundaries, generated contract drift is checked, and platform manual verification notes are recorded.
  </acceptance_criteria>
  <verify>
    <automated>pnpm run test:phase12-source-guards</automated>
    <automated>pnpm run test:phase12</automated>
    <human-check>On Windows and macOS, verify native capability reports, H.264 MP4/MOV first-frame decode, fallback diagnostics, texture compatibility, and session-close leak diagnostics; record results in `12-06C-SUMMARY.md`.</human-check>
  </verify>
  <done>Task complete when final automated gates pass and platform manual verification results are recorded in the summary.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| session close -> native resources | CPU frames and native texture leases are released or reported as leaks. |
| source tree -> phase verification | Source guards classify boundary violations before Phase 12 is accepted. |
| manual platform verification -> summary | Windows/macOS hardware results are recorded as evidence, not assumed. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-12-06C-01 | Denial of Service | unreleased frame/texture leases | mitigate | Session-close cascading release and leak diagnostics are tested for every storage kind. |
| T-12-06C-02 | Information Disclosure | binding/source guards | mitigate | Source guards reject full-frame bytes, native pointer strings, platform objects, and renderer-owned platform APIs. |
| T-12-06C-03 | Repudiation | manual platform results | mitigate | Summary must record platform, codec fixture, selected path, fallback reason, texture compatibility, and leak diagnostic outcome. |
| T-12-06C-SC | Tampering | npm/pip/cargo installs | accept | This plan adds no new packages beyond those verified in 12-02B. |
</threat_model>

<verification>
<automated>cargo test -p media_runtime_desktop session_leaks -- --nocapture</automated>
<automated>cargo test -p media_runtime frame_pool -- --nocapture</automated>
<automated>pnpm run test:phase12-source-guards</automated>
<automated>pnpm run test:phase12</automated>
<human-check>Windows/macOS manual verification recorded: native capability report, H.264 MP4/MOV first-frame decode, fallback diagnostics, texture compatibility, release/session-close leak diagnostics.</human-check>
</verification>

<source_audit>
GOAL | Finish Phase 12 with validated lifetime, fallback, and source-boundary enforcement | 12-06C adds leak tests, final guards, final script, and manual platform notes | COVERED
REQ | MEDIAIO-03 | release/session-close leak tests prove frame pool and texture lease ownership | COVERED
REQ | MEDIAIO-04 | source guards reject raw bytes, native pointers, and renderer-owned native APIs | COVERED
REQ | MEDIAIO-05 | final gates cover fallback diagnostics and platform acceptance notes | COVERED
RESEARCH | No hidden CPU copy, color/device diagnostics, and fallback visibility are common pitfalls | 12-06C final gates and manual notes | COVERED
CONTEXT | Phase 12 must define executable gates before implementation is complete | `test:phase12` and `12-VALIDATION.md` are enforced | COVERED
</source_audit>

<success_criteria>
Release/session-close leaks are tested, final source guards and `test:phase12` pass, generated contract drift is checked, and Windows/macOS manual platform verification notes are recorded with H.264 MP4/MOV acceptance and fallback outcomes.
</success_criteria>

<output>
Create `.planning/phases/12-media-io-hardware-decode-and-frame-texture-interop/12-06C-SUMMARY.md` when done.
</output>
