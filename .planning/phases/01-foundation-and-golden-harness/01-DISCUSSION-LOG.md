# Phase 1: Foundation And Golden Harness - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md - this log preserves the alternatives considered.

**Date:** 2026-06-17
**Phase:** 1-Foundation And Golden Harness
**Areas discussed:** Engineering scaffold and one-command workflow, Electron Rust binding boundary, FFmpeg and ffprobe discovery strategy, Golden fixture and test gate scope, Cross-platform abstraction boundaries

---

## Engineering Scaffold And One-Command Workflow

| Question | Options Considered | Selected |
|----------|--------------------|----------|
| Unified command standard | `pnpm dev/build/test` + `cargo test`; `just dev/build/test`; `make dev/build/test` | `just dev/build/test` |
| Initial scaffold breadth | Minimal runnable scaffold; full target layered structure; Phase 1-2 only scaffold | Full target layered structure |
| Node workspace tooling | `pnpm workspace` + Corepack; `npm workspaces`; `bun` | `pnpm workspace` + Corepack |
| Toolchain pinning | Pin key versions; pin only Rust stable + pnpm lockfile; do not pin yet | Pin key versions |

**Notes:** User wants the project goal, roadmap, and standards established early because this is a new, layered project expected to continue over time.

---

## Electron Rust Binding Boundary

| Question | Options Considered | Selected |
|----------|--------------------|----------|
| Phase 1 binding scope | Only `ping/version`; `ping/version` + typed command envelope; draft/material stub commands | `ping/version` + typed command envelope |
| Terminology in binding/API/tests | English Jianying concept names; pinyin/Chinese identifiers; mixed schema/UI/IPC vocabulary | English Jianying concept names |
| Contract source | Rust serde types exported to JSON Schema / TS; TypeScript types as source; handwritten types on both sides | Rust serde types exported to JSON Schema / TS |
| Result shape | Standardized `ok/error/events`; free per-function returns; standardized errors only | Standardized `ok/error/events` |

**Notes:** User emphasized internal and external terminology should be consistent with Jianying concepts, not just UI-facing.

---

## FFmpeg And ffprobe Discovery Strategy

| Question | Options Considered | Selected |
|----------|--------------------|----------|
| Discovery scope | Only system `PATH`; `PATH` + explicit paths + version probe; app-bundled FFmpeg management | `PATH` + explicit paths + version probe |
| Explicit config source | Environment variables first; project config file only; UI settings page only | Environment variables first |
| Error shape | String error; structured error + remediation + detected paths; automatic download/install | Structured error + remediation + detected paths |
| License posture | README reminder; runtime doc + license manifest placeholder; choose/download build; do not handle in Phase 1 | Do not handle in Phase 1 |

**Notes:** User said this is an open source project and does not want Phase 1 to spend effort on FFmpeg license handling. Boundary captured: no bundling/downloading/redistribution in Phase 1.

---

## Golden Fixture And Test Gate Scope

| Question | Options Considered | Selected |
|----------|--------------------|----------|
| Fixture/golden coverage | Directories + schema fixture only; schema fixture + tiny media + render smoke; full draft golden | Schema fixture + tiny media + render smoke |
| Tiny media source | Commit tiny binary media; generate with FFmpeg `lavfi`; generate and cache under `fixtures/generated` | Generate with FFmpeg `lavfi` |
| CI without FFmpeg | Fail directly; skip render smoke; split `just test` / `just test-full` | Fail directly |
| Render smoke assertions | Output file exists; output exists + ffprobe metadata; first-frame pixel hash | Output exists + ffprobe metadata |

**Notes:** User wants every step to be testable. Phase 1 render smoke is a required gate, but pixel/hash comparison waits until later.

---

## Cross-Platform Abstraction Boundaries

| Question | Options Considered | Selected |
|----------|--------------------|----------|
| Platform boundary model | Pure semantic core does not depend on platform traits; Engine Core depends on `PlatformSystem`; docs only | Pure semantic core does not depend on platform traits |
| Phase 1 interfaces | Only `FfmpegExecutor`; `FfmpegExecutor` + `PlatformFileSystem` + `PreviewRenderer`; full `PlatformSystem` | `FfmpegExecutor` + `PlatformFileSystem` + `PreviewRenderer` |
| Trait placement | Generic `platform` crate; consuming crate boundaries; TS/Rust duplicate abstractions | Consuming crate boundaries |
| Hardware encoder timing | Document only; add unused trait; probe hardware encoders now | Document only |

**Notes:** User supplied a layered cross-platform abstraction sketch. Discussion narrowed it into a Phase 1 boundary: service traits are good, but pure semantic core crates should not depend on a giant platform interface.

---

## Agent Discretion

- Exact dependency versions, generated file names, and smoke-test implementation details may be chosen during planning if they preserve the locked decisions.

## Deferred Ideas

- App-bundled FFmpeg runtime management.
- FFmpeg binary distribution notices and manifest.
- Hardware encoder implementation and probing.
- iOS/Android/server platform backends.
