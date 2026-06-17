---
phase: 02
slug: draft-and-material-system
status: verified
threats_open: 0
asvs_level: 1
block_on: high
register_authored_at_plan_time: true
created: 2026-06-17
verified: 2026-06-17
---

# Phase 02 - Security

Per-phase security contract for the draft and material system. This audit verifies the plan-time threat register from all six Phase 02 PLAN files against implemented code, tests, generated contracts, and gate scripts.

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| `project.json -> draft_model` | Untrusted persisted draft JSON crosses into strict Rust schema and migration logic. | Draft/material/timeline JSON |
| `filesystem -> project_store` | Local `.veproj/project.json` content and material paths cross into project persistence. | UTF-8 JSON, material URI strings |
| `media path -> media_runtime` | Local media paths cross into bounded ffprobe execution. | Local file paths, ffprobe process output |
| `ffprobe JSON -> media_runtime` | External process JSON is parsed into typed normalized metadata. | ffprobe stdout/stderr |
| `material_service -> project_store/media_runtime/draft_model` | Binding-facing service orchestrates path helpers, probing, registry helpers, validation, and save. | Material import requests and diagnostics |
| `Electron renderer -> preload/main -> Rust binding` | Renderer-originated commands cross through generated command contracts and typed preload API. | Command envelopes and material metadata |
| `generated contracts -> Electron TS` | Rust-owned schema and TypeScript artifacts are consumed by Electron and test fixtures. | JSON Schema and generated TS |

## Threat Register

| Threat ID | Category | Component | Disposition | Mitigation | Status | Evidence |
|-----------|----------|-----------|-------------|------------|--------|----------|
| T-02-01 | Tampering | `Draft` serde/schema | mitigate | Deny unknown fields, semantic validation, malformed project tests. | closed | `crates/draft_model/src/draft.rs:24`, `:42`; `crates/draft_model/src/validation.rs:61`; `crates/draft_model/tests/draft_schema.rs:330`; `crates/draft_model/tests/draft_fixtures.rs:75`. |
| T-02-02 | Tampering | Draft terminology/model | mitigate | Use Jianying names and guard against `Asset`/`Clip` aliases. | closed | `crates/draft_model/src/draft.rs:43`; `crates/draft_model/src/material.rs:87`; `crates/draft_model/src/timeline.rs:21`; `apps/desktop-electron/src/generated/Draft.ts:14`; `package.json:26`; `pnpm run test:phase2-source-guards` passed. |
| T-02-03 | Tampering | Schema migration | mitigate | Reject unknown future schema versions with structured recoverable errors. | closed | `crates/draft_model/src/validation.rs:64`; `crates/draft_model/src/validation.rs:69`; `crates/draft_model/tests/draft_schema.rs:113`; `crates/project_store/tests/project_bundle.rs:146`. |
| T-02-04 | Information Disclosure/Tampering | Semantic draft model | mitigate | Exclude derived paths and raw probe JSON from persisted Draft. | closed | `crates/draft_model/src/validation.rs:188`; `crates/draft_model/tests/draft_schema.rs:383`; `docs/runtime-boundaries.md:90`; `package.json:26`; source guard passed. |
| T-02-05 | Tampering | `.veproj/project.json` loader | mitigate | Strict JSON parse, migration, validation, and structured errors. | closed | `crates/project_store/src/bundle.rs:87`; `crates/project_store/src/bundle.rs:92`; `crates/project_store/src/error.rs:15`; `crates/project_store/tests/project_bundle.rs:103`; `:121`; `:146`; `:164`. |
| T-02-06 | Tampering | Save/open round trip | mitigate | Semantic equality tests and no silent invalid normalization. | closed | `crates/project_store/src/bundle.rs:38`; `crates/project_store/tests/project_bundle.rs:27`; `crates/project_store/tests/project_bundle.rs:42`; `crates/project_store/tests/project_bundle.rs:82`. |
| T-02-07 | Tampering/Information Disclosure | Material URI path resolution | mitigate | Central path classification and traversal rejection. | closed | `crates/project_store/src/paths.rs:25`; `crates/project_store/src/paths.rs:51`; `crates/project_store/src/paths.rs:88`; `crates/project_store/tests/project_bundle.rs:82`; `crates/project_store/src/paths.rs:207`. |
| T-02-08 | Tampering | Project-store responsibility boundary | mitigate | Project store does not call ffprobe, media_runtime, or registry import helpers. | closed | `crates/project_store/src/bundle.rs:3`; `crates/project_store/src/bundle.rs:7`; `package.json:26`; direct `rg` for forbidden project_store ownership patterns returned no matches. |
| T-02-09 | Tampering | Missing material persistence | mitigate | Preserve entries when media files are absent. | closed | `crates/project_store/src/bundle.rs:106`; `crates/project_store/src/bundle.rs:117`; `crates/project_store/tests/project_bundle.rs:181`; `crates/project_store/tests/project_bundle.rs:192`. |
| T-02-10 | Denial of Service | ffprobe process | mitigate | Use `FfmpegExecutor` argument arrays, timeout classification, bounded summaries, and no shell strings. | closed | `crates/media_runtime/src/lib.rs:42`; `crates/media_runtime/src/probe.rs:151`; `crates/media_runtime/src/probe.rs:163`; `crates/media_runtime/src/probe.rs:192`; `crates/media_runtime/src/probe.rs:384`; `crates/media_runtime/tests/material_probe.rs:95`. |
| T-02-11 | Tampering | ffprobe JSON parser | mitigate | Parse typed normalized metadata only; malformed JSON classified; no raw JSON persistence. | closed | `crates/media_runtime/src/probe.rs:178`; `crates/media_runtime/src/probe.rs:399`; `crates/media_runtime/tests/material_probe.rs:126`; `crates/draft_model/src/validation.rs:202`; source guard passed. |
| T-02-12 | Tampering | Time/fps normalization | mitigate | Convert decimal durations to integer microseconds and fps to rational values; reject malformed/zero. | closed | `crates/media_runtime/src/probe.rs:239`; `crates/media_runtime/src/probe.rs:318`; `crates/media_runtime/src/probe.rs:350`; `crates/media_runtime/tests/material_probe.rs:138`; `crates/media_runtime/tests/material_probe.rs:150`; `crates/media_runtime/tests/material_probe.rs:162`. |
| T-02-13 | Information Disclosure | Generated media fixtures | mitigate | Use temp directories only; no committed binary media fixtures. | closed | `crates/testkit/src/lib.rs:118`; `crates/testkit/src/lib.rs:512`; `crates/testkit/src/lib.rs:721`; `find fixtures goldens ...` returned no media files. |
| T-02-14 | Tampering/Information Disclosure | Material URI resolution | mitigate | Resolve through project-store helpers and preserve original URI for diagnostics. | closed | `crates/bindings_node/src/material_service.rs:137`; `crates/bindings_node/src/material_service.rs:396`; `crates/bindings_node/src/material_service.rs:417`; `crates/bindings_node/tests/material_service.rs:166`; `crates/project_store/src/paths.rs:88`. |
| T-02-15 | Tampering | Material import responsibility | mitigate | Material service owns import; project_store grep/tests prevent ffprobe/registry ownership. | closed | `crates/bindings_node/src/material_service.rs:12`; `crates/bindings_node/src/material_service.rs:16`; `crates/bindings_node/src/material_service.rs:176`; `crates/bindings_node/src/material_service.rs:230`; `package.json:26`; project_store forbidden-pattern grep returned no matches. |
| T-02-16 | Tampering | Material registry mutation | mitigate | Use pure draft_model helpers and validate before save. | closed | `crates/draft_model/src/material.rs:114`; `crates/draft_model/src/material.rs:126`; `crates/draft_model/src/material.rs:180`; `crates/bindings_node/src/material_service.rs:159`; `crates/project_store/src/bundle.rs:38`. |
| T-02-17 | Tampering | Missing material handling | mitigate | Recoverable diagnostics preserve material entries. | closed | `crates/bindings_node/src/material_service.rs:147`; `crates/bindings_node/src/material_service.rs:239`; `crates/bindings_node/src/material_service.rs:390`; `crates/bindings_node/tests/material_service.rs:117`; `crates/bindings_node/tests/binding_smoke.rs:150`. |
| T-02-18 | Tampering | Generated draft schema/TS | mitigate | Rust generator plus committed artifact drift gate. | closed | `crates/draft_model/tests/schema_exports.rs:30`; `crates/draft_model/tests/schema_exports.rs:82`; `crates/draft_model/tests/schema_exports.rs:143`; `package.json:27`; `git diff --exit-code schemas apps/desktop-electron/src/generated` passed. |
| T-02-19 | Elevation of Privilege | Renderer-native bridge | mitigate | Renderer uses generated preload command API only; no renderer fs/process/media runtime access. | closed | `apps/desktop-electron/src/preload/index.ts:8`; `apps/desktop-electron/src/main/index.ts:36`; `apps/desktop-electron/src/main/index.ts:89`; `apps/desktop-electron/src/renderer/App.tsx:3`; `apps/desktop-electron/tests/electron-smoke.spec.ts:79`; `package.json:26`; source guard passed. |
| T-02-20 | Tampering | Material command payloads | mitigate | Strict Rust payloads and generated schema/TS prevent IPC drift. | closed | `crates/draft_model/src/lib.rs:36`; `crates/draft_model/src/lib.rs:85`; `crates/draft_model/src/lib.rs:129`; `schemas/command.schema.json:17`; `schemas/command.schema.json:570`; `apps/desktop-electron/src/generated/CommandEnvelope.ts:9`; `crates/bindings_node/tests/binding_smoke.rs:214`. |
| T-02-21 | Tampering | Binding route responsibility | mitigate | Route through material_service; no direct Electron ffprobe/project JSON mutation. | closed | `crates/bindings_node/src/lib.rs:81`; `crates/bindings_node/src/lib.rs:145`; `crates/bindings_node/src/material_service.rs:128`; `apps/desktop-electron/src/renderer/App.tsx:89`; `apps/desktop-electron/tests/electron-smoke.spec.ts:144`; source guard passed. |
| T-02-22 | Information Disclosure | Material metadata display | mitigate | UI displays normalized material fields/status only, not raw ffprobe JSON or process logs. | closed | `apps/desktop-electron/src/renderer/App.tsx:196`; `apps/desktop-electron/src/renderer/App.tsx:203`; `apps/desktop-electron/tests/electron-smoke.spec.ts:132`; `crates/bindings_node/src/material_service.rs:287`; `crates/bindings_node/src/material_service.rs:316`. |
| T-02-23 | Tampering | Fixture corpus | mitigate | Explicit fixture classification and positive/negative schema/model behavior. | closed | `crates/draft_model/tests/draft_fixtures.rs:18`; `crates/draft_model/tests/draft_fixtures.rs:39`; `crates/draft_model/tests/draft_fixtures.rs:75`; `crates/draft_model/tests/draft_fixtures.rs:94`; `crates/draft_model/tests/draft_fixtures.rs:102`. |
| T-02-24 | Tampering | Generated schema/TS | mitigate | Rust generator plus git diff gate. | closed | `crates/draft_model/tests/schema_exports.rs:30`; `crates/draft_model/tests/schema_exports.rs:245`; `crates/draft_model/tests/schema_exports.rs:258`; `justfile:29`; `justfile:30`; `git diff --exit-code schemas apps/desktop-electron/src/generated` passed. |
| T-02-25 | Repudiation | Requirement coverage | mitigate | Every Phase 2 requirement maps to automated test/fixture/integration proof. | closed | `package.json:17`; `package.json:19`; `package.json:21`; `package.json:22`; `package.json:23`; `package.json:24`; `crates/draft_model/tests/draft_fixtures.rs:18`; `crates/bindings_node/tests/material_service.rs:17`; `apps/desktop-electron/tests/electron-smoke.spec.ts:79`. |
| T-02-26 | Elevation of Privilege/DoS | Final gates | mitigate | Final gates include renderer FFmpeg grep, ffprobe timeout/malformed JSON tests, missing-material corruption tests, and project-store responsibility grep. | closed | `package.json:19`; `package.json:21`; `package.json:26`; `justfile:17`; `justfile:29`; `crates/media_runtime/tests/material_probe.rs:95`; `crates/media_runtime/tests/material_probe.rs:126`; `crates/project_store/tests/project_bundle.rs:181`. |
| T-02-SC | Tampering | Supply chain | accept | Accepted risk for no planned package installs; actual `windows-sys` addition is Windows-only and already locked. | closed | Accepted risk log below; `crates/project_store/Cargo.toml:17`; `crates/project_store/Cargo.toml:18`; `Cargo.lock:1249`; `Cargo.lock:1250`; `Cargo.lock:1252`. |

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-02-SC-001 | T-02-SC | Phase plans accepted no new package-manager install. Implementation added `windows-sys = 0.61.2` only as a Windows-target direct dependency for atomic replacement semantics in `project_store`; the crate was already present in `Cargo.lock` with checksum and is cfg-gated to Windows. | Plan-time Phase 02 threat model; verified by Codex security audit | 2026-06-17 |

## Threat Flags

| Source | Flag | Resolution |
|--------|------|------------|
| 02-02 SUMMARY | None - filesystem/project-store boundary covered by plan threat model. | No unregistered flag. |
| 02-04 SUMMARY | None - local material path, project-store, media-runtime, generated-contract boundaries covered. | No unregistered flag. |
| 02-05 SUMMARY | None - binding/material_service/renderer display boundaries covered. | No unregistered flag. |
| 02-06 SUMMARY | None - local fixtures, classification, gate scripts, grep guards only. | No unregistered flag. |
| 02-01, 02-03 SUMMARY | No extra threat flags found. | No unregistered flag. |

## Verification Commands Run

| Command | Result |
|---------|--------|
| `pnpm run test:phase2-source-guards` | passed |
| `git diff --exit-code schemas apps/desktop-electron/src/generated` | passed |
| `find fixtures goldens -type f \( -name '*.mp4' -o -name '*.mov' -o -name '*.wav' -o -name '*.aac' -o -name '*.png' \) -print` | returned no files |
| `rg -n "probe_material_metadata\|FfmpegExecutor\|ffprobe\|ffmpeg\|add_material\|upsert_material\|mark_material_(available\|missing\|probe_failed)" crates/project_store/src` | returned no matches |

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-06-17 | 27 | 27 | 0 | Codex security audit |

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-06-17
