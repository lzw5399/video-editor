# Pitfalls Research

## High-Risk Pitfalls

### Duplicate State

Kdenlive's file-format history shows the risk of storing editor data and render data separately. The MVP must keep one canonical `project.json`; render artifacts are regenerated.

Prevention:

- Golden save/open round-trip tests.
- Render graph generated from project state.
- No persisted FFmpeg command strings as semantic truth.

### UI Owning Semantics

If Electron timeline code mutates draft state directly, cross-platform engine semantics are lost.

Prevention:

- UI emits commands only.
- Command tests cover every edit.
- IPC contract tests verify UI cannot bypass core command API.

### Terminology Drift

Inventing internal names that diverge from Jianying terms will make UI, docs, and adapters harder to reason about.

Prevention:

- Use Jianying concepts across UI, Rust domain model, schema, commands, docs, and tests.
- Add terminology checks in docs/reviews when introducing new domain types.

### Time And Locale Bugs

Video editors fail on frame rates, locale decimals, timecode conversion, and float drift.

Prevention:

- Integer microseconds, frame indices, and rational fps.
- Locale-independent serialization.
- Tests for 23.976, 29.97, 30, 59.94, and 60 fps.

### Preview/Export Drift

Preview shortcuts can diverge from export results.

Prevention:

- Preview and export share the same resolved draft/render graph path.
- Preview/export frame parity tests.
- Separate render intent, not separate semantics.

### FFmpeg Leakage

Ad hoc FFmpeg strings in UI or commands create brittle behavior.

Prevention:

- FFmpeg compiler is the only place generating commands.
- Snapshot tests for graph and filter scripts.
- Runtime only executes and reports progress/errors.

### Proprietary Compatibility Expectations

Jianying/CapCut/Kaipai private effects, encrypted drafts, VIP resources, and private IDs cannot be reliable MVP foundations.

Prevention:

- External adapters are post-MVP.
- Compatibility reports list supported, degraded, and unsupported features.
- No promise of 100% pixel parity.

### Licensing Risk

Kdenlive/GPL code and FFmpeg build options can create commercial distribution problems.

Prevention:

- References only; no copied GPL code/assets/presets.
- FFmpeg license/build manifest before release.
- Review GPL/nonfree encoder choices.

