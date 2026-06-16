---
phase: 01-foundation-and-golden-harness
reviewed: 2026-06-16T23:09:20Z
depth: standard
files_reviewed: 51
files_reviewed_list:
  - .github/workflows/ci.yml
  - apps/desktop-electron/index.html
  - apps/desktop-electron/package.json
  - apps/desktop-electron/playwright.config.ts
  - apps/desktop-electron/src/generated/CommandEnvelope.ts
  - apps/desktop-electron/src/generated/CommandResultEnvelope.ts
  - apps/desktop-electron/src/main/index.ts
  - apps/desktop-electron/src/main/nativeBinding.ts
  - apps/desktop-electron/src/preload/index.ts
  - apps/desktop-electron/src/renderer/App.tsx
  - apps/desktop-electron/src/renderer/main.tsx
  - apps/desktop-electron/src/renderer/styles.css
  - apps/desktop-electron/tests/electron-smoke.spec.ts
  - apps/desktop-electron/tsconfig.json
  - apps/desktop-electron/vite.config.ts
  - crates/bindings_node/Cargo.toml
  - crates/bindings_node/build.rs
  - crates/bindings_node/src/lib.rs
  - crates/bindings_node/tests/binding_smoke.rs
  - crates/draft_commands/Cargo.toml
  - crates/draft_commands/src/lib.rs
  - crates/draft_model/Cargo.toml
  - crates/draft_model/src/lib.rs
  - crates/draft_model/tests/contract.rs
  - crates/draft_model/tests/schema_exports.rs
  - crates/engine_core/Cargo.toml
  - crates/engine_core/src/lib.rs
  - crates/ffmpeg_compiler/Cargo.toml
  - crates/ffmpeg_compiler/src/lib.rs
  - crates/media_runtime/Cargo.toml
  - crates/media_runtime/src/discovery.rs
  - crates/media_runtime/src/error.rs
  - crates/media_runtime/src/lib.rs
  - crates/media_runtime/tests/discovery.rs
  - crates/media_runtime_desktop/Cargo.toml
  - crates/media_runtime_desktop/src/lib.rs
  - crates/preview_service/Cargo.toml
  - crates/preview_service/src/lib.rs
  - crates/project_store/Cargo.toml
  - crates/project_store/src/lib.rs
  - crates/render_graph/Cargo.toml
  - crates/render_graph/src/lib.rs
  - crates/testkit/Cargo.toml
  - crates/testkit/src/lib.rs
  - crates/testkit/tests/render_smoke.rs
  - docs/runtime-boundaries.md
  - fixtures/draft/invalid-unknown-field.json
  - fixtures/draft/minimal-command.json
  - fixtures/media-generated/.gitkeep
  - goldens/README.md
  - schemas/command.schema.json
findings:
  critical: 2
  warning: 2
  info: 0
  total: 4
status: issues_found
---

# Phase 01: Code Review Report

**Reviewed:** 2026-06-16T23:09:20Z
**Depth:** standard
**Files Reviewed:** 51
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

Reviewed the Electron shell, preload bridge, generated Rust/TypeScript command contracts, Node-API binding, runtime discovery/executor code, fixture/schema gates, CI wiring, and boundary documentation. The implementation has blocker-level IPC/runtime boundary defects: malformed command envelopes can execute by `command` while ignoring a contradictory `payload.kind`, and renderer-triggered runtime probes can block indefinitely on external FFmpeg-family processes. The remaining warnings are generated-contract drift around `requestId` optionality and stale runtime-boundary documentation.

Targeted context command run: `cargo test -p bindings_node execute_command` passed 4 covered binding tests, but those tests do not cover mismatched `command`/`payload.kind` envelopes or hung runtime probes.

## Critical Issues

### CR-01: BLOCKER - Mismatched Command Envelopes Execute Instead Of Being Rejected

**File:** `crates/bindings_node/src/lib.rs:40`
**Related:** `crates/bindings_node/src/lib.rs:51`, `crates/draft_model/src/lib.rs:18`, `schemas/command.schema.json:35`, `crates/draft_model/tests/schema_exports.rs:83`
**Issue:** `execute_command` deserializes `CommandEnvelope` and dispatches only on `envelope.command`. The payload enum is accepted independently, so `{ "command": "version", "payload": { "kind": "ping" } }` deserializes successfully and returns the version response instead of an `invalidPayload` error. The generated JSON Schema has the same gap because it constrains `command` and `payload` separately. This breaks the IPC contract at the renderer/native boundary and lets malformed commands execute a different operation than their payload declares.
**Fix:**
```rust
impl CommandPayload {
    pub fn command_name(&self) -> CommandName {
        match self {
            Self::Ping(_) => CommandName::Ping,
            Self::Version(_) => CommandName::Version,
            Self::ProbeMediaRuntime(_) => CommandName::ProbeMediaRuntime,
        }
    }
}

let envelope = match serde_json::from_value::<CommandEnvelope>(command) {
    Ok(envelope) => envelope,
    Err(error) => {
        return to_js_value(error_envelope(
            CommandErrorKind::InvalidPayload,
            format!("Invalid command envelope: {error}"),
            command_name,
        ));
    }
};

if envelope.payload.command_name() != envelope.command {
    return to_js_value(error_envelope(
        CommandErrorKind::InvalidPayload,
        "Command name does not match payload kind".to_string(),
        Some(format!("{:?}", envelope.command)),
    ));
}
```
Add a negative fixture such as `invalid-mismatched-command-payload.json`, classify it in `schema_fixtures_validate_command_contracts`, and add binding tests that assert mismatches return `invalidPayload`. If JSON Schema is meant to be an external contract, replace the top-level schema with a `oneOf` of paired `{ command: const, payload.kind: const }` variants or add equivalent conditional constraints.

### CR-02: BLOCKER - IPC-Triggered Runtime Discovery Can Hang The Main Process And CI Indefinitely

**File:** `crates/media_runtime/src/discovery.rs:105`
**Related:** `crates/media_runtime_desktop/src/lib.rs:25`, `crates/media_runtime_desktop/src/lib.rs:29`, `crates/testkit/src/lib.rs:249`, `apps/desktop-electron/src/main/index.ts:11`
**Issue:** Runtime discovery and desktop execution use `Command::output()` with no timeout or cancellation. The renderer can invoke `core:executeCommand` with `probeMediaRuntime`, which runs `discover_runtime_config()` synchronously through the Node binding. If `VE_FFMPEG_PATH`, `VE_FFPROBE_PATH`, or PATH resolves to a binary/script that never exits, the Electron main process blocks on the IPC handler and the CI render/runtime gates can hang until the outer job timeout. This is a runtime-boundary correctness failure, not just a performance concern.
**Fix:**
```rust
const VERSION_PROBE_TIMEOUT: Duration = Duration::from_secs(5);

fn run_with_timeout(path: &Path, args: &[&str], timeout: Duration) -> io::Result<Output> {
    let mut child = Command::new(path)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    match child.wait_timeout(timeout)? {
        Some(_) => child.wait_with_output(),
        None => {
            let _ = child.kill();
            let output = child.wait_with_output()?;
            Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("{} timed out after {:?}", path.display(), timeout),
            ))
        }
    }
}
```
Introduce a bounded process helper in the runtime boundary, map timeout failures to `DiscoveryErrorKind::VersionProbeFailed` or a dedicated timeout kind, and add tests using a fake binary that sleeps longer than the timeout. Apply the same bounded execution path to `DesktopFfmpegExecutor::run` so render smoke gates cannot hang indefinitely.

## Warnings

### WR-01: WARNING - Generated TypeScript Requires `requestId` That Rust And JSON Schema Treat As Optional

**File:** `apps/desktop-electron/src/generated/CommandEnvelope.ts:8`
**Related:** `crates/draft_model/src/lib.rs:21`, `schemas/command.schema.json:21`, `crates/draft_model/tests/contract.rs:20`
**Issue:** Rust models `request_id` as `Option<String>` and the schema does not include `requestId` in `required`, so envelopes without `requestId` are valid and are already tested. The generated TypeScript declares `requestId: string | null` as a required property, so TypeScript clients cannot construct a schema-valid minimal command without adding `requestId`. That is generated-contract drift between Rust, JSON Schema, and renderer types.
**Fix:** Mark the Rust field optional for ts-rs generation and regenerate artifacts through the existing schema export gate.
```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
#[ts(optional)]
pub request_id: Option<String>,
```
The regenerated TypeScript should be `requestId?: string | null` or equivalent, and a TypeScript compile-time fixture should cover the minimal command shape from `fixtures/draft/minimal-command.json`.

### WR-02: WARNING - Runtime Boundary Documentation Contradicts Implemented Discovery Behavior

**File:** `docs/runtime-boundaries.md:61`
**Related:** `crates/media_runtime/src/discovery.rs:62`, `crates/media_runtime/src/error.rs:21`, `crates/media_runtime_desktop/src/lib.rs:16`
**Issue:** The Desktop Runtime section says later discovery work will add version probes, structured missing-binary errors, checked paths, bounded stderr summaries, and argument-array process execution. Those behaviors are already implemented in `media_runtime`, while `media_runtime_desktop` is currently only the executor shell. This stale source documentation will mislead later phases about which boundary owns discovery and what remains deferred.
**Fix:** Update lines 61-65 to state that `media_runtime` already owns FFmpeg/ffprobe discovery, version probing, checked paths, and bounded output summaries, while `media_runtime_desktop::DesktopFfmpegExecutor` owns desktop process execution. Explicitly list the remaining deferred work, such as timeouts/cancellation, packaged binary management, and licensing review for redistributed FFmpeg builds.

---

_Reviewed: 2026-06-16T23:09:20Z_
_Reviewer: the agent (gsd-code-reviewer)_
_Depth: standard_
