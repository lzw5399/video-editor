---
phase: "12-media-io-hardware-decode-and-frame-texture-interop"
plan: "06B"
type: execute
wave: 7
depends_on:
  - "12-06"
files_modified:
  - "crates/draft_model/src/lib.rs"
  - "crates/draft_model/tests/schema_exports.rs"
  - "crates/bindings_node/src/lib.rs"
  - "crates/bindings_node/src/preview_export_service.rs"
  - "crates/bindings_node/tests/preview_commands.rs"
  - "schemas/command.schema.json"
  - "apps/desktop-electron/src/generated/CommandEnvelope.ts"
  - "apps/desktop-electron/src/generated/CommandResultEnvelope.ts"
autonomous: true
requirements:
  - MEDIAIO-02
  - MEDIAIO-03
  - MEDIAIO-04
  - MEDIAIO-05
user_setup: []
must_haves:
  truths:
    - "MEDIAIO-02: preview decode binding responses expose selected decode path, texture compatibility, and fallback diagnostics."
    - "MEDIAIO-03: preview frame handles can be explicitly released through session/generation-checked APIs."
    - "MEDIAIO-04: handle-capable preview decode responses contain opaque handles and metadata without full-frame 4K JS byte payloads."
    - "MEDIAIO-05: existing preview artifact command behavior remains available for fallback and compatibility."
  artifacts:
    - path: "crates/draft_model/src/lib.rs"
      provides: "PreviewDecodeRequest, DecodedPreviewFrameResponse, release command contracts"
    - path: "crates/bindings_node/src/preview_export_service.rs"
      provides: "handle-based preview decode request/response and release route"
    - path: "crates/bindings_node/tests/preview_commands.rs"
      provides: "preview decode, release, stale/wrong-session rejection, and artifact fallback tests"
    - path: "schemas/command.schema.json"
      provides: "generated schema for preview decode/release contracts"
  key_links:
    - from: "crates/bindings_node/src/preview_export_service.rs"
      to: "crates/realtime_preview_runtime/src/media_io_adapter.rs"
      via: "preview material decode request"
      pattern: "PreviewDecodeRequest"
    - from: "crates/bindings_node/src/preview_export_service.rs"
      to: "crates/draft_model/src/lib.rs"
      via: "generated handle-based response types"
      pattern: "DecodedPreviewFrameResponse"
---

<objective>
Add handle-based preview decode binding contracts and release APIs so Electron receives opaque frame/texture handles, metadata, and fallback diagnostics without full-frame decoded byte payloads.

Purpose: MEDIAIO-03 and MEDIAIO-04 require binding-visible retain/release behavior and handle-safe response contracts after the Phase 11 media IO adapter exists.
Output: Preview decode request/response contracts, release command contracts, generated schema/TypeScript outputs, and binding tests.
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
@.planning/phases/12-media-io-hardware-decode-and-frame-texture-interop/12-06-PLAN.md
@crates/draft_model/src/lib.rs
@crates/draft_model/tests/schema_exports.rs
@crates/bindings_node/src/lib.rs
@crates/bindings_node/src/preview_export_service.rs
@crates/bindings_node/tests/preview_commands.rs
</context>

## Artifacts this plan produces

- `PreviewDecodeRequest`
- `PreviewDecodeResponse`
- `DecodedPreviewFrameResponse`
- `PreviewFrameStorageKind`
- `PreviewDecodeDiagnostic`
- `releasePreviewFrame`
- generated preview decode/release schema and TypeScript contracts

<tasks>

<task type="auto" tdd="true">
  <name>Task 12-06B-01: Add handle-based preview decode binding contracts and release APIs</name>
  <files>crates/draft_model/src/lib.rs, crates/draft_model/tests/schema_exports.rs, crates/bindings_node/src/lib.rs, crates/bindings_node/src/preview_export_service.rs, crates/bindings_node/tests/preview_commands.rs, schemas/command.schema.json, apps/desktop-electron/src/generated/CommandEnvelope.ts, apps/desktop-electron/src/generated/CommandResultEnvelope.ts</files>
  <read_first>
    - `crates/draft_model/src/lib.rs`
    - `crates/draft_model/tests/schema_exports.rs`
    - `crates/bindings_node/src/lib.rs`
    - `crates/bindings_node/src/preview_export_service.rs`
    - `crates/bindings_node/tests/preview_commands.rs`
    - `.planning/phases/12-media-io-hardware-decode-and-frame-texture-interop/12-06-PLAN.md`
  </read_first>
  <behavior>
    - Test 1: binding request for a preview material decode returns `DecodedPreviewFrameResponse` with frame handle ID, optional texture handle ID, storage kind, source time, dimensions, pixel format, color metadata, selected path, compatibility state, and fallback diagnostics.
    - Test 2: the response schema rejects or omits full-frame byte arrays, `ArrayBuffer`, `Uint8Array`, native pointer strings, and platform object fields.
    - Test 3: `releasePreviewFrame` rejects unknown, wrong-session, and stale-generation handles, and accepts valid retained handles.
    - Test 4: existing `requestPreviewFrame` artifact command behavior remains available for fallback and compatibility.
  </behavior>
  <action>Per MEDIAIO-03 and MEDIAIO-04, add binding-safe request/response contracts for handle-based preview decode and explicit release. The binding layer may route commands and map Rust-owned results, but it must not construct FFmpeg args, import platform media APIs, map full native textures, expose native pointers, or copy full 4K frame bytes into JS. Include owner session, playback generation, `RuntimeDeviceId`, backend, compatibility state, selected decode path, and fallback diagnostics in handle-capable responses. Generate schema and TypeScript outputs from Rust contract types. Keep existing `requestPreviewFrame` artifact command behavior available for fallback and compatibility.</action>
  <acceptance_criteria>
    Binding tests prove handle responses, release behavior, stale/wrong-session rejection, existing preview artifact command availability, and generated contract drift cleanliness.
  </acceptance_criteria>
  <verify>
    <automated>cargo test -p bindings_node preview_decode -- --nocapture</automated>
    <automated>cargo test -p draft_model schema_exports -- --nocapture</automated>
    <automated>git diff --exit-code schemas apps/desktop-electron/src/generated</automated>
  </verify>
  <done>Task complete when handle-based preview decode/release contracts pass tests and generated artifacts are current.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| bindings -> renderer | Opaque frame/texture handles and diagnostics cross to JS without native pointers or full frame buffers. |
| renderer -> bindings | Release commands carry untrusted handle IDs, sessions, and generations. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-12-06B-01 | Spoofing | preview decode handles | mitigate | Validate session ownership and playback generation on retain/release and response use. |
| T-12-06B-02 | Information Disclosure | binding responses | mitigate | Generated contracts reject full-frame bytes, native pointer strings, and platform objects. |
| T-12-06B-03 | Tampering | generated contracts | mitigate | `schema_exports` and generated TypeScript drift checks keep JS contracts derived from Rust types. |
| T-12-06B-SC | Tampering | npm/pip/cargo installs | accept | This plan adds no new packages beyond those verified in 12-02B. |
</threat_model>

<verification>
<automated>cargo test -p bindings_node preview_decode -- --nocapture</automated>
<automated>cargo test -p draft_model schema_exports -- --nocapture</automated>
<automated>git diff --exit-code schemas apps/desktop-electron/src/generated</automated>
</verification>

<source_audit>
GOAL | Introduce handle-based preview decode contracts without JS-owned 4K frame copies | 12-06B adds binding/schema decode and release contracts | COVERED
REQ | MEDIAIO-02 | Binding responses include selected path, compatibility state, and fallback diagnostics | COVERED
REQ | MEDIAIO-03 | Release API validates session/generation ownership | COVERED
REQ | MEDIAIO-04 | Response schema omits full-frame byte arrays and native pointers | COVERED
REQ | MEDIAIO-05 | Existing preview artifact path remains available as fallback | COVERED
RESEARCH | Opaque handle boundary and no hidden full-frame JS/Rust copies | 12-06B contracts and tests | COVERED
CONTEXT | Binding layer routes commands but does not own FFmpeg construction or platform media APIs | 12-06B action and tests | COVERED
</source_audit>

<success_criteria>
Preview decode and release bindings are handle-based, generated contracts are current, stale/wrong-session handles are rejected, existing artifact fallback stays available, and no full-frame/native pointer payload crosses into JS.
</success_criteria>

<output>
Create `.planning/phases/12-media-io-hardware-decode-and-frame-texture-interop/12-06B-SUMMARY.md` when done.
</output>
