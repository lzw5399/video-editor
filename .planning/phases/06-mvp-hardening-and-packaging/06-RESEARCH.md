# Phase 06: MVP Hardening And Packaging - Research

**Researched:** 2026-06-18
**Domain:** Electron packaging, Node-API native binding distribution, FFmpeg runtime capability reporting, no-mock Electron E2E, and release/license documentation
**Confidence:** HIGH for local architecture and external-FFmpeg MVP; MEDIUM for exact `electron-builder` directory-package CLI spelling because the generated docs were not cleanly extractable in this session.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

### Packaging Strategy
- **D-01:** Use a standard Electron packaging tool for Phase 6, with `electron-builder` as the preferred default unless implementation research finds a concrete blocker in this repository.
- **D-02:** Add explicit package scripts rather than overloading the existing dev build scripts. Expected command surface should include a directory-package smoke path (`package:dir` / `test:packaged`) before any installer or signed artifact work.
- **D-03:** Packaged app smoke must launch from packaged artifacts, not from `dist/main/index.cjs`, and must verify offline file-based renderer loading, preload bridge availability, `ping`, `version`, and `probeMediaRuntime`.
- **D-04:** The native Node-API binding resolver must support development, unpackaged test, and packaged app paths. Packaged resolution should explicitly handle Electron resource paths / asar-unpacked behavior rather than relying only on relative `dist` paths.
- **D-05:** Build output must be clean before packaging. Existing stale Vite renderer assets in `dist/renderer/assets` should not be copied into packaged artifacts.

### FFmpeg Runtime And License Posture
- **D-06:** Phase 6 MVP packaged smoke may continue using externally configured FFmpeg/ffprobe through `VE_FFMPEG_PATH`, `VE_FFPROBE_PATH`, or `PATH`. Do not silently download FFmpeg.
- **D-07:** If Phase 6 chooses to bundle FFmpeg binaries, that same plan must create and test a license/build manifest, third-party notices, and a runtime path resolver for packaged resources. Bundling without those artifacts is not allowed.
- **D-08:** Packaged runtime failures must be classified and user-actionable. Missing FFmpeg/ffprobe should produce a clear Chinese UI/runtime error rather than a crash or renderer-owned fallback.
- **D-09:** Product runtime should expose a real capability report before preview/export hardening is considered complete. At minimum this report should cover discovered ffmpeg/ffprobe paths, version/configure summary, H.264/AAC encoder availability, ASS/subtitles filter availability, and deterministic font readiness.
- **D-10:** Local Homebrew FFmpeg with `--enable-gpl` is acceptable for development tests but must not be represented as the project's redistributable FFmpeg build.

### Real MVP E2E Scope
- **D-11:** Keep the existing mock-based workspace tests, but add a separate no-mock Electron E2E gate that generates deterministic temporary media, imports it through UI commands, adds/edits timeline content, requests real preview artifacts, starts export, polls status to completion, and validates the output file.
- **D-12:** Real preview/export E2E must still go through `window.videoEditorCore.executeCommand`; renderer code must not construct FFmpeg commands, render graphs, export scripts, validation expectations, or process handles for test convenience.
- **D-13:** Packaged smoke and real E2E can be small and deterministic. The goal is confidence in wiring and runtime boundaries, not large project performance testing.

### Release Readiness
- **D-14:** Phase 6 should produce a concise known-limits document for MVP, covering external FFmpeg dependency, packaging/signing limitations, unsupported advanced editor semantics, and deferred compatibility/mobile/server work.
- **D-15:** Add release gates to public command surfaces. `pnpm run test` and `just test` should include Phase 6 checks that are suitable for local/CI execution, or clearly separate slower/packaging gates with documented commands.
- **D-16:** Desktop user-facing copy remains Simplified Chinese and Jianying-style. Release/diagnostic UI should not introduce English-only product copy.

### the agent's Discretion
- The implementation may choose exact package output names, release directory names, and Playwright helper structure as long as the resulting commands are deterministic and covered by gates.
- Signing/notarization may be documented as a known limit or placeholder unless local tooling and certificates are available. The hard requirement is offline packaged launch smoke, not public distribution signing.

### Deferred Ideas (OUT OF SCOPE)

- Code signing, notarization, installer polish, auto-update, and app icon polish may be documented as known limits unless they are cheap and non-blocking.
- Bundled FFmpeg binaries can be deferred if external FFmpeg smoke is documented and tested.
- Advanced editor semantics remain in Phases 7-13 and should not be pulled into Phase 6.
- Jianying/CapCut/Kaipai adapters remain post-MVP and must not drive Phase 6 packaging scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| TEST-06 | Electron E2E imports material, edits timeline, previews, exports, and verifies output. | Add a separate no-mock Playwright Electron test that generates deterministic media, drives existing Chinese UI controls, calls only generated command envelopes, polls export status, and validates the output through ffprobe/Rust-owned validation expectations. [VERIFIED: .planning/REQUIREMENTS.md; VERIFIED: apps/desktop-electron/tests/workspace.spec.ts; VERIFIED: crates/testkit/tests/preview_export_parity.rs] |
| TEST-07 | Packaged app smoke launches offline and completes import-preview-export. | Add a packaged-artifact launch test that uses the packaged executable/resource tree, file renderer loading, unpacked native binding resolution, and external FFmpeg discovery through `probeMediaRuntime`; then run the same small import-preview-export smoke against the packaged app. [VERIFIED: .planning/REQUIREMENTS.md; VERIFIED: 06-CONTEXT.md; CITED: https://playwright.dev/docs/api/class-electron] |
</phase_requirements>

## Summary

Phase 6 should harden what already exists instead of adding editor semantics: build the Electron desktop app into a directory package, launch that package offline, prove the Node-API binding loads outside `dist/main/index.cjs`, and prove the Rust-owned FFmpeg runtime can complete a real import/preview/export path. [VERIFIED: 06-CONTEXT.md; VERIFIED: apps/desktop-electron/src/main/index.ts; VERIFIED: apps/desktop-electron/src/main/nativeBinding.ts]

The recommended MVP path is external FFmpeg only: continue resolving `VE_FFMPEG_PATH`, `VE_FFPROBE_PATH`, then `PATH`; generate release docs that state FFmpeg is not bundled; record local runtime capability/build data for tests; and defer bundled FFmpeg until a later plan includes resource resolution plus legal artifacts. [VERIFIED: crates/media_runtime/src/discovery.rs; VERIFIED: docs/runtime-boundaries.md; CITED: https://ffmpeg.org/legal.html]

**Primary recommendation:** Use `electron-builder` for a directory-package first, set ASAR/unpacked rules for the native `.node` binding, keep FFmpeg external for MVP, add a Rust-owned runtime capability report command, and split Phase 6 into packaging, no-mock E2E/release gates, and release documentation. [VERIFIED: 06-CONTEXT.md; VERIFIED: npm registry; CITED: https://www.electronjs.org/docs/latest/tutorial/asar-archives]

## Project Constraints (from AGENTS.md)

- UI emits commands and Rust core owns project/timeline semantics; UI must not construct FFmpeg commands. [VERIFIED: AGENTS.md]
- `.veproj/project.json` is canonical; render graphs, FFmpeg scripts, thumbnails, waveform data, proxy files, preview caches, and exports are derived artifacts. [VERIFIED: AGENTS.md]
- Product, code, IPC, docs, schema, and tests should use Jianying concepts such as draft/material/track/segment/keyframe/filter/transition. [VERIFIED: AGENTS.md]
- Core time math must use integer microseconds, frame indices, or rational frame rates, not naked persisted floating-point time. [VERIFIED: AGENTS.md]
- Render Graph isolates editing semantics from FFmpeg; FFmpeg Runtime executes jobs and reports progress/errors without deciding editing behavior. [VERIFIED: AGENTS.md]
- Kdenlive and MLT are references only; do not copy GPL code/assets/XML/presets/UI. [VERIFIED: AGENTS.md]
- External drafts go through adapters and compatibility reports; proprietary IDs are external references, not internal render semantics. [VERIFIED: AGENTS.md]
- Each roadmap phase must define executable gates before implementation is considered complete. [VERIFIED: AGENTS.md]
- FFmpeg distribution must be reviewed for LGPL/GPL/nonfree options, notices, and commercial obligations. [VERIFIED: AGENTS.md]
- Repo-level workflow says source edits should happen through GSD commands, but this research task is explicitly constrained to writing only `06-RESEARCH.md`. [VERIFIED: AGENTS.md; VERIFIED: user prompt]

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|--------------|----------------|-----------|
| Directory packaging and artifact layout | Frontend Server / Electron main | Build tooling | Electron main owns packaged paths and preload/native binding bootstrapping; build tooling only assembles artifacts. [VERIFIED: apps/desktop-electron/src/main/index.ts; VERIFIED: apps/desktop-electron/src/main/nativeBinding.ts] |
| Native Node-API binding load | Electron main | Rust binding crate | Main resolves packaged/dev paths and requires the binding; Rust binding exposes `ping`, `version`, and `executeCommand`. [VERIFIED: apps/desktop-electron/src/main/nativeBinding.ts; VERIFIED: crates/bindings_node/src/lib.rs] |
| Runtime capability report | API / Rust binding | Electron renderer display | Rust/runtime must probe FFmpeg, encoders, filters, and fonts; renderer displays read-only Chinese diagnostics. [VERIFIED: crates/media_runtime/src/discovery.rs; VERIFIED: crates/testkit/src/render_compare.rs; VERIFIED: 06-UI-SPEC.md] |
| Import/edit/preview/export smoke | Electron E2E test harness | Rust services | Test drives UI and generated commands; Rust services own import, preview, export, and validation semantics. [VERIFIED: apps/desktop-electron/tests/workspace.spec.ts; VERIFIED: crates/bindings_node/src/preview_export_service.rs] |
| FFmpeg legal/release manifest | Release documentation | Runtime capability report | External-runtime MVP documents no bundled binary; bundled future must include build flags/notices/source obligations. [VERIFIED: docs/runtime-boundaries.md; CITED: https://ffmpeg.org/legal.html] |

## Standard Stack

### Core

| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `electron-builder` | 26.15.3 recommended; npm `latest` was 26.15.3 and `v26` was 26.15.4 during research. | Build directory packaged Electron artifacts before installers/signing. | Locked as preferred default by Phase 6 context; npm reports a maintained official repo and no postinstall script. [VERIFIED: 06-CONTEXT.md; VERIFIED: npm registry] |
| `electron` | 42.4.1 existing dependency. | Desktop shell and packaged runtime. | Already pinned in the desktop package and used by Playwright Electron tests. [VERIFIED: apps/desktop-electron/package.json; VERIFIED: apps/desktop-electron/tests/electron-smoke.spec.ts] |
| `@napi-rs/cli` | 3.7.2 existing dev dependency. | Build Rust Node-API binding into `apps/desktop-electron/native`. | Existing build script uses `napi build --platform --release --output-dir native`. [VERIFIED: apps/desktop-electron/package.json] |
| `@playwright/test` | 1.61.0 existing dev dependency. | Electron smoke, workspace layout, and packaged/no-mock E2E tests. | Existing tests launch Electron with `_electron.launch`, and official Playwright Electron docs support launch args/env/executablePath. [VERIFIED: apps/desktop-electron/package.json; CITED: https://playwright.dev/docs/api/class-electron] |
| Rust `media_runtime` + `media_runtime_desktop` | Workspace crates. | FFmpeg discovery, execution, progress/errors, and output validation. | Existing crates own FFmpeg boundary and process execution; renderer must not duplicate it. [VERIFIED: crates/media_runtime/src/lib.rs; VERIFIED: crates/media_runtime_desktop/src/lib.rs] |

### Supporting

| Library / Tool | Version | Purpose | When to Use |
|----------------|---------|---------|-------------|
| FFmpeg / ffprobe | Local FFmpeg 8.1 observed in this environment. | External runtime for tests and MVP packaged smoke. | Use through `VE_FFMPEG_PATH`, `VE_FFPROBE_PATH`, or `PATH`; do not bundle in MVP unless legal artifacts are added. [VERIFIED: local command; VERIFIED: crates/media_runtime/src/discovery.rs; VERIFIED: 06-CONTEXT.md] |
| `testkit::render_compare` | Workspace crate. | Capability probes for encoders, filters, deterministic fonts, RGB frame extraction. | Promote probe logic into product runtime report or move shared pieces out of test-only naming. [VERIFIED: crates/testkit/src/render_compare.rs] |
| `just` | 1.43.0 observed locally. | Public build/test gate wrapper. | Add Phase 6 gates to `just test` or document slower package gates clearly. [VERIFIED: local command; VERIFIED: justfile] |

### Alternatives Considered

| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `electron-builder` | Electron Forge | Forge is an official Electron tool, but Phase 6 context prefers `electron-builder`; switching would add unnecessary migration risk. [VERIFIED: 06-CONTEXT.md; CITED: https://www.electronjs.org/docs/latest/tutorial/asar-archives] |
| External FFmpeg MVP | Bundled FFmpeg resources | Bundling improves offline runtime availability but forces build manifest, third-party notices, source/build-flag obligations, resource path resolver, and packaged-resource tests in the same plan. [VERIFIED: 06-CONTEXT.md; CITED: https://ffmpeg.org/legal.html] |
| Mocked E2E only | No-mock E2E | Existing mocks are useful for UI/layout stability, but TEST-06/TEST-07 require real preview/export proof. [VERIFIED: apps/desktop-electron/tests/workspace.spec.ts; VERIFIED: .planning/REQUIREMENTS.md] |

**Installation:**

```bash
pnpm --filter @video-editor/desktop add -D electron-builder@26.15.3
```

**Version verification:** `npm view electron-builder version time.modified time.created repository.url homepage dist-tags --json` returned 26.15.3 as `latest`, `time.modified` 2026-06-16, and the official repository `electron-userland/electron-builder`. [VERIFIED: npm registry]

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | slopcheck | Disposition |
|---------|----------|-----|-----------|-------------|-----------|-------------|
| `electron-builder` | npm | Created 2022-01-26 in current npm package metadata; repository history predates that metadata. | 2,967,117 downloads for 2026-06-10 through 2026-06-16. | `github.com/electron-userland/electron-builder` | OK | Approved. [VERIFIED: npm registry; VERIFIED: slopcheck] |

**Packages removed due to slopcheck [SLOP] verdict:** none. [VERIFIED: slopcheck]
**Packages flagged as suspicious [SUS]:** none. [VERIFIED: slopcheck]

Audit notes: installed `slopcheck` 0.6.1 did not accept the requested `--json` flag, so the audit used `slopcheck install electron-builder` and then reverted the npm install side effects from the workspace. [VERIFIED: local command] `npm view electron-builder scripts.postinstall` returned no postinstall script output. [VERIFIED: npm registry]

## Architecture Patterns

### System Architecture Diagram

```text
Developer command
  -> pnpm desktop build
  -> clean dist + napi native build + Vite main/preload/renderer build
  -> electron-builder directory package
  -> packaged executable/resources
  -> Electron main
      -> file:// renderer load
      -> preload bridge
      -> nativeBinding resolver
          -> dev path | unpacked native path | explicit VE_NATIVE_BINDING_PATH
          -> Rust binding executeCommand
              -> importMaterial/list/update draft commands
              -> preview_service via media_runtime
              -> export registry via media_runtime + ffprobe validation
              -> probeRuntimeCapabilities
  -> Playwright packaged/no-mock tests
      -> UI import/edit/preview/export
      -> poll export completion
      -> validate output file metadata
  -> release docs gate
      -> FFmpeg external-runtime manifest
      -> third-party notices
      -> MVP known limits/backlog
```

### Recommended Project Structure

```text
apps/desktop-electron/
├── electron-builder.yml          # package files, asar, asarUnpack, artifact dirs [RECOMMENDED]
├── tests/
│   ├── packaged-smoke.spec.ts    # launches packaged executable/resource tree [RECOMMENDED]
│   └── real-workflow.spec.ts     # no-mock import/edit/preview/export smoke [RECOMMENDED]
└── src/main/
    └── nativeBinding.ts          # dev/unpacked/packaged resolver [VERIFIED]

crates/
├── media_runtime/                # RuntimeCapabilityReport types/probes [RECOMMENDED]
├── bindings_node/                # generated command route for capability report [RECOMMENDED]
└── testkit/                      # deterministic media helpers and validation reuse [VERIFIED]

docs/
├── release-ffmpeg-manifest.md    # external-runtime MVP posture [RECOMMENDED]
├── third-party-notices.md        # Electron/Rust/FFmpeg posture summary [RECOMMENDED]
└── mvp-known-limits.md           # external FFmpeg, signing, advanced semantics backlog [RECOMMENDED]

scripts/
└── phase6-release-guards.sh      # docs/source/package artifact checks [RECOMMENDED]
```

### Pattern 1: Package Native Binding Outside ASAR

**What:** Configure packaging so the native `.node` binary remains a real filesystem file, then resolve it from development, unpacked package resources, or `VE_NATIVE_BINDING_PATH`. [CITED: https://www.electronjs.org/docs/latest/tutorial/asar-archives; VERIFIED: apps/desktop-electron/src/main/nativeBinding.ts]

**When to use:** Always for this Node-API binding; Electron ASAR docs state native/shared-library style files can be left unpacked and shipped beside `app.asar`. [CITED: https://www.electronjs.org/docs/latest/tutorial/asar-archives]

**Example:**

```yaml
# Source: Electron ASAR docs + electron-builder package metadata.
asar: true
asarUnpack:
  - "native/**"
files:
  - "dist/main/**"
  - "dist/preload/**"
  - "dist/renderer/**"
  - "native/**"
```

### Pattern 2: Runtime Capability Report Is Rust-Owned

**What:** Add a command such as `probeRuntimeCapabilities` or extend `probeMediaRuntime` to return discovered binary paths, version/configure line, encoder/filter readiness, deterministic font readiness, and external-runtime/license posture. [VERIFIED: 06-CONTEXT.md; VERIFIED: crates/media_runtime/src/discovery.rs; VERIFIED: crates/testkit/src/render_compare.rs]

**When to use:** Before enabling real preview/export UI in packaged smoke, because missing runtime features must disable affected actions with Chinese copy instead of crashing. [VERIFIED: 06-UI-SPEC.md]

**Example:**

```rust
// Source: derived from media_runtime::RuntimeConfig and testkit::render_compare probes.
pub struct RuntimeCapabilityReport {
    pub runtime: RuntimeConfig,
    pub ffmpeg_configure_summary: String,
    pub supports_h264_encoder: bool,
    pub supports_aac_encoder: bool,
    pub supports_ass_filter: bool,
    pub supports_subtitles_filter: bool,
    pub deterministic_font_ready: bool,
    pub redistributable_build: bool,
}
```

### Pattern 3: Packaged E2E Launches the Packaged Executable

**What:** Build a directory package first, locate the packaged executable, then launch it through Playwright with `executablePath` or the platform-specific packaged app path and pass `VE_FFMPEG_PATH`/`VE_FFPROBE_PATH` in env. [CITED: https://playwright.dev/docs/api/class-electron; VERIFIED: 06-CONTEXT.md]

**When to use:** TEST-07; existing tests launching `dist/main/index.cjs` do not satisfy packaged launch proof. [VERIFIED: apps/desktop-electron/tests/electron-smoke.spec.ts; VERIFIED: 06-CONTEXT.md]

### Anti-Patterns to Avoid

- **Renderer-owned runtime fallback:** Do not let React inspect PATH, build FFmpeg commands, validate output metadata, or infer render graph behavior. [VERIFIED: AGENTS.md; VERIFIED: scripts/phase5-source-guards.sh]
- **Bundled FFmpeg without legal artifacts:** Do not copy FFmpeg into `resources` unless the same plan creates a build manifest, third-party notices, and tests packaged resource resolution. [VERIFIED: 06-CONTEXT.md; CITED: https://ffmpeg.org/legal.html]
- **Testing `dist/main/index.cjs` as “packaged”:** The existing Electron smoke path is a dev build path and misses ASAR/unpacked/resource-path failures. [VERIFIED: apps/desktop-electron/tests/electron-smoke.spec.ts; VERIFIED: 06-CONTEXT.md]
- **Dirty Vite output packaging:** Current Vite config has `emptyOutDir: false`; packaging must clean output first or stale renderer assets can be copied. [VERIFIED: apps/desktop-electron/vite.config.ts; VERIFIED: 06-CONTEXT.md]

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Electron directory package | Custom copy script that approximates app layout | `electron-builder` directory package | Standard packager handles Electron app structure and native resource layout better than ad hoc copying. [VERIFIED: 06-CONTEXT.md; VERIFIED: npm registry] |
| Native module archive behavior | Custom ASAR extractor | ASAR unpack configuration and explicit resolver paths | Electron documents ASAR execution/stat/path limitations and supports unpacked files. [CITED: https://www.electronjs.org/docs/latest/tutorial/asar-archives] |
| Runtime capability probing | Renderer or Playwright parsing FFmpeg output | Rust `media_runtime` plus promoted `testkit::render_compare` probes | Keeps runtime ownership in Rust and avoids test-only assumptions leaking into UI. [VERIFIED: crates/media_runtime/src/lib.rs; VERIFIED: crates/testkit/src/render_compare.rs] |
| Output verification | Playwright-only file existence checks | Existing `validate_rendered_output` / ffprobe metadata validation | Existing Rust validation checks duration, frame rate, resolution, audio, file presence, and non-empty output. [VERIFIED: crates/media_runtime/src/validate.rs] |
| FFmpeg licensing summary | Generic “uses FFmpeg” text | Manifest generated from actual `ffmpeg -version` configure output plus official checklist | FFmpeg legal posture depends on build flags and external libraries. [CITED: https://ffmpeg.org/legal.html; VERIFIED: local command] |

**Key insight:** Phase 6 is about proving the packaged shell still reaches the same Rust-owned draft/material/preview/export pipeline; custom renderer shortcuts would make the test pass while weakening the product boundary. [VERIFIED: AGENTS.md; VERIFIED: 06-CONTEXT.md]

## Common Pitfalls

### Pitfall 1: Native Binding Hidden Inside ASAR

**What goes wrong:** The packaged app launches but `require()` fails to load the `.node` binding or a process API receives a virtual archive path. [CITED: https://www.electronjs.org/docs/latest/tutorial/asar-archives; VERIFIED: apps/desktop-electron/src/main/nativeBinding.ts]
**Why it happens:** Current resolver only checks `../../native/index.cjs` or `../native/index.cjs`; packaged ASAR/unpacked paths are not represented. [VERIFIED: apps/desktop-electron/src/main/nativeBinding.ts]
**How to avoid:** Add explicit packaged path candidates using Electron resource paths and `app.asar.unpacked` layout, while keeping `VE_NATIVE_BINDING_PATH` as a test override. [CITED: https://www.electronjs.org/docs/latest/tutorial/asar-archives; VERIFIED: 06-CONTEXT.md]
**Warning signs:** `Native binding failed to load` appears only in packaged runs; `ping` and `version` fail while renderer loads. [VERIFIED: apps/desktop-electron/src/main/nativeBinding.ts]

### Pitfall 2: External FFmpeg Treated as Redistributable

**What goes wrong:** Release docs imply Homebrew/local FFmpeg is a project-shipped build even though local `ffmpeg -version` includes `--enable-gpl` and `--enable-libx264`. [VERIFIED: local command]
**Why it happens:** Development tests need libx264/AAC/ASS/subtitles, but redistribution obligations depend on actual build flags and libraries. [VERIFIED: crates/testkit/src/render_compare.rs; CITED: https://ffmpeg.org/legal.html]
**How to avoid:** MVP release manifest should say FFmpeg is external/user-provided; bundled FFmpeg remains deferred unless legal artifacts and resource resolver are implemented together. [VERIFIED: 06-CONTEXT.md]
**Warning signs:** Docs say “bundled FFmpeg” without a configure line, source/offers/notices, or a packaged resource test. [CITED: https://ffmpeg.org/legal.html]

### Pitfall 3: Mock E2E Masks Runtime Failures

**What goes wrong:** Workspace tests pass with `VIDEO_EDITOR_TEST_MOCK_PREVIEW_COMMANDS=1` and `VIDEO_EDITOR_TEST_MOCK_EXPORT_COMMANDS=1`, while real preview/export fails. [VERIFIED: apps/desktop-electron/tests/workspace.spec.ts]
**Why it happens:** Existing tests intentionally mock preview/export to stabilize UI assertions. [VERIFIED: apps/desktop-electron/src/main/index.ts; VERIFIED: apps/desktop-electron/tests/workspace.spec.ts]
**How to avoid:** Add a separate no-mock spec with generated temporary media and output validation; keep existing mock tests for UI/layout. [VERIFIED: 06-CONTEXT.md]
**Warning signs:** Phase 6 test command only runs `test:workspace` or never disables mock env flags. [VERIFIED: apps/desktop-electron/tests/workspace.spec.ts]

### Pitfall 4: Stale Build Assets Packaged

**What goes wrong:** Packaged artifact includes stale `dist/renderer/assets` files from earlier Vite builds. [VERIFIED: 06-CONTEXT.md]
**Why it happens:** Current main/preload/renderer Vite builds use `emptyOutDir: false`. [VERIFIED: apps/desktop-electron/vite.config.ts]
**How to avoid:** Add an explicit clean step for `dist`, `out/release`, and package output before packaging. [VERIFIED: 06-CONTEXT.md]
**Warning signs:** Packaged UI shows old labels/assets or package contents include unexpected stale files. [ASSUMED]

## Code Examples

### Packaged Native Binding Resolver

```ts
// Source: apps/desktop-electron/src/main/nativeBinding.ts + Electron ASAR docs.
const candidates = [
  process.env.VE_NATIVE_BINDING_PATH,
  join(__dirname, "../../native/index.cjs"),
  join(__dirname, "../native/index.cjs"),
  app.isPackaged ? join(process.resourcesPath, "app.asar.unpacked/native/index.cjs") : null,
  app.isPackaged ? join(process.resourcesPath, "native/index.cjs") : null
].filter(Boolean);
```

### Packaged Smoke Assertions

```ts
// Source: existing electron-smoke.spec.ts and Playwright Electron launch docs.
const app = await electron.launch({
  executablePath: packagedExecutable,
  env: {
    ...process.env,
    VE_FFMPEG_PATH: ffmpegPath,
    VE_FFPROBE_PATH: ffprobePath
  }
});
const page = await app.firstWindow();
await expect(page).toHaveURL(/file:/);
await expect(page.getByRole("main", { name: "剪映风格编辑工作区" })).toBeVisible();
expect(await page.evaluate(() => window.videoEditorCore?.ping())).toMatchObject({ ok: true });
```

### Runtime Capability Probe Shape

```rust
// Source: media_runtime::discover_runtime_config and testkit::probe_phase5_render_capabilities.
let runtime = discover_runtime_config()?;
let executor = DesktopFfmpegExecutor::default();
let capabilities = probe_runtime_render_capabilities(&executor, &runtime)?;
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Launch `dist/main/index.cjs` for Electron smoke | Launch packaged executable/resource tree for TEST-07 | Phase 6 planning | Catches ASAR, unpacked native, file renderer, and resource path failures. [VERIFIED: apps/desktop-electron/tests/electron-smoke.spec.ts; VERIFIED: 06-CONTEXT.md] |
| `probeMediaRuntime` discovers only FFmpeg/ffprobe versions | Capability report covers encoders, filters, fonts, and redistribution posture | Phase 6 planning | Preview/export can be disabled with actionable Chinese diagnostics before failing jobs. [VERIFIED: crates/media_runtime/src/discovery.rs; VERIFIED: 06-UI-SPEC.md] |
| Local FFmpeg is enough for dev tests | Release docs distinguish external test runtime from bundled redistributable runtime | Phase 6 planning | Avoids misrepresenting GPL-enabled Homebrew FFmpeg as a redistributable project build. [VERIFIED: local command; CITED: https://ffmpeg.org/legal.html] |

**Deprecated/outdated:**
- Treating mock workspace export as TEST-06 completion is insufficient for Phase 6; TEST-06 needs no-mock import/edit/preview/export output verification. [VERIFIED: .planning/REQUIREMENTS.md; VERIFIED: 06-CONTEXT.md]
- Treating `dist/main/index.cjs` smoke as packaged launch is insufficient for TEST-07. [VERIFIED: apps/desktop-electron/tests/electron-smoke.spec.ts; VERIFIED: 06-CONTEXT.md]

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Exact `electron-builder` directory package command spelling should be verified against the installed CLI during implementation; the desired public script name remains `package:dir`. | Standard Stack / Validation Architecture | Low; implementation can adjust script internals without changing the phase command surface. |
| A2 | Stale packaged assets will be visible through unexpected package contents or old UI labels. | Common Pitfalls | Low; clean output and package-content checks mitigate it. |

## Open Questions

1. **Should Phase 6 bundle FFmpeg or explicitly defer it?**
   - What we know: Context allows external FFmpeg for MVP and forbids silent downloads. [VERIFIED: 06-CONTEXT.md]
   - What's unclear: Whether the product owner wants a bundled binary for the first public release. [ASSUMED]
   - Recommendation: Plan external FFmpeg for Phase 6 and document bundled FFmpeg as a known limit/backlog item. [VERIFIED: 06-CONTEXT.md]

2. **What exact packaged artifact names should be stable in tests?**
   - What we know: Implementation may choose output names if commands are deterministic. [VERIFIED: 06-CONTEXT.md]
   - What's unclear: Final branding/app ID/signing names are not locked. [ASSUMED]
   - Recommendation: Use deterministic test helper discovery by platform plus documented package output directory, not hard-coded marketing artifact names. [ASSUMED]

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|-------------|-----------|---------|----------|
| Node.js | pnpm/Electron build | yes | v24.12.0 | None needed. [VERIFIED: local command] |
| pnpm | package scripts | yes | 10.32.1 | None needed. [VERIFIED: local command] |
| Rust/Cargo | native binding and Rust tests | yes | cargo 1.95.0, rustc 1.95.0 | None needed. [VERIFIED: local command] |
| just | public gates | yes | 1.43.0 observed | Root `pnpm run ...` scripts can be used if just is unavailable. [VERIFIED: local command; VERIFIED: justfile] |
| FFmpeg | preview/export smoke | yes | 8.1 local | Missing runtime should produce Chinese diagnostics and fail no-mock gates. [VERIFIED: local command; VERIFIED: 06-UI-SPEC.md] |
| ffprobe | output validation | yes | 8.1 local | Missing runtime should produce Chinese diagnostics and fail no-mock gates. [VERIFIED: local command; VERIFIED: crates/media_runtime/src/validate.rs] |
| slopcheck | package audit | yes with caveat | 0.6.1; `--json` unsupported | Manual slopcheck text result recorded; planner need not install more packages beyond audited `electron-builder`. [VERIFIED: local command] |

**Missing dependencies with no fallback:** none found for research-time planning. [VERIFIED: local command]

**Missing dependencies with fallback:** Context7 CLI was not available, so official docs and registry/CLI checks were used. [VERIFIED: local command]

## Validation Architecture

### Test Framework

| Property | Value |
|----------|-------|
| Framework | Rust `cargo test`; Playwright Electron via `@playwright/test` 1.61.0; root pnpm/just gates. [VERIFIED: package.json; VERIFIED: apps/desktop-electron/package.json] |
| Config file | `apps/desktop-electron/playwright.config.ts`. [VERIFIED: apps/desktop-electron/playwright.config.ts] |
| Quick run command | `pnpm --filter @video-editor/desktop test:packaged-smoke` after implementation. [RECOMMENDED] |
| Full suite command | `pnpm run test` and `just test` after Phase 6 scripts are chained or documented as slower release gates. [VERIFIED: package.json; VERIFIED: justfile; RECOMMENDED] |

### Phase Requirements -> Test Map

| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|--------------|
| TEST-06 | Dev/no-mock Electron imports generated material, edits timeline, requests preview frame/segment, starts export, polls completion, verifies output metadata. | Electron integration + Rust runtime validation | `pnpm --filter @video-editor/desktop test:real-workflow` | No - Wave 0 create `apps/desktop-electron/tests/real-workflow.spec.ts`. [RECOMMENDED] |
| TEST-07 | Directory-packaged app launches offline from packaged artifact, loads file renderer/preload/native binding, probes runtime, completes same small import-preview-export smoke. | Packaged Electron E2E | `pnpm --filter @video-editor/desktop test:packaged` | No - Wave 0 create `apps/desktop-electron/tests/packaged-smoke.spec.ts`. [RECOMMENDED] |
| D-09 | Runtime capability report covers binary paths, version/configure summary, H.264/AAC, ASS/subtitles, deterministic fonts, and external-runtime posture. | Rust unit + Electron smoke | `cargo test -p media_runtime runtime_capability -- --nocapture && pnpm --filter @video-editor/desktop test:runtime-diagnostics` | No - Wave 0 add Rust report tests and UI/IPC assertions. [RECOMMENDED] |
| D-14 | Known limits, FFmpeg manifest, and third-party notices exist and match external-runtime posture. | Docs/release guard | `bash scripts/phase6-release-guards.sh` | No - Wave 0 create docs and guard. [RECOMMENDED] |

### Sampling Rate

- **Per task commit:** run the focused command for that task: package build smoke, runtime capability Rust test, or no-mock E2E. [RECOMMENDED]
- **Per wave merge:** run `pnpm run test:phase6-packaging` or equivalent plus existing Phase 5 runtime gate. [RECOMMENDED]
- **Phase gate:** run full `pnpm run test` and `just test`, plus any deliberately separated slow packaged release command if not chained. [VERIFIED: package.json; VERIFIED: justfile; RECOMMENDED]

### Wave 0 Gaps

- [ ] `apps/desktop-electron/tests/packaged-smoke.spec.ts` - covers TEST-07 packaged launch, preload, native binding, file renderer, `probeMediaRuntime`. [RECOMMENDED]
- [ ] `apps/desktop-electron/tests/real-workflow.spec.ts` - covers TEST-06 no-mock import/edit/preview/export output validation. [RECOMMENDED]
- [ ] `apps/desktop-electron/electron-builder.yml` or package `build` block - defines files, ASAR, unpacked native binding, output directory. [RECOMMENDED]
- [ ] Runtime capability report Rust types/command route - covers D-09 and UI diagnostics. [RECOMMENDED]
- [ ] `scripts/phase6-release-guards.sh` - checks docs, no silent FFmpeg download, no renderer FFmpeg ownership, package script existence, generated contracts drift. [RECOMMENDED]
- [ ] `docs/release-ffmpeg-manifest.md`, `docs/third-party-notices.md`, `docs/mvp-known-limits.md` - covers release readiness. [RECOMMENDED]

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|------------------|
| V2 Authentication | no | No auth surface in this phase. [VERIFIED: phase scope] |
| V3 Session Management | no | No session surface in this phase. [VERIFIED: phase scope] |
| V4 Access Control | yes | Keep renderer sandbox/context isolation and IPC sender validation; packaged tests must preserve untrusted navigation rejection. [VERIFIED: apps/desktop-electron/src/main/index.ts; VERIFIED: apps/desktop-electron/tests/electron-smoke.spec.ts] |
| V5 Input Validation | yes | Generated command schemas and Rust command payload validation remain the trust boundary; renderer only builds envelopes. [VERIFIED: apps/desktop-electron/src/generated/CommandEnvelope.ts; VERIFIED: crates/bindings_node/src/lib.rs] |
| V6 Cryptography | no | Signing/notarization is deferred/known-limit unless local certs are available; no crypto implementation is planned. [VERIFIED: 06-CONTEXT.md] |

### Known Threat Patterns for Electron Packaging

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Untrusted renderer invokes privileged IPC | Elevation of Privilege | Keep `assertAllowedIpcSender`, context isolation, sandbox, and preload-only bridge; packaged smoke must verify bridge shape. [VERIFIED: apps/desktop-electron/src/main/index.ts; VERIFIED: apps/desktop-electron/tests/electron-smoke.spec.ts] |
| Renderer constructs FFmpeg/process behavior | Tampering / Information Disclosure | Phase 5/6 source guards block renderer FFmpeg/render graph/export validation/process ownership. [VERIFIED: scripts/phase5-source-guards.sh; VERIFIED: AGENTS.md] |
| Native binding path spoofing | Tampering / Elevation of Privilege | Preserve explicit `VE_NATIVE_BINDING_PATH` for tests but prefer deterministic packaged resource candidates and validate loaded API shape. [VERIFIED: apps/desktop-electron/src/main/nativeBinding.ts] |
| Bundled binary license drift | Compliance / Repudiation | External-runtime manifest for MVP; bundled future requires build flags, third-party notices, source/license obligations, and resource tests. [CITED: https://ffmpeg.org/legal.html; VERIFIED: 06-CONTEXT.md] |

## Recommended Plan Split

| Plan | Scope | Key Gates |
|------|-------|-----------|
| 06-01 Packaging and runtime boot | Add `electron-builder`, clean build output, package config, native resolver hardening, packaged launch smoke for file renderer/preload/binding/runtime probe. [VERIFIED: 06-CONTEXT.md] | `pnpm --filter @video-editor/desktop package:dir`; `pnpm --filter @video-editor/desktop test:packaged-smoke`. [RECOMMENDED] |
| 06-02 Runtime capability and no-mock workflow | Add Rust-owned capability report, UI diagnostic states from UI-SPEC, generated command route, no-mock generated-media import/edit/preview/export E2E. [VERIFIED: 06-UI-SPEC.md; VERIFIED: crates/testkit/src/render_compare.rs] | Rust capability tests; `test:real-workflow`; output validation through ffprobe/Rust report. [RECOMMENDED] |
| 06-03 Release docs and gates | Add FFmpeg external-runtime manifest, third-party notices, MVP known limits/backlog, source/release guard, root script/just integration. [VERIFIED: 06-CONTEXT.md; CITED: https://ffmpeg.org/legal.html] | `scripts/phase6-release-guards.sh`; full `pnpm run test`; full `just test`. [RECOMMENDED] |

## Sources

### Primary (HIGH confidence)
- `AGENTS.md` - project architecture, terminology, rendering, testing, and licensing constraints. [VERIFIED: AGENTS.md]
- `06-CONTEXT.md` - locked Phase 6 packaging, FFmpeg, E2E, and release decisions. [VERIFIED: 06-CONTEXT.md]
- `06-UI-SPEC.md` - runtime diagnostics UI placement, Simplified Chinese copy, and accessibility labels. [VERIFIED: 06-UI-SPEC.md]
- `package.json`, `justfile`, `apps/desktop-electron/package.json`, `apps/desktop-electron/vite.config.ts` - current scripts, versions, and build behavior. [VERIFIED: local files]
- `apps/desktop-electron/src/main/index.ts`, `nativeBinding.ts`, `tests/*.spec.ts` - existing Electron launch, IPC, binding, mock/no-mock patterns. [VERIFIED: local files]
- `crates/media_runtime/*`, `crates/media_runtime_desktop/src/lib.rs`, `crates/testkit/src/render_compare.rs`, `crates/testkit/tests/preview_export_parity.rs` - FFmpeg discovery, execution, output validation, and capability probe patterns. [VERIFIED: local files]
- Electron ASAR docs - ASAR limitations and unpacked native files. [CITED: https://www.electronjs.org/docs/latest/tutorial/asar-archives]
- Playwright Electron API docs - `_electron.launch`, args/env/executablePath/offline support. [CITED: https://playwright.dev/docs/api/class-electron]
- FFmpeg legal page - LGPL/GPL/nonfree and checklist obligations. [CITED: https://ffmpeg.org/legal.html]

### Secondary (MEDIUM confidence)
- npm registry metadata for `electron-builder` version, repository, downloads API, and postinstall absence. [VERIFIED: npm registry]
- slopcheck text audit for `electron-builder`. [VERIFIED: slopcheck]

### Tertiary (LOW confidence)
- Exact internal `electron-builder` directory target flag spelling was not fully verified from official docs in this session; verify with installed CLI help during implementation. [ASSUMED]

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH for existing stack and `electron-builder` legitimacy; MEDIUM for exact package CLI syntax. [VERIFIED: local files; VERIFIED: npm registry; ASSUMED]
- Architecture: HIGH because ownership boundaries are explicit in AGENTS, Phase 6 context, and existing crates/tests. [VERIFIED: AGENTS.md; VERIFIED: 06-CONTEXT.md; VERIFIED: local files]
- Pitfalls: HIGH for ASAR/native/runtime/mock/license issues; MEDIUM for stale asset symptoms. [CITED: https://www.electronjs.org/docs/latest/tutorial/asar-archives; CITED: https://ffmpeg.org/legal.html; VERIFIED: local files; ASSUMED]

**Research date:** 2026-06-18
**Valid until:** 2026-07-18 for local architecture; re-check npm/electron-builder/FFmpeg legal details before implementation if delayed more than 30 days. [ASSUMED]
