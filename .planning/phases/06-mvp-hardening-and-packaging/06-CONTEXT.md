# Phase 6: MVP Hardening And Packaging - Context

**Gathered:** 2026-06-18
**Status:** Ready for planning
**Source:** `$gsd-discuss-phase 6 --auto` with local codebase scout and subagent read-only investigations

<domain>
## Phase Boundary

Phase 6 turns the existing development MVP into a release-ready desktop MVP gate. It does not add new editor semantics. It verifies that the Electron desktop app can be built, packaged, launched offline, load the Rust Node-API binding, use the FFmpeg runtime through the existing Rust boundary, and complete a real import/edit/preview/export smoke flow with release documentation and known limits captured.

</domain>

<decisions>
## Implementation Decisions

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

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project Scope And Requirements
- `.planning/PROJECT.md` — Product identity, architecture constraints, terminology, and Phase 6-adjacent decisions.
- `.planning/ROADMAP.md` — Phase 6 goal, success criteria, and planned 06-01 through 06-03 work.
- `.planning/REQUIREMENTS.md` — TEST-06 and TEST-07 traceability plus v1/v2 boundaries.
- `AGENTS.md` — Repository-level GSD, architecture, terminology, time-model, rendering, and testing constraints.

### Runtime And Packaging Boundaries
- `docs/runtime-boundaries.md` — FFmpeg discovery, desktop runtime boundary, and deferred packaged binary/license review rules.
- `apps/desktop-electron/package.json` — Existing desktop build/test scripts and dependencies.
- `apps/desktop-electron/vite.config.ts` — Current Electron/Vite output directories and `emptyOutDir` behavior.
- `apps/desktop-electron/src/main/index.ts` — Packaged renderer loading, IPC sender validation, test command mocks.
- `apps/desktop-electron/src/main/nativeBinding.ts` — Native binding resolution paths that must be hardened for packaged apps.
- `crates/media_runtime/src/discovery.rs` — FFmpeg/ffprobe discovery policy and env var names.
- `crates/media_runtime/src/job.rs` — FFmpeg job execution, progress, timeout, and classified runtime error behavior.
- `crates/media_runtime/src/validate.rs` — Export output validation for file, duration, frame rate, resolution, and audio stream.
- `crates/media_runtime_desktop/src/lib.rs` — Desktop FFmpeg executor implementation boundary.
- `crates/preview_service/src/service.rs` — Preview generation currently defaults to test-like compiler capabilities and needs product capability readiness review.
- `crates/bindings_node/src/preview_export_service.rs` — Binding export registry and background export lifecycle.
- `https://ffmpeg.org/legal.html` — Official FFmpeg license/legal checklist; bundled FFmpeg decisions must be checked against actual build flags and external libraries.

### Existing Tests And Guards
- `apps/desktop-electron/tests/electron-smoke.spec.ts` — Current Electron launch/preload/native binding smoke pattern.
- `apps/desktop-electron/tests/workspace.spec.ts` — Current workspace, preview, export, and layout E2E patterns; mock preview/export behavior lives here and in main process env.
- `scripts/phase4-source-guards.sh` — Renderer command-boundary and UI source guard baseline.
- `scripts/phase5-source-guards.sh` — Renderer render/export/runtime ownership guard baseline.
- `crates/testkit/tests/preview_export_parity.rs` — Existing real FFmpeg shared-path parity gate to reuse for deterministic expectations.
- `package.json` — Root public build/test command surface.
- `justfile` — Root public `just build` and `just test` gate surface.

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `apps/desktop-electron/tests/electron-smoke.spec.ts`: Launches Electron from built artifacts and verifies preload bridge shape, `ping`, `version`, and trusted renderer isolation.
- `apps/desktop-electron/tests/workspace.spec.ts`: Provides helpers for workspace visibility, command call spying, viewport/layout checks, and preview/export UI assertions.
- `crates/testkit`: Generates deterministic media and contains FFmpeg/ffprobe helper patterns for output metadata and preview/export parity.
- `media_runtime::discover_runtime_config`: Existing Rust-owned FFmpeg/ffprobe discovery that packaged smoke should call through commands rather than duplicating in Electron.
- `crates/testkit/src/render_compare.rs`: Existing capability-probe helpers for encoders, filters, text rendering, RGB frame extraction, and FFmpeg/ffprobe diagnostics; useful source for a product `RuntimeCapabilityReport`.

### Established Patterns
- Root commands chain named phase gates into `pnpm run test` and `just test`.
- Renderer tests use env flags to record or mock `executeCommand` calls; Phase 6 should add a no-mock path rather than weakening existing mocks.
- Source guards block renderer ownership of FFmpeg, render graph, export scripts, preview cache semantics, and draft/timeline mutation.
- User-visible desktop copy is Simplified Chinese.

### Integration Points
- Desktop packaging integrates at `apps/desktop-electron/package.json`, likely with a new packaging config file or `build` block plus package scripts.
- Native binding packaged loading integrates in `apps/desktop-electron/src/main/nativeBinding.ts`.
- Packaged app launch tests integrate with Playwright under `apps/desktop-electron/tests/` and should be exposed through root scripts.
- Release readiness docs likely belong under `docs/` and should be checked by a Phase 6 release gate script.

</code_context>

<specifics>
## Specific Ideas

- Keep Phase 6 as hardening, not feature expansion. Project canvas, transform/compositing, text, keyframes, retiming, effects, and transitions start in Phases 7-13.
- Use small generated media for real E2E so tests stay deterministic and fast.
- Treat FFmpeg distribution as a decision with artifacts. External-runtime MVP is acceptable; bundled-runtime MVP must include manifest/notices and resource path tests.
- Homebrew FFmpeg 8.1 with `--enable-gpl` was observed locally by the runtime investigation and is suitable for tests only, not a redistributable project binary.
- Keep compact Jianying-style UI baseline from Phase 04.1 and Phase 05, including dark scrollbars and no duplicate left primary menu.

</specifics>

<deferred>
## Deferred Ideas

- Code signing, notarization, installer polish, auto-update, and app icon polish may be documented as known limits unless they are cheap and non-blocking.
- Bundled FFmpeg binaries can be deferred if external FFmpeg smoke is documented and tested.
- Advanced editor semantics remain in Phases 7-13 and should not be pulled into Phase 6.
- Jianying/CapCut/Kaipai adapters remain post-MVP and must not drive Phase 6 packaging scope.

</deferred>

---

*Phase: 6-MVP Hardening And Packaging*
*Context gathered: 2026-06-18*
