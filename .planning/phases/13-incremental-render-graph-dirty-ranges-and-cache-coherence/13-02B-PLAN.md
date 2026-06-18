---
phase: "13-incremental-render-graph-dirty-ranges-and-cache-coherence"
plan: "02B"
type: execute
wave: 3
depends_on:
  - "13-02"
files_modified:
  - "crates/draft_model/tests/schema_exports.rs"
  - "schemas/command.schema.json"
  - "apps/desktop-electron/src/generated/CommandEnvelope.ts"
  - "apps/desktop-electron/src/generated/CommandResultEnvelope.ts"
autonomous: true
requirements:
  - INCR-02
  - INCR-03
must_haves:
  truths:
    - "Generated schema and TypeScript contracts expose `TimelineCommandResponse.delta` per D-01."
    - "Generated contracts include `CommandDelta`, `ChangedEntity`, `DirtyDomain`, `DirtyRange`, `DirtyRangeSource`, and `InvalidationScope`."
    - "Renderer code receives Rust-owned delta facts as transport data and does not construct dirty/cache decisions per D-06."
  artifacts:
    - path: "crates/draft_model/tests/schema_exports.rs"
      provides: "schema/export assertions for Phase 13 delta contracts"
    - path: "schemas/command.schema.json"
      provides: "generated command response schema with delta contract"
    - path: "apps/desktop-electron/src/generated/CommandResultEnvelope.ts"
      provides: "generated TypeScript response contract with CommandDelta"
  key_links:
    - from: "crates/draft_model/tests/schema_exports.rs"
      to: "schemas/command.schema.json"
      via: "schema export test"
      pattern: "CommandDelta"
    - from: "schemas/command.schema.json"
      to: "apps/desktop-electron/src/generated/CommandResultEnvelope.ts"
      via: "contract generation"
      pattern: "delta.*CommandDelta"
---

<objective>
Generate and verify the schema and TypeScript contract surface for Phase 13 command deltas.

Purpose: Keep generated contract work separate from behavioral crate changes while proving desktop transports the Rust-owned `CommandDelta` response shape.
Output: Updated command schema, generated TypeScript command result contract, and export assertions for the delta types.
</objective>

<context>
@AGENTS.md
@.planning/ROADMAP.md
@.planning/REQUIREMENTS.md
@.planning/phases/13-incremental-render-graph-dirty-ranges-and-cache-coherence/13-CONTEXT.md
@.planning/phases/13-incremental-render-graph-dirty-ranges-and-cache-coherence/13-RESEARCH.md
@.planning/phases/13-incremental-render-graph-dirty-ranges-and-cache-coherence/13-DESIGN.md
@crates/draft_model/src/lib.rs
@crates/draft_model/src/delta.rs
@crates/draft_model/tests/schema_exports.rs
@schemas/command.schema.json
@apps/desktop-electron/src/generated/CommandEnvelope.ts
@apps/desktop-electron/src/generated/CommandResultEnvelope.ts
</context>

## Artifacts this plan produces

- schema export assertions for `CommandDelta`, `ChangedEntity`, `DirtyDomain`, `DirtyRange`, `DirtyRangeSource`, and `InvalidationScope`
- `schemas/command.schema.json` containing `TimelineCommandResponse.delta`
- generated TypeScript command result contract containing `delta: CommandDelta`
- generated TypeScript command envelope compatibility retained without renderer-owned delta construction

<tasks>

<task type="auto" tdd="true">
  <name>Task 13-02B-01: Add schema export assertions for delta contracts</name>
  <files>crates/draft_model/tests/schema_exports.rs</files>
  <read_first>
    - `crates/draft_model/tests/schema_exports.rs`
    - `crates/draft_model/src/lib.rs`
    - `crates/draft_model/src/delta.rs`
    - `.planning/phases/13-incremental-render-graph-dirty-ranges-and-cache-coherence/13-RESEARCH.md`
  </read_first>
  <action>Extend schema/export tests so the generated command response schema must include direct `TimelineCommandResponse.delta` and all Phase 13 delta types per D-01. Treat serde/default or optional response migration only as an implementation bridge; the assertions must describe the completed Phase 13 contract where accepted command responses expose delta data. Do not add renderer-side construction helpers or product behavior in this plan per D-06.</action>
  <acceptance_criteria>
    Schema export tests fail if `delta`, `CommandDelta`, `ChangedEntity`, `DirtyDomain`, `DirtyRange`, `DirtyRangeSource`, or `InvalidationScope` are missing from generated contracts.
  </acceptance_criteria>
  <verify>
    <automated>cargo test -p draft_model schema_exports -- --nocapture</automated>
  </verify>
  <done>Delta contract export assertions exist and describe the final direct response contract.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 13-02B-02: Regenerate command schema and TypeScript response contracts</name>
  <files>schemas/command.schema.json, apps/desktop-electron/src/generated/CommandEnvelope.ts, apps/desktop-electron/src/generated/CommandResultEnvelope.ts</files>
  <read_first>
    - `schemas/command.schema.json`
    - `apps/desktop-electron/src/generated/CommandEnvelope.ts`
    - `apps/desktop-electron/src/generated/CommandResultEnvelope.ts`
    - `package.json`
  </read_first>
  <action>Regenerate command schema and generated TypeScript contracts after Plan 13-02 adds the direct response delta field. Keep the generated desktop surface as transport-only data: no graph diffs, dirty range derivation, cache key derivation, preview invalidation decisions, or FFmpeg commands belong in renderer code per D-06. Preserve existing command envelope compatibility while adding response delta exports.</action>
  <acceptance_criteria>
    Generated contracts include the Phase 13 delta surface, contract drift checks pass, and no renderer-owned dirty/cache/render graph logic is introduced.
  </acceptance_criteria>
  <verify>
    <automated>pnpm run test:contracts</automated>
    <automated>rg -n "CommandDelta|DirtyDomain|DirtyRange|InvalidationScope" schemas/command.schema.json apps/desktop-electron/src/generated/CommandResultEnvelope.ts</automated>
  </verify>
  <done>Command response schema and generated TypeScript contracts expose the Phase 13 delta surface.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| Rust schema export -> generated TypeScript | Generated contracts must match Rust-owned command response semantics. |
| generated TypeScript -> renderer | Renderer may transport delta facts but must not compute dirty/cache decisions. |

## STRIDE Threat Register

| Threat ID | Category | Component | Disposition | Mitigation Plan |
|-----------|----------|-----------|-------------|-----------------|
| T-13-06B-01 | Tampering | generated command schema | mitigate | Schema export tests require direct delta response field and all delta types. |
| T-13-06B-02 | Repudiation | contract drift | mitigate | `pnpm run test:contracts` proves generated schema and TypeScript are synchronized. |
| T-13-06B-03 | Tampering | renderer transport | mitigate | Generated contracts carry Rust-owned facts only; source guards in Plans 13-01 and 13-06 reject renderer-owned dirty/cache logic. |
| T-13-SC | Tampering | npm/pip/cargo installs | accept | No package installation is required. |
</threat_model>

<verification>
<automated>cargo test -p draft_model schema_exports -- --nocapture</automated>
<automated>pnpm run test:contracts</automated>
<automated>rg -n "CommandDelta|DirtyDomain|DirtyRange|InvalidationScope" schemas/command.schema.json apps/desktop-electron/src/generated/CommandResultEnvelope.ts</automated>
</verification>

<source_audit>
REQ | INCR-02 | Accepted command response delta contract is generated | 13-02B | COVERED
REQ | INCR-03 | Dirty domain and range contracts are generated | 13-02B | COVERED
CONTEXT | D-01 | CommandDelta lives directly on TimelineCommandResponse | 13-02, 13-02B | COVERED
CONTEXT | D-06 | Renderer transports but does not compute dirty/cache decisions | 13-02B | COVERED
CONTEXT | D-07 | Generated contract time values remain integer/rational | 13-02B | COVERED
RESEARCH | Direct response delta field is final Phase 13 contract | 13-02B | COVERED
</source_audit>

<success_criteria>
Plan 13-02B is complete when schema export tests, command schema, and generated TypeScript contracts expose the direct command delta response surface and contract drift checks pass.
</success_criteria>

<output>
Create `.planning/phases/13-incremental-render-graph-dirty-ranges-and-cache-coherence/13-02B-SUMMARY.md` when done.
</output>
