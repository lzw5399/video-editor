---
phase: "13 - incremental-render-graph-dirty-ranges-and-cache-coherence"
status: verified
threats_open: 0
threats_closed: 25
audit_timestamp: "2026-06-19T02:49:11Z"
asvs_level: 1
block_on: open
threat_count_basis: unique_threat_ids
---

# Phase 13 - Security

Per-phase security contract for Phase 13 plan-time threat mitigations.

## Audit Scope

- PLAN files audited: `13-01-PLAN.md`, `13-02-PLAN.md`, `13-02B-PLAN.md`, `13-03-PLAN.md`, `13-04-PLAN.md`, `13-05-PLAN.md`, `13-05B-PLAN.md`, `13-06-PLAN.md`.
- Summary threat flags: no unregistered threat flags found. `13-01`, `13-03`, `13-04`, `13-05`, `13-05B`, and `13-06` explicitly report none; `13-02` and `13-02B` have no `## Threat Flags` section.
- Count basis: repeated `T-13-SC` accept rows across PLAN files are consolidated into one unique accepted-risk entry.
- Implementation files were used as read-only evidence. No implementation files were modified.

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date | Status |
|---------|------------|-----------|-------------|------|--------|
| AR-13-01 | T-13-SC | All Phase 13 PLAN files declare no package installation required; package/script changes are covered by existing lockfile and phase gates. | Plan disposition | 2026-06-19 | accepted |
| AR-13-02 | T-13-09 | Material IDs and dirty ranges are already command contract data; Phase 13 does not introduce a new secret-bearing surface. | Plan disposition | 2026-06-19 | accepted |

## Threat Verification

| Threat ID | Category | Component | Disposition | Status | Evidence |
|-----------|----------|-----------|-------------|--------|----------|
| T-13-01 | Tampering | `scripts/phase13-source-guards.sh` | mitigate | closed | Guard rejects renderer dirty/cache/render graph/FFmpeg/float-time/derived-artifact patterns at `scripts/phase13-source-guards.sh:89`; negative self-checks at `scripts/phase13-source-guards.sh:95`; final renderer/source rejects at `scripts/phase13-source-guards.sh:141`; package wiring required at `scripts/phase13-source-guards.sh:112`. |
| T-13-02 | DoS | large-timeline fixtures | mitigate | closed | Fixture cap `MAX_SEGMENTS_PER_TRACK` at `crates/testkit/src/large_timeline.rs:10`; config rejection at `crates/testkit/src/large_timeline.rs:189`; deterministic/bounds tests at `crates/testkit/tests/large_timeline_incremental.rs:18` and `crates/testkit/tests/large_timeline_incremental.rs:82`; no-runtime-dependency fixture test at `crates/testkit/tests/large_timeline_incremental.rs:63`. |
| T-13-03 | Tampering | contract/schema gates | mitigate | closed | Generated contract/draft schema anchors asserted at `crates/draft_model/tests/schema_exports.rs:431`; forbidden derived draft fields asserted absent at `crates/draft_model/tests/schema_exports.rs:461`; source guard requires Phase 13 generated symbols at `scripts/phase13-source-guards.sh:127` and rejects derived draft metadata at `scripts/phase13-source-guards.sh:160`. |
| T-13-SC | Tampering | npm/pip/cargo installs | accept | closed | Documented in Accepted Risks Log as `AR-13-01`. |
| T-13-04 | Tampering | `CommandDelta.changed_ranges` | mitigate | closed | `TargetTimerange` checked integer helpers at `crates/draft_model/src/timeline.rs:59`; move delta includes previous and current ranges at `crates/draft_commands/src/delta.rs:223`; move/trim tests assert old+current ranges at `crates/draft_commands/tests/command_delta.rs:55` and `crates/draft_commands/tests/command_delta.rs:136`. |
| T-13-05 | DoS | range merge helpers | mitigate | closed | `merge_dirty_ranges` sorts deterministically and fails on checked-end/union overflow at `crates/preview_service/src/cache.rs:589`; `PreviewInvalidationRequest::normalize` converts `None` into `dirty_ranges.clear()`, `full_draft = true`, and `PreviewCache` domain retention at `crates/preview_service/src/cache.rs:276`; regression test `invalidation_range_merge_overflow_falls_back_to_full_draft` asserts full-draft fallback, empty targeted ranges, and all entries invalidated at `crates/preview_service/tests/cache_invalidation.rs:170`. Focused verification passed: `cargo test -p preview_service --test cache_invalidation invalidation_range_merge_overflow_falls_back_to_full_draft -- --nocapture`. |
| T-13-06 | Repudiation | response contract migration | mitigate | closed | `CommandDelta` requires `command` and `reason` at `crates/draft_model/src/delta.rs:13`; `TimelineCommandResponse.delta` is required by schema at `crates/draft_model/tests/schema_exports.rs:499`; simple command delta tests begin at `crates/draft_commands/tests/command_delta.rs:25`; accepted response carries delta command at `crates/draft_commands/tests/command_delta.rs:213`. |
| T-13-06B-01 | Tampering | generated command schema | mitigate | closed | Schema export test requires `ChangedEntity`, `DirtyDomain`, `DirtyRange`, `InvalidationScope`, `CommandDelta`, and `TimelineCommandResponse` at `crates/draft_model/tests/schema_exports.rs:475`; generated TS exposes `CommandDelta` and required `delta` at `apps/desktop-electron/src/generated/CommandResultEnvelope.ts:59`. |
| T-13-06B-02 | Repudiation | contract drift | mitigate | closed | `test:contracts` is wired to `git diff --exit-code schemas apps/desktop-electron/src/generated` at `package.json:72`; Phase 13 guard repeats the generated-contract drift check at `scripts/phase13-source-guards.sh:173`. |
| T-13-06B-03 | Tampering | renderer transport | mitigate | closed | Generated response contracts expose Rust-owned delta facts at `apps/desktop-electron/src/generated/CommandResultEnvelope.ts:59`; desktop helper forwards payload fields without building dirty logic at `apps/desktop-electron/src/renderer/commandHelpers.ts:501`; renderer dirty/cache decision guard at `scripts/phase13-source-guards.sh:141`. |
| T-13-07 | Tampering | command-specific delta emission | mitigate | closed | Domain-specific delta builders cover segment/text/audio/visual/canvas/material/history at `crates/draft_commands/src/delta.rs:9`; mutating command tests cover simple timeline at `crates/draft_commands/tests/command_delta.rs:25`, text/audio at `crates/draft_commands/tests/command_delta.rs:238`, visual/keyframe at `crates/draft_commands/tests/command_delta.rs:430`, canvas at `crates/draft_commands/tests/command_delta.rs:507`, and material dependency at `crates/draft_commands/tests/command_delta.rs:566`. |
| T-13-08 | Tampering | undo/redo cache coherence | mitigate | closed | Undo/redo restore snapshots and emit `restored_draft_delta` at `crates/draft_commands/src/history.rs:72` and `crates/draft_commands/src/history.rs:116`; restored delta full-draft fallback exists at `crates/draft_commands/src/delta.rs:572`; undo/redo dirty range tests at `crates/draft_commands/tests/command_delta.rs:649`. |
| T-13-09 | Information Disclosure | material dependency expansion | accept | closed | Documented in Accepted Risks Log as `AR-13-02`; material dependency dirty facts remain material IDs/ranges at `crates/draft_commands/src/delta.rs:455` and `crates/draft_commands/tests/command_delta.rs:566`. |
| T-13-10 | Tampering | `RenderGraphNodeId` | mitigate | closed | Stable node keys are semantic role/draft/track/segment/material/local anchors at `crates/render_graph/src/incremental.rs:99`; tests assert stable IDs survive content/timing/material metadata changes at `crates/render_graph/tests/node_identity.rs:39`. |
| T-13-11 | Tampering | `RenderGraphNodeFingerprint` | mitigate | closed | Fingerprints are separate from node IDs and include semantic/input/output/runtime/schema/generator fields at `crates/render_graph/src/fingerprint.rs:15`; `fingerprint_parts` fills those fields at `crates/render_graph/src/fingerprint.rs:396`; tests assert semantic/input/output/runtime fingerprint changes without ID change at `crates/render_graph/tests/node_identity.rs:64`. |
| T-13-12 | DoS | graph diff | mitigate | closed | Diff compares BTreeMap/BTreeSet keyed by stable node key at `crates/render_graph/src/incremental.rs:254`; deterministic add/remove/change/unchanged tests at `crates/render_graph/tests/node_identity.rs:134` and `crates/render_graph/tests/node_identity.rs:179`; large localized diff bounds at `crates/testkit/tests/large_timeline_incremental.rs:94`. |
| T-13-13 | Tampering | `PreviewCacheKey` v2 | mitigate | closed | Cache key v2 derives graph node keys and fingerprints from `RenderGraphNodeFingerprint` at `crates/preview_service/src/cache.rs:32`; preview service builds snapshot-derived keys at `crates/preview_service/src/service.rs:223`; renderer cache/dirty decision guard at `scripts/phase13-source-guards.sh:141`. |
| T-13-14 | Tampering | invalidation predicate | mitigate | closed | Invalidation predicate covers full-draft/range/material/graph-node/runtime/profile at `crates/preview_service/src/cache.rs:504`; tests explicitly cover range/material/graph/runtime/profile/full-draft at `crates/preview_service/tests/cache_invalidation.rs:299`; domain-specific range invalidation test at `crates/preview_service/tests/dirty_propagation.rs:170`. |
| T-13-15 | DoS | export prep invalidation | mitigate | closed | Export prep remains classification data in `ExportPrepDirtyFacts` at `crates/preview_service/src/cache.rs:289`; conversion mirrors preview invalidation at `crates/preview_service/src/cache.rs:302`; source guard rejects Phase 14/16 artifact store or scheduler scope at `scripts/phase13-source-guards.sh:165`; binding stores dirty facts without scheduling work at `crates/bindings_node/src/preview_export_service.rs:759`. |
| T-13-15B-01 | Tampering | binding invalidation payloads | mitigate | closed | Binding constructs `PreviewInvalidationRequest` from payload at `crates/bindings_node/src/preview_export_service.rs:148`; export dirty facts are carried into status at `crates/bindings_node/src/preview_export_service.rs:514`; binding tests cover preview v2 facts at `crates/bindings_node/tests/preview_commands.rs:156` and export dirty facts at `crates/bindings_node/tests/export_commands.rs:59`. |
| T-13-15B-02 | Tampering | generated TypeScript contracts | mitigate | closed | Schema export test requires dirty fact contracts and TS fields at `crates/draft_model/tests/schema_exports.rs:535`; generated command contracts expose `DirtyRange`, preview invalidation fields, and `ExportPrepDirtyFacts` at `apps/desktop-electron/src/generated/CommandEnvelope.ts:37`; source guard rejects renderer-owned dirty/cache logic at `scripts/phase13-source-guards.sh:141`. |
| T-13-15B-03 | Repudiation | contract drift | mitigate | closed | Schema export tests include Phase 13 dirty fact contracts at `crates/draft_model/tests/schema_exports.rs:535`; `test:contracts` is wired at `package.json:72`; guard enforces generated diff clean at `scripts/phase13-source-guards.sh:173`. |
| T-13-16 | DoS | large-timeline gates | mitigate | closed | Fixture counts are bounded at `crates/testkit/src/large_timeline.rs:10`; structural validity/determinism tests at `crates/testkit/tests/large_timeline_incremental.rs:18`; bounded localized diff assertions use counts, not timings, at `crates/testkit/tests/large_timeline_incremental.rs:156` and `crates/testkit/tests/large_timeline_incremental.rs:389`. |
| T-13-17 | Tampering | renderer source | mitigate | closed | Final source guard rejects renderer-owned graph/dirty/cache decisions at `scripts/phase13-source-guards.sh:141` and FFmpeg command construction at `scripts/phase13-source-guards.sh:148`; desktop helper only forwards dirty fact fields at `apps/desktop-electron/src/renderer/commandHelpers.ts:501`. |
| T-13-18 | Repudiation | final contract drift | mitigate | closed | `test:phase13` includes `pnpm run test:contracts` at `package.json:67`; guard requires Phase 13 package scripts at `scripts/phase13-source-guards.sh:112` and runs `git diff --exit-code schemas apps/desktop-electron/src/generated` at `scripts/phase13-source-guards.sh:173`. |

## Open Threats

None. All 25 unique plan-time threats are closed or documented as accepted risks.

## Unregistered Flags

None.

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-06-19T02:40:57Z | 25 | 24 | 1 | Codex |
| 2026-06-19T02:49:11Z | 25 | 25 | 0 | Codex |

## Sign-Off

- [x] All plan-time threats have a disposition.
- [x] Accepted risks are documented in Accepted Risks Log.
- [x] `threats_open: 0` confirmed.
- [x] `status: verified` set in frontmatter.

Approval: verified. Phase 13 security gate passes with no open threats.
