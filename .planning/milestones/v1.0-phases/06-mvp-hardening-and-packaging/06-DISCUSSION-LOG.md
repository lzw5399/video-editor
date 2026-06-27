# Phase 6: MVP Hardening And Packaging - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-18
**Phase:** 6-MVP Hardening And Packaging
**Areas discussed:** Packaging strategy, FFmpeg runtime and license posture, Real MVP E2E, Release readiness
**Mode:** Auto-selected defaults under the user's ongoing instruction to continue autonomously toward Phase 13.

---

## Packaging Strategy

| Option | Description | Selected |
|--------|-------------|----------|
| `electron-builder` directory-package first | Add packaging config/scripts and validate unpacked packaged app before installer/signing work | ✓ |
| Keep Vite-only build | Continue launching `dist/main/index.cjs`; cheaper but does not satisfy packaged smoke | |
| Full signed installer first | Attempt distribution-grade signing/notarization immediately | |

**User's choice:** Auto-selected `electron-builder` directory-package first.
**Notes:** Existing repo has Vite/Electron build output but no packaging dependency/config, no asar/native binding rule, and no packaged app launch smoke.

---

## FFmpeg Runtime And License Posture

| Option | Description | Selected |
|--------|-------------|----------|
| External FFmpeg for MVP package smoke | Use `VE_FFMPEG_PATH`, `VE_FFPROBE_PATH`, or PATH and document this known limit | ✓ |
| Bundle FFmpeg now with manifest/notices | Valid if the same plan creates runtime resolver, license manifest, and notices | |
| Product capability report | Promote runtime capability probing into a user/test-visible report before preview/export hardening closes | ✓ |
| Silent download or unmanaged bundle | Not acceptable for this architecture | |

**User's choice:** Auto-selected external FFmpeg for MVP package smoke plus a product capability report, with a hard rule that any bundled binary requires manifest/notices in the same phase.
**Notes:** `docs/runtime-boundaries.md` explicitly deferred packaged FFmpeg management and license review to later release work. Runtime investigation found local Homebrew FFmpeg 8.1 built with `--enable-gpl`, which is acceptable for dev tests but not a project redistribution plan.

---

## Real MVP E2E

| Option | Description | Selected |
|--------|-------------|----------|
| Add no-mock Electron E2E | Generate media, import/edit/preview/export through UI commands, validate output | ✓ |
| Rely on current mock workspace tests | Keeps UI stable but does not prove real runtime wiring | |
| Rust-only parity gate | Already exists; insufficient for packaged desktop workflow | |

**User's choice:** Auto-selected no-mock Electron E2E.
**Notes:** Current workspace preview/export success tests use main-process mocks. Phase 5 already proves real Rust shared-path parity; Phase 6 should connect that through the desktop app.

---

## Release Readiness

| Option | Description | Selected |
|--------|-------------|----------|
| Known limits + release gates | Document MVP limitations and add executable checks to public gates | ✓ |
| Feature expansion before release docs | Would blur Phase 6 with Phases 7-13 | |
| Manual checklist only | Easier initially but not aligned with phase gate discipline | |

**User's choice:** Auto-selected known limits plus release gates.
**Notes:** Release docs should include external FFmpeg dependency, packaging/signing limits, unsupported advanced semantics, and deferred compatibility/mobile/server scope.

---

## the agent's Discretion

- Exact package output names, test helper file layout, and release directory names can follow implementation patterns discovered during planning.
- Signing/notarization can remain a documented known limit if local certificates/tooling are unavailable.

## Deferred Ideas

- Code signing, notarization, auto-update, installer polish, bundled FFmpeg, app icon polish, and cross-platform package variants can follow once MVP package smoke is stable.
