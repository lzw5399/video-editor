---
phase: 01
slug: foundation-and-golden-harness
status: verified
threats_open: 0
asvs_level: 1
created: 2026-06-17
---

# Phase 01 - Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| developer/CI shell -> build tools | Local and CI commands execute package manager, compiler, native binding, and test behavior. | Commands, lockfiles, toolchain config |
| package manager -> install scripts | npm/pnpm and Cargo dependency resolution may execute package build hooks. | Dependency metadata and install behavior |
| command JSON -> Rust model | Renderer/native callers submit command envelopes to Rust-owned types. | Untrusted JSON command payloads |
| semantic crates -> service/runtime crates | Pure editor semantics must not depend on platform execution boundaries. | Rust crate dependencies |
| service crates -> OS/runtime | Runtime, filesystem, preview, and testkit services cross into local OS APIs. | Paths, process args, filesystem IO |
| env/PATH -> media_runtime | User-controlled paths select FFmpeg/ffprobe candidates. | Executable paths |
| media_runtime/testkit -> OS process | Rust launches local FFmpeg/ffprobe candidates. | Process args, stdout, stderr |
| process stdout/stderr -> command envelope | External process output is summarized for UI-ready errors. | Bounded process output |
| fixture JSON -> Rust/schema validation | Test fixtures simulate user-controlled command payloads. | JSON fixtures |
| Rust contract types -> generated TypeScript | Generated UI-facing types cross from Rust source of truth into desktop code. | JSON Schema and TS declarations |
| renderer -> preload -> Electron main | Renderer code crosses into privileged desktop APIs through preload. | IPC calls |
| Electron main -> Node-API addon | JavaScript calls native Rust binding functions. | Native binding calls |
| CI runner -> FFmpeg/ffprobe | CI installs media tools for tests without packaging them into the app. | Runner tool binaries |
| generated files -> committed repo | Drift checks compare generated schema/types to committed artifacts. | Generated schema and TypeScript |

---

## Threat Register

| Threat ID | Category | Component | Disposition | Mitigation | Status |
|-----------|----------|-----------|-------------|------------|--------|
| 01-01/T-01-01 | Tampering | `package.json` / `pnpm-lock.yaml` | mitigate | Corepack pin in `package.json:5`; frozen installs in `justfile:13` and `justfile:18`. | closed |
| 01-01/T-01-02 | Tampering | `Cargo.toml` / `Cargo.lock` | mitigate | Locked Cargo checks/build/tests in `package.json:12` and `package.json:16`. | closed |
| 01-01/T-01-03 | Repudiation | root command surface | mitigate | Auditable `just` recipes define build/test entrypoints in `justfile:12` and `justfile:17`. | closed |
| 01-01/T-01-SC | Tampering | npm/cargo installs | mitigate | Direct npm deps are pinned in `apps/desktop-electron/package.json:14`; approved Rust deps appear in crate manifests and `Cargo.lock`. | closed |
| 01-02/T-01-01 | Tampering | `CommandEnvelope` | mitigate | `deny_unknown_fields` on command/result types in `crates/draft_model/src/lib.rs:18`, with negative fixture validation in `crates/draft_model/tests/schema_exports.rs:151`. | closed |
| 01-02/T-01-02 | Information disclosure | contract vocabulary | mitigate | Rust model comments use draft/material/track/segment vocabulary in `crates/draft_model/src/lib.rs:1`; guard scan found no `Asset`/`Clip` aliases in semantic crates. | closed |
| 01-02/T-01-03 | Elevation of privilege | pure semantic crates | mitigate | Runtime/platform traits are forbidden by `docs/runtime-boundaries.md:24`; guard scan found no `FfmpegExecutor`, `PlatformFileSystem`, `PreviewRenderer`, `std::process`, `which`, `ffmpeg`, or `ffprobe` imports in `draft_model`, `draft_commands`, or `engine_core`. | closed |
| 01-02/T-01-SC | Tampering | cargo dependency additions | mitigate | `crates/draft_model/Cargo.toml:13` through `crates/draft_model/Cargo.toml:19` list only approved `schemars`, `serde`, `serde_json`, `ts-rs`, and `jsonschema`. | closed |
| 01-03/T-01-01 | Elevation of privilege | service-boundary trait placement | mitigate | Traits live at consuming boundaries: `media_runtime/src/lib.rs:27`, `project_store/src/lib.rs:12`, `preview_service/src/lib.rs:8`; `crates/platform` is absent. | closed |
| 01-03/T-01-02 | Tampering | pure semantic crate dependencies | mitigate | Pure-crate isolation is documented in `docs/runtime-boundaries.md:24`; guard scan found no platform trait or process imports in pure crates. | closed |
| 01-03/T-01-03 | Repudiation | FFmpeg distribution scope | mitigate | Phase 1 explicitly excludes FFmpeg download, install, bundling, redistribution, and license review in `docs/runtime-boundaries.md:44`. | closed |
| 01-03/T-01-SC | Tampering | cargo dependency additions | mitigate | Service-boundary crate manifests have no unapproved direct dependencies; runtime deps are approved `serde`, `thiserror`, and `which` in `crates/media_runtime/Cargo.toml:13`. | closed |
| 01-04/T-01-01 | Elevation of privilege | `bindings_node` exported functions | mitigate | The only napi exports are `ping`, `version`, and `execute_command` in `crates/bindings_node/src/lib.rs:16`, `:21`, and `:26`. | closed |
| 01-04/T-01-02 | Tampering | `execute_command` payload handling | mitigate | Binding imports Rust contracts in `crates/bindings_node/src/lib.rs:6`, rejects non-Phase-1 command names at `:31`, and deserializes `CommandEnvelope` at `:39`. | closed |
| 01-04/T-01-03 | Denial of service | native error handling | mitigate | Unsupported commands return structured envelopes in `crates/bindings_node/src/lib.rs:33`; tests assert `ok: false`, structured error, and empty events in `crates/bindings_node/tests/binding_smoke.rs:57`. | closed |
| 01-04/T-01-SC | Tampering | cargo dependency additions | mitigate | Approved napi-rs deps are pinned in `crates/bindings_node/Cargo.toml:16`, `:17`, and `:22`. | closed |
| 01-05/T-01-01 | Spoofing/Tampering | `VE_FFMPEG_PATH`, `VE_FFPROBE_PATH`, PATH lookup | mitigate | Env vars are defined in `crates/media_runtime/src/discovery.rs:31`; explicit paths are checked before PATH in `:60`, and tests assert env precedence in `crates/media_runtime/tests/discovery.rs:13`. | closed |
| 01-05/T-01-02 | Denial of service | FFmpeg/ffprobe stderr/stdout | mitigate | `MAX_STDERR_SUMMARY_BYTES` is defined in `crates/media_runtime/src/discovery.rs:11`, enforced in `:190`, and tested in `crates/media_runtime/tests/discovery.rs:94`. | closed |
| 01-05/T-01-03 | Tampering/Elevation of privilege | process execution | mitigate | Process execution uses `Command::new(binary).args(args)` in `crates/media_runtime/src/process.rs:17`; version probes pass `-version` as an arg in `crates/media_runtime/src/discovery.rs:116`. | closed |
| 01-05/T-01-04 | Repudiation | runtime probe command | mitigate | Stable discovery errors include kind, checked paths, remediation, and summaries in `crates/media_runtime/src/error.rs:14`; binding maps them to `RuntimeDiscoveryFailed` in `crates/bindings_node/src/lib.rs:100`. | closed |
| 01-05/T-01-SC | Tampering | cargo dependency additions | mitigate | Runtime deps are approved `serde`, `thiserror`, and `which` in `crates/media_runtime/Cargo.toml:13` through `:15`. | closed |
| 01-06/T-01-01 | Tampering | generated schema/types | mitigate | Generated artifacts are produced from Rust in `crates/draft_model/tests/schema_exports.rs:25`, and drift is gated by `justfile:25` / `package.json:22`. | closed |
| 01-06/T-01-02 | Tampering | unknown-field fixture | mitigate | Negative fixtures are listed in `crates/draft_model/tests/schema_exports.rs:105` and must fail serde plus JSON Schema at `:151` and `:157`. | closed |
| 01-06/T-01-03 | Repudiation | fixture coverage | mitigate | Test classifies every `fixtures/draft/*.json` in `crates/draft_model/tests/schema_exports.rs:102` through `:134`. | closed |
| 01-06/T-01-SC | Tampering | cargo dependency additions | mitigate | Approved schema/test deps are pinned in `crates/draft_model/Cargo.toml:13` through `:19`. | closed |
| 01-07/T-01-01 | Tampering/Elevation of privilege | render smoke FFmpeg invocation | mitigate | Testkit builds FFmpeg args as `OsString` arrays in `crates/testkit/src/lib.rs:226`; desktop executor runs explicit args in `crates/media_runtime_desktop/src/lib.rs:48`. | closed |
| 01-07/T-01-02 | Denial of service | FFmpeg/ffprobe output | mitigate | Testkit bounds process-output summaries using `MAX_STDERR_SUMMARY_BYTES` in `crates/testkit/src/lib.rs:375`. | closed |
| 01-07/T-01-03 | Tampering | binary media fixtures | mitigate | `find fixtures goldens -type f` returned only JSON fixtures, `.gitkeep`, and `goldens/README.md`; binary-media grep found no MP4/MOV/WAV/AAC/PNG/JPEG/WebM/MKV files. | closed |
| 01-07/T-01-04 | Spoofing | FFmpeg/ffprobe candidates | mitigate | Smoke generation calls runtime discovery before execution in `crates/testkit/src/lib.rs:109`; probing also discovers runtime at `:155`. | closed |
| 01-07/T-01-SC | Tampering | cargo dependency additions | mitigate | Testkit uses approved `tempfile` plus existing workspace deps in `crates/testkit/Cargo.toml:13` through `:16`. | closed |
| 01-08/T-01-01 | Elevation of privilege | preload bridge | mitigate | Preload exposes only `ping`, `version`, and `executeCommand` in `apps/desktop-electron/src/preload/index.ts:7`; tests assert exact keys and no raw `ipcRenderer` in `electron-smoke.spec.ts:85` through `:98`. | closed |
| 01-08/T-01-02 | Tampering | renderer command payload | mitigate | Renderer/preload/main import generated contracts in `App.tsx:3`, `preload/index.ts:3`, and `main/index.ts:5`; Rust validation is in `draft_model/src/lib.rs:18`. | closed |
| 01-08/T-01-03 | Information disclosure | native binding load errors | mitigate | Native binding load errors are bounded by `MAX_LOAD_ERROR_LENGTH` in `nativeBinding.ts:18` and truncated in `nativeBinding.ts:105`. | closed |
| 01-08/T-01-04 | Tampering/Elevation of privilege | renderer/media boundary | mitigate | Guard scan found no `ffmpeg` or `ffprobe` references under `apps/desktop-electron/src`; IPC sender and navigation are restricted in `main/index.ts:45` and `:89`, with untrusted navigation tested in `electron-smoke.spec.ts:149`. | closed |
| 01-08/T-01-SC | Tampering | npm dependency additions | mitigate | Direct npm deps are the approved set in `apps/desktop-electron/package.json:14` through `:24`; lockfile importer pins them in `pnpm-lock.yaml:9`. | closed |
| 01-09/T-01-01 | Tampering | generated schema/type files | mitigate | `just test` gates generated contract drift with `git diff --exit-code` in `justfile:25` and `package.json:22`. | closed |
| 01-09/T-01-02 | Repudiation | CI/local gate divergence | mitigate | CI runs the same local commands in `.github/workflows/ci.yml:58` and `.github/workflows/ci.yml:61`. | closed |
| 01-09/T-01-03 | Spoofing/Tampering | FFmpeg availability in CI | mitigate | CI installs/probes FFmpeg tools in `.github/workflows/ci.yml:26`; render smoke is required by `justfile:24`. | closed |
| 01-09/T-01-04 | Information disclosure | CI artifacts | mitigate | Guard scan found no `electron-builder`, `upload-artifact`, release, or FFmpeg bundling path in `.github/workflows/ci.yml`. | closed |
| 01-09/T-01-SC | Tampering | package installs | mitigate | `pnpm install --frozen-lockfile` runs in `justfile:13` and `:18`; locked Cargo checks/tests run through `package.json:12` and `:16`. | closed |

*Status: open - closed*
*Disposition: mitigate (implementation required) - accept (documented risk) - transfer (third-party)*

---

## Threat Flags

All `## Threat Flags` sections from `01-01-SUMMARY.md` through `01-09-SUMMARY.md` were parsed.

Unregistered flags: none.

Mapped informational flags:

| Summary | Result |
|---------|--------|
| 01-01 through 01-03 | `None.` |
| 01-04 | JavaScript-to-Node-API addon and command-envelope handling mapped to 01-04 threat model. |
| 01-05 | env/PATH, OS process, process-output, and runtime-probe surfaces mapped to 01-05 threat model. |
| 01-06 | generated-contract and fixture-validation surfaces mapped to 01-06 threat model. |
| 01-07 | FFmpeg/ffprobe process execution and ffprobe JSON parsing mapped to 01-07 threat model. |
| 01-08 | renderer-to-preload, preload-to-main, and main-to-native surfaces mapped to 01-08 threat model. |
| 01-09 | CI shell, FFmpeg/ffprobe runner dependency, and generated-contract drift surfaces mapped to 01-09 threat model. |

---

## Accepted Risks Log

No accepted risks.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-06-17 | 40 | 40 | 0 | Codex gsd-secure-phase |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-06-17
