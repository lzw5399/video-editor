---
phase: "13-incremental-render-graph-dirty-ranges-and-cache-coherence"
plan: "05B"
type: execute
wave: 5
depends_on:
  - "13-02B"
  - "13-05"
files_modified:
  - "crates/bindings_node/src/preview_export_service.rs"
  - "crates/bindings_node/tests/preview_commands.rs"
  - "crates/bindings_node/tests/export_commands.rs"
  - "crates/draft_model/tests/schema_exports.rs"
  - "schemas/command.schema.json"
  - "apps/desktop-electron/src/generated/CommandEnvelope.ts"
  - "apps/desktop-electron/src/generated/CommandResultEnvelope.ts"
autonomous: true
requirements:
  - INCR-01
  - INCR-03
  - INCR-04
must_haves:
  truths:
    - "Bindings transport preview invalidation and export-prep dirty facts produced by Rust services per D-03."
    - "Generated contracts expose v2 dirty fields without renderer-owned graph/cache decisions per D-06."
    - "Contract generation remains drift-free after cache/invalidation v2 and CommandDelta transport changes."
  artifacts:
    - path: "crates/bindings_node/src/preview_export_service.rs"
      provides: "binding-safe preview/export invalidation transport"
    - path: "schemas/command.schema.json"
      provides: "generated schema for v2 dirty and invalidation contracts"
    - path: "apps/desktop-electron/src/generated/CommandResultEnvelope.ts"
      provides: "generated TypeScript transport for dirty facts"
  key_links:
    - from: "crates/bindings_node/src/preview_export_service.rs"
      to: "crates/preview_service/src/cache.rs"
      via: "PreviewInvalidationRequest v2 and export-prep dirty facts"
      pattern: "PreviewInvalidationRequest"
    - from: "schemas/command.schema.json"
      to: "apps/desktop-electron/src/generated/CommandResultEnvelope.ts"
      via: "contract generation"
      pattern: "dirtyRanges|changedDomains|changedGraphNodeIds"
---

<objective>
Expose Phase 13 preview/export dirty facts through binding-safe contracts and regenerated desktop TypeScript.

Purpose: Keep binding/schema/generated work separate from preview-service behavior while preserving the architecture rule that renderer code transports, but does not compute, dirty/cache decisions.
Output: Binding transport, schema assertions, generated command contracts, and binding tests for preview/export invalidation facts.
</objective>

<context>
@AGENTS.md
@.planning/ROADMAP.md
@.planning/REQUIREMENTS.md
@.planning/phases/13-incremental-render-graph-dirty-ranges-and-cache-coherence/13-CONTEXT.md
@.planning/phases/13-incremental-render-graph-dirty-ranges-and-cache-coherence/13-RESEARCH.md
@.planning/phases/13-incremental-render-graph-dirty-ranges-and-cache-coherence/13-DESIGN.md
@crates/draft_model/src/delta.rs
@crates/draft_model/tests/schema_exports.rs
@crates/preview_service/src/cache.rs
@crates/bindings_node/src/preview_export_service.rs
@crates/bindings_node/tests/preview_commands.rs
@crates/bindings_node/tests/export_commands.rs
@schemas/command.schema.json
@apps/desktop-electron/src/generated/CommandEnvelope.ts
@apps/desktop-electron/src/generated/CommandResultEnvelope.ts
</context>

## Artifacts this plan produces

- binding-safe preview invalidation request/result transport for v2 dirty facts
- binding-safe export-prep dirty fact transport using the same ranges/domains as preview invalidation
- schema export assertions for v2 invalidation fields
- regenerated command schema and TypeScript command result contracts
- tests proving bindings transport Rust-owned facts without renderer-side computation

<tasks>

<task type="auto" tdd="true">
  <name>Task 13-05B-01: Wire binding-safe preview/export dirty transport</name>
  <files>crates/bindings_node/src/preview_export_service.rs, crates/bindings_node/tests/preview_commands.rs, crates/bindings_node/tests/export_commands.rs</files>
  <read_first>
    - `crates/bindings_node/src/preview_export_service.rs`
    - `crates/bindings_node/tests/preview_commands.rs`
    - `crates/bindings_node/tests/export_commands.rs`
    - `crates/preview_service/src/cache.rs`
    - `crates/draft_model/src/delta.rs`
  </read_first>
  <action>Update binding preview invalidation and export-prep command paths so they transport `PreviewInvalidationRequest` v2, changed graph node IDs, dirty ranges, changed domains, runtime/output/profile fingerprint facts, and full-draft fallback data produced by Rust services per D-03. The binding layer may adapt serialized payloads and cache entry refs, but must not compute graph diffs, dirty ranges, cache keys, invalidation predicates, or FFmpeg commands per D-06. Add tests for preview and export commands proving dirty facts match the Rust service contract and unrelated renderer data cannot override Rust-owned decisions.</action>
  <acceptance_criteria>
    Binding tests prove preview invalidation and export prep receive v2 dirty facts from Rust-owned contracts, and renderer payloads do not compute or replace those facts.
  </acceptance_criteria>
  <verify>
    <automated>cargo test -p bindings_node preview_commands -- --nocapture</automated>
    <automated>cargo test -p bindings_node export_commands -- --nocapture</automated>
  </verify>
  <done>Bindings safely transport preview/export dirty facts without owning invalidation semantics.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 13-05B-02: Generate schema and TypeScript contracts for invalidation v2</name>
  <files>crates/draft_model/tests/schema_exports.rs, schemas/command.schema.json, apps/desktop-electron/src/generated/CommandEnvelope.ts, apps/desktop-electron/src/generated/CommandResultEnvelope.ts</files>
  <read_first>
    - `crates/draft_model/tests/schema_exports.rs`
    - `schemas/command.schema.json`
    - `apps/desktop-electron/src/generated/CommandEnvelope.ts`
    - `apps/desktop-electron/src/generated/CommandResultEnvelope.ts`
    - `package.json`
  </read_first>
  <action>Add schema/export assertions and regenerate schema plus TypeScript contracts so preview invalidation and export preparation expose v2 dirty fields: dirty ranges, changed material IDs, changed graph node IDs, changed domains, runtime capability fingerprint, full-draft flag, reason, schema version, and generator version per D-02 and D-03. Keep these contracts transport-only; do not add renderer helpers that derive cache keys, dirty ranges, graph node fingerprints, or invalidation predicates per D-06.</action>
  <acceptance_criteria>
    Generated contracts include v2 dirty fields, `CommandDelta` transport remains available, and contract drift is clean.
  </acceptance_criteria>
  <verify>
    <automated>cargo test -p draft_model schema_exports -- --nocapture</automated>
    <automated>pnpm run test:contracts</automated>
    <automated>rg -n "dirtyRanges|changedDomains|changedGraphNodeIds|runtimeCapabilityFingerprint|generatorVersion" schemas/command.schema.json apps/desktop-electron/src/generated/CommandResultEnvelope.ts</automated>
  </verify>
  <done>Preview invalidation and export-prep contracts are generated without renderer-owned coherence logic.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| preview/export service -> binding bridge | Binding transports Rust-owned invalidation facts across the Node-API boundary. |
| generated TypeScript -> renderer | Renderer sees dirty facts but must not compute graph/cache decisions. |
| binding export prep -> FFmpeg compiler | Export prep transports dirty facts while FFmpeg compiler remains a pure job compiler. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-13-15B-01 | Tampering | binding invalidation payloads | mitigate | Tests ensure Rust-owned v2 dirty facts are transported and renderer data cannot replace invalidation decisions. |
| T-13-15B-02 | Tampering | generated TypeScript contracts | mitigate | Contract generation exposes fields as transport data while source guards reject renderer-owned cache/dirty logic. |
| T-13-15B-03 | Repudiation | contract drift | mitigate | `cargo test -p draft_model schema_exports` and `pnpm run test:contracts` prove schema/TypeScript drift is clean. |
| T-13-SC | Tampering | npm/pip/cargo installs | accept | No package installation is required. |
</threat_model>

<verification>
<automated>cargo test -p bindings_node preview_commands -- --nocapture</automated>
<automated>cargo test -p bindings_node export_commands -- --nocapture</automated>
<automated>cargo test -p draft_model schema_exports -- --nocapture</automated>
<automated>pnpm run test:contracts</automated>
</verification>

<source_audit>
REQ | INCR-01 | Graph node identity/fingerprint facts are transported for cache coherence | 13-05B | COVERED
REQ | INCR-03 | Dirty propagation reaches preview/export binding contracts | 13-05B | COVERED
REQ | INCR-04 | Undo/redo invalidation facts remain transportable through bindings | 13-05B | COVERED
CONTEXT | D-02 | Node identity separate from fingerprints in generated transport | 13-05B | COVERED
CONTEXT | D-03 | Dirty propagation across required consumers is binding-safe | 13-05B | COVERED
CONTEXT | D-05 | No SQLite artifact store or scheduler | 13-05B | COVERED
CONTEXT | D-06 | UI/renderer transports but does not compute dirty/cache/FFmpeg decisions | 13-05B | COVERED
CONTEXT | D-07 | Dirty/cache ranges use integer microseconds in generated contracts | 13-05B | COVERED
RESEARCH | Preview cache key v2 and invalidation request v2 are generated/transported | 13-05B | COVERED
</source_audit>

<success_criteria>
Plan 13-05B is complete when binding tests, schema export tests, and contract drift checks prove preview/export dirty facts are transported without renderer-owned invalidation logic.
</success_criteria>

<output>
Create `.planning/phases/13-incremental-render-graph-dirty-ranges-and-cache-coherence/13-05B-SUMMARY.md` when done.
</output>
