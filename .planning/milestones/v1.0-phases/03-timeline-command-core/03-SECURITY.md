---
phase: 03
slug: timeline-command-core
status: verified
threats_open: 0
asvs_level: 1
created: 2026-06-17
verified: 2026-06-17
register_authored_at_plan_time: true
---

# Phase 03 - Security

> Per-phase security contract: threat register, accepted risks, and audit trail for the Rust-owned timeline command core.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| Electron / binding -> Rust command model | Renderer and binding callers submit generated command envelopes; Rust deserializes and validates command/payload pairing before execution. | CommandEnvelope JSON, Draft, CommandState, TimelineSelection |
| Rust command model -> draft_commands | Pure semantic command crate owns timeline mutation semantics and returns updated Draft/state/events. | Typed command payloads and semantic Draft values |
| `.veproj/project.json` -> session command state | Persisted Draft remains canonical; undo/redo history is session-only and must not leak into project fixtures. | Draft JSON vs CommandState snapshots |
| draft_commands -> platform/runtime crates | Command semantics must stay independent from Electron, filesystem, project_store, media_runtime, preview, render graph, and FFmpeg. | No platform/runtime imports allowed |
| Generated contracts -> desktop TypeScript | Rust-generated schema and TypeScript contracts are consumed by Electron and checked for drift. | `schemas/*.json`, generated TypeScript declarations |

---

## Threat Register

| Threat ID | Category | Component | Disposition | Mitigation | Status |
|-----------|----------|-----------|-------------|------------|--------|
| T-03-01 | Tampering | Timeline command payloads and fixtures | mitigate | Strict `deny_unknown_fields` serde contracts, command/payload pairing, positive/negative classified fixtures, generated schema checks, and binding invalid-payload tests. Evidence: `CommandEnvelope` deserialize guard, `schema_fixtures_validate_command_contracts`, `execute_command_rejects_mismatched_command_payload_kind`, `pnpm run test:phase3-source-guards`. | closed |
| T-03-02 | Tampering/DoS | SourceTimerange, TargetTimerange, snapping, split/trim, text/audio timerange math | mitigate | Checked integer microsecond helpers reject overflow, zero duration, material-duration overrun, invalid split points, and source/target float seconds. Evidence: `validate_timeranges`, `validate_segment_material_bounds`, `timeline_edits`, `invalid_edits_are_atomic`, `snapping`, and Phase 3 source guards. | closed |
| T-03-03 | Tampering | Track mutation rules, locked tracks, material compatibility, platform leakage | mitigate | Commands validate locked tracks, material/track compatibility, no same-track overlap, and no runtime/platform imports in `draft_commands`. Evidence: `validate_timeline_rules`, `validate_track_unlocked`, `validate_track_material_rules`, `validate_track_overlaps`, `timeline_tracks`, `track_rules`, and `pnpm run test:phase3-source-guards`. | closed |
| T-03-04 | Repudiation/Tampering | Rejected command and undo/redo history mutation | mitigate | Commands use clone/validate/commit discipline and push undo snapshots only after successful validation; rejected edits do not mutate Draft, selection, undo, or redo state. Evidence: `push_undo_snapshot`, `clear_redo_after_commit`, `invalid_edits_are_atomic`, `undo_redo`, `text_commands`, and `audio_commands`. | closed |
| T-03-05 | Elevation of Privilege/Tampering | Renderer bypass of Rust timeline, text/audio, undo, snapping, or magnet semantics | mitigate | Binding routes generated command payloads to `draft_commands`; renderer source guards reject direct timeline semantic mutation/repair patterns. Evidence: `execute_timeline_command`, `bindings_node` timeline route smoke tests, and `pnpm run test:phase3-source-guards`. | closed |
| T-03-SC | Tampering | npm/pip/cargo dependency installs during Phase 3 | accept | No new package-manager installs were performed for Phase 3. Risk accepted as "no dependency change" and rechecked through committed task scope and lockfile stability. | closed |

*Status: open / closed*  
*Disposition: mitigate (implementation required) / accept (documented risk) / transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| R-03-SC | T-03-SC | Phase 3 added no npm/pip/cargo dependencies; no package legitimacy checkpoint was required. Future dependency additions must stop for human legitimacy verification. | GSD / Codex | 2026-06-17 |

---

## Verification Evidence

| Control | Evidence |
|---------|----------|
| Command payload strictness and generated drift | `cargo test -p draft_model schema_fixtures -- --nocapture`; `git diff --exit-code schemas apps/desktop-electron/src/generated` |
| Command behavior and atomicity | `pnpm run test:phase3-commands`; `cargo test -p draft_commands invalid_edits_are_atomic -- --nocapture` |
| Runtime/platform boundary guards | `pnpm run test:phase3-source-guards` |
| Binding delegation and invalid-payload handling | `cargo test -p bindings_node -- --nocapture`; `cargo test -p bindings_node timeline -- --nocapture` |
| Final build and regression gates | `PATH="$HOME/.cargo/bin:$PATH" just build`; `PATH="$HOME/.cargo/bin:$PATH" just test` |

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-06-17 | 6 | 6 | 0 | Codex |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-06-17
