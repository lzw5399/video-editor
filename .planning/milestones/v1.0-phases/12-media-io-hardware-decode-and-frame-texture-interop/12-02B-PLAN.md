---
phase: "12-media-io-hardware-decode-and-frame-texture-interop"
plan: "02B"
type: execute
wave: 3
depends_on:
  - "12-01"
  - "12-02"
files_modified:
  - "Cargo.lock"
  - "crates/media_runtime_desktop/Cargo.toml"
  - "crates/draft_model/src/lib.rs"
  - "crates/draft_model/tests/schema_exports.rs"
  - "crates/bindings_node/src/runtime_capability_service.rs"
  - "crates/bindings_node/tests/runtime_capabilities.rs"
  - "schemas/command.schema.json"
  - "scripts/phase12-source-guards.sh"
  - "package.json"
autonomous: false
requirements:
  - MEDIAIO-02
  - MEDIAIO-04
  - MEDIAIO-05
user_setup: []
must_haves:
  truths:
    - "MEDIAIO-02: binding-facing runtime capability contracts expose native media IO capability data from 12-02."
    - "MEDIAIO-04: binding-facing contracts expose handles and metadata, not full-frame 4K byte payloads for handle-capable paths."
    - "MEDIAIO-05: platform dependencies are approved before native Windows/macOS plans rely on them."
  artifacts:
    - path: "crates/draft_model/src/lib.rs"
      provides: "binding-safe generated capability and handle response contract types"
    - path: "crates/bindings_node/src/runtime_capability_service.rs"
      provides: "capability service mapping desktop runtime capabilities into command results"
    - path: "schemas/command.schema.json"
      provides: "schema export for runtime capability and handle-safe response contracts"
    - path: "scripts/phase12-source-guards.sh"
      provides: "source guard preventing renderer/native-pointer/full-frame-byte ownership violations"
    - path: "crates/media_runtime_desktop/Cargo.toml"
      provides: "target-specific platform dependency declarations after blocking package approval"
  key_links:
    - from: "crates/bindings_node/src/runtime_capability_service.rs"
      to: "crates/media_runtime_desktop/src/capabilities.rs"
      via: "desktop capability aggregation"
      pattern: "probe_desktop_runtime_capabilities"
    - from: "crates/draft_model/tests/schema_exports.rs"
      to: "schemas/command.schema.json"
      via: "generated schema export"
      pattern: "RuntimeCapabilityReport"
    - from: "scripts/phase12-source-guards.sh"
      to: "apps/desktop-electron"
      via: "renderer boundary grep checks"
      pattern: "MediaFoundation|VideoToolbox|ArrayBuffer|Uint8Array"
---

<objective>
Map Phase 12 runtime capability reports into binding/schema contracts, add no-full-frame-byte source guards, and perform the blocking platform dependency legitimacy checkpoint before native platform implementation plans.

Purpose: MEDIAIO-04 requires binding-visible handle contracts and source guards, while MEDIAIO-05 requires package legitimacy approval before native Windows/macOS dependencies are added.
Output: Binding/schema capability contracts, source guard script coverage, package verification checkpoint, and target-specific platform dependency declarations after approval.
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
@.planning/phases/12-media-io-hardware-decode-and-frame-texture-interop/12-02-PLAN.md
@crates/draft_model/src/lib.rs
@crates/draft_model/tests/schema_exports.rs
@crates/bindings_node/src/runtime_capability_service.rs
@crates/bindings_node/tests/runtime_capabilities.rs
@crates/media_runtime_desktop/Cargo.toml
@package.json
</context>

## Artifacts this plan produces

- binding-safe generated capability contract additions
- runtime capability binding tests
- `scripts/phase12-source-guards.sh`
- `test:phase12-source-guards`
- target-specific platform dependency declarations after approval
- package verification checkpoint summary in `12-02B-SUMMARY.md`

<tasks>

<task type="auto" tdd="true">
  <name>Task 12-02B-01: Generate binding-safe capability contracts and source guards</name>
  <files>crates/draft_model/src/lib.rs, crates/draft_model/tests/schema_exports.rs, crates/bindings_node/src/runtime_capability_service.rs, crates/bindings_node/tests/runtime_capabilities.rs, schemas/command.schema.json, scripts/phase12-source-guards.sh, package.json</files>
  <read_first>
    - `crates/draft_model/src/lib.rs`
    - `crates/draft_model/tests/schema_exports.rs`
    - `crates/bindings_node/src/runtime_capability_service.rs`
    - `crates/bindings_node/tests/runtime_capabilities.rs`
    - `.planning/phases/12-media-io-hardware-decode-and-frame-texture-interop/12-02-PLAN.md`
    - `package.json`
  </read_first>
  <behavior>
    - Test 1: `probeRuntimeCapabilities` returns generated contract data with `mediaIo`, native platform domains, fallback ladder, texture compatibility state, codec status, and existing FFmpeg fields.
    - Test 2: schema output contains handle response metadata fields but no `bytes`, `pixels`, `rgba`, `bgra`, `ArrayBuffer`, or `Uint8Array` field on handle-capable frame/texture responses.
    - Test 3: source guard fails if renderer code imports platform media APIs, constructs FFmpeg commands, exposes native pointers, or adds raw frame byte payloads to binding response types.
  </behavior>
  <action>Per MEDIAIO-04, map the extended desktop capability report from 12-02 through `bindings_node` into Rust-generated command result contracts owned by `draft_model`. Export schemas with contract drift checks. Add `scripts/phase12-source-guards.sh` and a `test:phase12-source-guards` package script. The guard must reject renderer ownership of Media Foundation, DXVA, D3D, AVFoundation, VideoToolbox, CoreVideo, Metal, FFmpeg fallback selection, native pointers, and full-frame byte fields on decoded frame or texture responses. Use `rg` checks that filter comments before count-based gates where comments could invalidate the result. Allow existing preview artifact response fields that are not handle-capable decoded frame or texture payloads.</action>
  <acceptance_criteria>
    Binding tests prove the new capability report shape; generated schema is clean; the source guard blocks raw full-frame JS/Rust payloads for handle-capable paths while preserving existing preview artifact responses.
  </acceptance_criteria>
  <verify>
    <automated>cargo test -p bindings_node runtime_capabilities -- --nocapture</automated>
    <automated>cargo test -p draft_model schema_exports -- --nocapture</automated>
    <automated>pnpm run test:phase12-source-guards</automated>
    <automated>git diff --exit-code schemas</automated>
  </verify>
  <done>Task complete when binding/schema contracts expose capability and handle metadata without full-frame byte payloads, and guard commands pass.</done>
</task>

<task type="checkpoint:human-verify" gate="blocking-human">
  <name>Task 12-02B-02: Verify platform dependency legitimacy before native implementation plans</name>
  <files>Cargo.lock, crates/media_runtime_desktop/Cargo.toml</files>
  <read_first>
    - `.planning/phases/12-media-io-hardware-decode-and-frame-texture-interop/12-RESEARCH.md`
    - `crates/media_runtime_desktop/Cargo.toml`
    - `Cargo.toml`
  </read_first>
  <action>Before 12-04 and 12-05 add or rely on platform Cargo dependencies, present the package legitimacy audit from `12-RESEARCH.md` to the user and verify the exact crates planned for target-specific dependencies: `windows` for Windows APIs, and `objc2`, `objc2-av-foundation`, `objc2-video-toolbox`, `objc2-core-video`, `objc2-core-media`, and `objc2-metal` or an explicitly approved equivalent for macOS APIs. If the user rejects any crate, record the rejection and selected equivalent in `12-02B-SUMMARY.md`; do not let 12-04 or 12-05 add an unapproved replacement. If approved, add target-specific dependency sections in `crates/media_runtime_desktop/Cargo.toml` and update `Cargo.lock` so 12-04 and 12-05 do not need to edit Cargo dependency files in parallel.</action>
  <acceptance_criteria>
    The user has explicitly approved the dependency set or approved equivalent platform binding approach, `12-02B-SUMMARY.md` records the approved versions/registries and any rejected crates, and Cargo metadata resolves after target-specific dependency entries are added.
  </acceptance_criteria>
  <verify>
    <automated>cargo metadata --locked --format-version 1 >/tmp/video-editor-phase12-cargo-metadata.json</automated>
    <human-check>Verify the package audit against crates.io pages and source repositories, then type "approved" or list rejected crates and approved equivalents.</human-check>
  </verify>
  <done>Task complete when dependency legitimacy is approved, dependency files resolve, and the approval record exists before 12-04 or 12-05 proceeds.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Rust binding -> Electron renderer | Generated contracts cross into JS without exposing native pointers or full frame buffers. |
| Cargo registry -> desktop runtime | New platform binding crates enter native media IO implementation. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-12-02B-01 | Information Disclosure | binding contracts | mitigate | Source guard rejects native pointer fields and full-frame byte payloads in handle-capable responses. |
| T-12-02B-02 | Tampering | generated contracts | mitigate | `schema_exports` and `git diff --exit-code schemas` enforce generated-contract drift checks. |
| T-12-02B-03 | Elevation of Privilege | decoded frame handles | mitigate | Binding contracts carry owner session, generation, backend, and device compatibility metadata instead of raw platform objects. |
| T-12-02B-SC | Tampering | Cargo dependencies | mitigate | Blocking human package verification before platform dependency additions; no unapproved replacement dependency proceeds to 12-04 or 12-05. |
</threat_model>

<verification>
<automated>cargo test -p bindings_node runtime_capabilities -- --nocapture</automated>
<automated>cargo test -p draft_model schema_exports -- --nocapture</automated>
<automated>pnpm run test:phase12-source-guards</automated>
<automated>git diff --exit-code schemas</automated>
<automated>cargo metadata --locked --format-version 1 >/tmp/video-editor-phase12-cargo-metadata.json</automated>
<human-check>Windows/macOS package dependency legitimacy approved before 12-04 and 12-05.</human-check>
</verification>

<source_audit>
GOAL | Native media IO and hardware decode capability reporting must be binding-visible without JS-owned frame bytes | 12-02B maps capability reports into binding/schema contracts and guards source boundaries | COVERED
REQ | MEDIAIO-02 | Runtime capability report is available through bindings | COVERED
REQ | MEDIAIO-04 | No full-frame JS/Rust byte payload contract guards | COVERED
REQ | MEDIAIO-05 | Platform dependency legitimacy is approved before native implementation and fallback claims | COVERED
RESEARCH | Package Legitimacy Audit requires checkpoints before Cargo platform dependency additions | 12-02B blocking human verification | COVERED
CONTEXT | UI and bindings do not construct FFmpeg commands or own platform media APIs | Source guard action and tests | COVERED
</source_audit>

<success_criteria>
Binding/schema contracts expose Phase 12 capability and handle metadata without full-frame byte payloads, source guards protect renderer/binding boundaries, and platform dependencies are approved and resolved before native decoder plans execute.
</success_criteria>

<output>
Create `.planning/phases/12-media-io-hardware-decode-and-frame-texture-interop/12-02B-SUMMARY.md` when done.
</output>
