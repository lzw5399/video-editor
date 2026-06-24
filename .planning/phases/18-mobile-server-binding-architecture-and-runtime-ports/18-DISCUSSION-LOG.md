# Phase 18: Mobile/Server Binding Architecture And Runtime Ports - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-06-25T07:16:00+08:00
**Phase:** 18-mobile-server-binding-architecture-and-runtime-ports
**Mode:** `--auto`
**Areas discussed:** Binding ownership, handle lifecycle, low-copy media boundary, C ABI/mobile contracts, server runtime, verification gates

---

## Binding Ownership

| Option | Description | Selected |
|--------|-------------|----------|
| Shared Rust runtime API under adapters | Move semantic ownership below Node/C/server adapters; adapters only translate transport and platform resources. | ✓ |
| Extend `bindings_node` and mirror later | Keep desktop Node-API as the primary implementation and add C/server wrappers later. | |
| Preserve existing boundaries for compatibility | Avoid breaking old paths and add compatibility layers. | |

**User's choice:** Auto-selected shared Rust runtime API. This follows the user's production-grade, destructive-refactor, no compatibility-first instruction.
**Notes:** Existing `bindings_node` has useful APIs but should not remain the semantic owner if Phase 18 needs portable runtime surfaces.

---

## Handle Lifecycle

| Option | Description | Selected |
|--------|-------------|----------|
| Rust-owned opaque resource registry | Opaque IDs with owner session, generation, explicit release, cascading close, and leak diagnostics. | ✓ |
| Language-owned object lifetime | Let JS/C/JNI/Swift object lifetimes imply release. | |
| Best-effort cleanup without strict owner checks | Permit stale/wrong-owner cases to be tolerated for compatibility. | |

**User's choice:** Auto-selected Rust-owned opaque registry.
**Notes:** This aligns with existing `FramePool`, `TextureHandle`, `NativeTextureLeaseRegistry`, and Phase 17.1 session-generation patterns.

---

## Low-Copy Media Boundary

| Option | Description | Selected |
|--------|-------------|----------|
| Handle-first frame/texture path | Use handles whenever supported; raw bytes only for explicit diagnostics/tests or typed unsupported paths. | ✓ |
| Always serialize frames across FFI | Simple but structurally wrong for 4K preview/export and mobile/server GPU paths. | |
| Fallback to artifacts when handles fail | Disallowed as product success by project policy. | |

**User's choice:** Auto-selected handle-first path.
**Notes:** Native device identity and generation checks must remain explicit.

---

## C ABI And Mobile Contracts

| Option | Description | Selected |
|--------|-------------|----------|
| Contract-first C ABI plus JNI/Swift docs/smoke tests | Build C ABI as real portable boundary; represent JNI/Swift contracts without full mobile apps. | ✓ |
| Build full mobile app shells now | Too broad for Phase 18. | |
| Leave mobile as prose only | Too weak for PLAT/BIND requirements. | |

**User's choice:** Auto-selected contract-first ABI.
**Notes:** Full app productization remains deferred; Phase 18 still needs executable smoke-level contract checks.

---

## Server Runtime

| Option | Description | Selected |
|--------|-------------|----------|
| Real Rust server runtime path | Open `.veproj`, resolve materials, run export/render jobs, and report progress without Electron. | ✓ |
| Metadata-only CLI | Parses projects but does not render/export; insufficient for Phase 18 success. | |
| Electron-driven server wrapper | Reuses desktop shell but violates server boundary. | |

**User's choice:** Auto-selected real Rust server runtime.
**Notes:** Server runtime must use shared `project_store`, render/export, media runtime, and scheduler paths.

---

## Verification Gates

| Option | Description | Selected |
|--------|-------------|----------|
| Broad production gates | Rust tests, C ABI smoke, server export smoke, Node desktop smoke, source guards, contract drift checks. | ✓ |
| Unit-only gates | Easier but too weak for portable runtime acceptance. | |
| Existing desktop gates only | Misses C/server/mobile contract drift. | |

**User's choice:** Auto-selected broad production gates.
**Notes:** Any UI changes should get independent UI/design review per user instruction.

---

## the agent's Discretion

- Exact crate names and wave breakdown can be chosen by researcher/planner, provided semantic ownership is shared Rust-first and adapters remain thin.
- Planner may increase plan count if needed for production-grade execution and verification.

## Deferred Ideas

- Full mobile apps.
- Cloud service deployment and multi-tenant render queue.
- Production effects/retiming/transitions, which remain Phase 19.
