---
phase: 03
slug: timeline-command-core
status: draft
nyquist_compliant: true
wave_0_complete: false
created: 2026-06-17
---

# Phase 03 - Validation Strategy

Per-phase validation contract for Rust-owned timeline commands, atomic invalid-edit rejection, undo/redo, snapping/MainTrackMagnet, text semantics, and audio semantics.

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust `cargo test`, generated JSON Schema/TypeScript drift checks, and binding smoke tests where command routes are exposed |
| **Config file** | `Cargo.toml`, `crates/draft_commands/Cargo.toml`, `crates/draft_model/Cargo.toml`, `package.json`, `justfile` |
| **Quick run command** | `cargo test -p draft_commands -- --nocapture` |
| **Full suite command** | `PATH="$HOME/.cargo/bin:$PATH" just test` if `just` is installed; otherwise `pnpm run test` as the local fallback noted by research |
| **Estimated runtime** | ~90-240 seconds after Phase 3 command and contract tests are added |

## Sampling Rate

- **After every task commit:** Run the narrowest affected Rust test target, plus `cargo test -p draft_commands -- --nocapture` once command modules exist.
- **After every plan wave:** Run `pnpm run test:rust && pnpm run test:contracts`; include `cargo test -p bindings_node -- --nocapture` after binding routes are added.
- **Before `$gsd-verify-work`:** Run `PATH="$HOME/.cargo/bin:$PATH" just build`, `PATH="$HOME/.cargo/bin:$PATH" just test`, and `git diff --exit-code schemas apps/desktop-electron/src/generated`. If `just` is unavailable locally, run `pnpm run build` and `pnpm run test`, then record the local `just` absence in verification.
- **Max feedback latency:** 240 seconds for the full Phase 3 suite on the local machine.

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 03-01-W0 | 03-01 | 1 | TIME-01, TIME-07 | T-03-01 / T-03-03 | Root timeline supports video/audio/text tracks; locked tracks reject mutation; same-track overlap is rejected | unit | `cargo test -p draft_commands timeline_tracks -- --nocapture` | no W0 | pending |
| 03-01-W1 | 03-01 | 1 | TIME-02, TIME-06 | T-03-02 / T-03-04 | Timerange helpers use checked integer microsecond math and reject source/target overflow or invalid material bounds | unit | `cargo test -p draft_commands timerange_rules -- --nocapture` | no W0 | pending |
| 03-02-W0 | 03-02 | 2 | TIME-02, TIME-03, TIME-06 | T-03-01 / T-03-04 | Add, move, split, trim, delete, and select commands mutate only validated clone state and leave original draft unchanged on failure | unit | `cargo test -p draft_commands timeline_edits -- --nocapture` | no W0 | pending |
| 03-02-W1 | 03-02 | 2 | TIME-02, TIME-03 | T-03-05 | Generated command payloads/responses expose Rust-owned timeline commands through the standard envelope | contract/binding | `cargo test -p draft_model schema -- --nocapture && cargo test -p bindings_node timeline -- --nocapture` | partial existing | pending |
| 03-03-W0 | 03-03 | 3 | TIME-04, TIME-06 | T-03-04 | Undo/redo history is bounded, session-only, and updated only after committed edits | unit | `cargo test -p draft_commands undo_redo -- --nocapture` | no W0 | pending |
| 03-03-W1 | 03-03 | 3 | TIME-05 | T-03-02 / T-03-05 | Snapping and MainTrackMagnet are computed in Rust and emit deterministic command events | unit | `cargo test -p draft_commands snapping -- --nocapture` | no W0 | pending |
| 03-04-W0 | 03-04 | 4 | TEXT-01, TEXT-02 | T-03-01 / T-03-04 | Text segments persist editable text content and MVP style values without hiding text in a URI string | unit/contract | `cargo test -p draft_commands text_commands -- --nocapture && cargo test -p draft_model schema -- --nocapture` | no W0 | pending |
| 03-04-W1 | 03-04 | 4 | AUD-01, AUD-02 | T-03-01 / T-03-03 | Audio commands reject incompatible tracks/materials and update segment volume plus track mute in Rust | unit | `cargo test -p draft_commands audio_commands -- --nocapture` | no W0 | pending |
| 03-05-W0 | 03-05 | 5 | TEST-02 | T-03-01 / T-03-02 / T-03-04 / T-03-05 | Required command coverage, fixture classification, source guards, and generated contract drift gates pass before phase verification | full gate | `pnpm run test:rust && pnpm run test:contracts && pnpm run test:bindings` | partial existing | pending |

## Threat References

| Ref | Threat | Required Mitigation |
|-----|--------|---------------------|
| T-03-01 | Malformed command payload changes draft unexpectedly | Strict Rust command payloads, `deny_unknown_fields`, command/payload matching, and binding routes through generated `CommandEnvelope` only |
| T-03-02 | Overflowed source or target timeranges bypass validation | Centralized checked microsecond arithmetic and explicit invalid timerange errors |
| T-03-03 | Locked track or incompatible track/material edit mutates protected draft state | Command-level locked-track, track-kind, material-kind, and overlap checks before commit |
| T-03-04 | Rejected command mutates draft or undo history | Clone/patch/validate/commit transaction with history push only after successful commit |
| T-03-05 | Renderer bypasses Rust-owned timeline semantics | Electron receives generated command contracts and updated draft/state/events; renderer source must not mutate `Draft.tracks` directly |

## Wave 0 Requirements

- [ ] Add `draft_model = { path = "../draft_model" }` to `crates/draft_commands/Cargo.toml`.
- [ ] Add command modules and tests under `crates/draft_commands/src/` for timeranges, track rules, timeline edits, snapping, history, text, and audio.
- [ ] Extend `crates/draft_model/src/lib.rs`, `timeline.rs`, and schema export tests with timeline command payloads/responses, command state, selection state, text style/content, and segment volume types.
- [ ] Extend `crates/bindings_node/src/lib.rs` or a binding command-service module with timeline command routes after pure command tests exist.
- [ ] Extend source guards to prove `draft_commands` has no platform/runtime imports and no semantic float seconds are introduced in Rust contracts, schemas, or generated TypeScript.

## Manual-Only Verifications

All Phase 3 behaviors have automated verification. Rich drag UI, visual timeline checks, preview parity, waveform caches, render graph, and export are deferred to later phases.

## Validation Sign-Off

- [x] All tasks have automated verify targets or Wave 0 dependencies.
- [x] Sampling continuity: no 3 consecutive tasks without automated verify.
- [x] Wave 0 covers all missing test references.
- [x] No watch-mode flags.
- [x] Feedback latency target is below 240 seconds.
- [x] `nyquist_compliant: true` set in frontmatter.

**Approval:** pending execution
