# Mobile Runtime Contracts

This document is the mobile runtime contract for future Android JNI and
iOS Swift/ObjC adapters. Phase 18 implements the shared Rust runtime, portable C
ABI, generated header, and smoke tests that define the boundary. It does not implement full Android or iOS app shells, mobile UI, platform permission UX,
packaging, store deployment, or mobile product release flows.

## Contract Evidence

The executable contract is:

- `crates/bindings_c/include/video_editor_runtime.h` - generated C ABI header
  for runtime, project, handle, texture, status, and buffer types.
- `crates/bindings_c/tests/mobile_contract_handles.rs` - smoke tests for
  mobile-held opaque handles.
- `cargo test -p bindings_c --test mobile_contract_handles -- --nocapture` -
  required smoke gate for owner, generation, release, device, and cascading
  session close behavior.

The header is generated through `scripts/phase18-abi-drift.sh` with pinned
`cbindgen 0.29.4`; future mobile adapters import the checked-in header and must
not hand-author a parallel ABI surface.

## Boundary Map

| Layer | Owns | Must Not Own |
|-------|------|--------------|
| `editor_runtime` | Runtime sessions, project sessions, export service, handle registry, owner/generation validation, explicit release, cascading close diagnostics | Adapter transport parsing, platform UI, platform permission prompts |
| `bindings_c` | Stable C ABI transport, `repr(C)` status/handle/buffer/texture structs, bounded diagnostic output | Draft semantics, project lifecycle policy, export scheduler policy, handle lifetime metadata |
| Android JNI adapter | Java/Kotlin thread attachment, lifecycle forwarding, C ABI calls, Java object wrappers around opaque handles | Rust resource metadata, fabricated handles, garbage-collection-only release |
| iOS Swift/ObjC adapter | Swift/ObjC C import, ARC wrappers around C handles, security-scoped resource coordination, C ABI calls | Rust resource metadata, fabricated handles, ARC-only release |
| `server_runtime` | Electron-free server entrypoints over shared runtime services | Mobile lifecycle policy, UI permission UX |

The C ABI is the common import layer for Android JNI, Swift/ObjC, server shells,
and any future non-Node adapter. Desktop Node-API remains a separate transport
adapter over the same Rust runtime; mobile adapters must not route ownership
through desktop transport objects.

## C ABI Contract

The C ABI exposes explicit functions over Rust-owned state:

- `ve_runtime_create` creates a runtime session and returns `ve_runtime_t`.
- `ve_runtime_close` closes the runtime session and reports unreleased resources
  with leak diagnostics.
- `ve_project_open` opens a `.veproj` bundle and returns an opaque
  `ve_handle_t` for the project session.
- `ve_handle_acquire`, `ve_handle_retain`, and `ve_handle_release` are the only
  portable retain/release surface for media, frame, texture, and artifact
  handles.
- `ve_texture_handle_resolve` validates backend, adapter ID, device ID, size,
  pixel format, and color metadata before a texture handle may be used.
- `ve_last_error_json` writes structured diagnostics for the runtime session.
- `ve_buffer_free` releases Rust-owned buffers if a later ABI function returns
  allocated memory. Current bounded diagnostic buffers are caller-owned unless a
  function explicitly documents Rust allocation.

All ABI calls return `ve_status_t`. Callers must treat any non-OK status as
fail-closed. They may show diagnostics, retry after reacquiring permission, or
close the affected session, but must not silently fall back to byte copies,
debug artifacts, mock frames, or platform-owned state as product success.

## Android JNI Contract

Future Android adapters call the C ABI through JNI wrappers.

- JNI functions attach only on valid Android runtime threads. Long-running
  export, decode, or render work must remain inside Rust runtime jobs, not on
  the UI thread.
- Java/Kotlin objects may wrap `ve_runtime_t` and `ve_handle_t`, but those
  wrappers are tokens only. Rust remains the authority for owner session,
  generation, reference count, lease, texture metadata, and release state.
- JNI finalizers or cleaners may be defensive leak cleanup, but correctness
  depends on explicit release from app lifecycle code.
- Activity or process background events must forward cancellation or suspension
  intent to Rust-owned sessions before Android can reclaim file, codec, or GPU
  resources.
- Foreground resume must revalidate file handle permissions and texture/device
  identity before reuse. A stale token from a prior device/context generation is
  an error, not a reusable resource.

## Swift/ObjC C Import Contract

Future iOS adapters import `video_editor_runtime.h` directly from Swift or ObjC.

- Swift and ObjC wrappers may provide ARC-managed classes around C handles, but
  ARC is not the lifetime authority. The wrapper must call explicit release
  during deterministic close paths.
- Security-scoped resources and app sandbox bookmarks belong to the iOS shell.
  The Rust runtime receives only valid file paths or file descriptors that the
  shell has permission to use at call time.
- Swift `String`, `Data`, and buffer wrappers must respect the ABI ownership
  contract. Caller-owned output buffers remain valid for the duration of the C
  call. Any Rust-allocated buffer returned by a future function must be released
  with `ve_buffer_free`.
- ObjC and Swift wrappers must keep `ve_handle_t` opaque. They cannot edit
  owner ID, owner generation, handle ID, kind, or generation fields except by
  receiving a fresh token from the C ABI.

## Runtime And Project Session Lifecycle

Runtime sessions are parent owners for project sessions and resource handles.

1. Create a runtime session with `ve_runtime_create`.
2. Open each `.veproj` bundle with `ve_project_open`, producing a project
   session handle.
3. Acquire media, frame, texture, and artifact handles only through runtime
   APIs.
4. Retain a handle only when a second platform object needs to outlive the
   current scope.
5. Release each retained handle explicitly.
6. Close the runtime session with `ve_runtime_close` during app shutdown,
   project close, fatal permission invalidation, or unrecoverable device loss.

`ve_runtime_close` performs cascading close for outstanding resources and writes
diagnostics for leaks. Cascading close is recovery and observability, not a
replacement for explicit release in normal adapter code.

## Background And Foreground Lifecycle

Backgrounding is a lifecycle boundary:

- Pause or cancel active preview/export work that depends on foreground-only
  resources.
- Stop creating new texture imports while the app is backgrounded.
- Flush or release platform objects whose lifecycle is tied to the foreground
  graphics context.
- Keep `.veproj/project.json` as the canonical draft source; derived frames,
  textures, artifacts, and export sidecars remain disposable.

Foregrounding requires revalidation:

- Re-open or revalidate sandboxed media permission before reading materials.
- Recreate device/context-bound texture handles if the platform GPU device or
  surface generation changed.
- Treat stale owner generation, stale handle generation, wrong device, lease
  expiry, and missing permission as typed diagnostics.

## Sandboxed Media Permissions

Mobile shells own platform permission prompts. Rust owns semantic validation and
fail-closed diagnostics once a file is presented to the runtime.

- Android scoped storage and iOS security-scoped resources can be revoked while
  a draft still references a material URI.
- Permission invalidation must be surfaced as a material/file access diagnostic.
  It must not rewrite `.veproj/project.json` or mark fallback media as product
  success.
- When permission revocation occurs, the shell should cancel affected jobs,
  release dependent file handle and texture handle tokens, and request user
  reauthorization through platform UI outside Phase 18.

## File Handle Lifetime

A file handle is valid only for the runtime session, project session, and
permission grant that created it.

- Handles to media files must not outlive their runtime session.
- Handles derived from scoped storage or security-scoped resources must be
  released before the platform grant is relinquished.
- Bundle-relative `.veproj` material paths remain draft semantics. Resolved file
  paths are runtime/export inputs and are not written back as canonical state.
- If the platform invalidates a file handle, Rust must report a typed diagnostic
  and the adapter must reacquire permission before retrying.

## Texture And Device Handles

Texture handles are device-bound resource tokens. They carry:

- backend, such as Metal texture, Core Video pixel buffer, D3D11 texture, or
  D3D12 resource;
- adapter ID and device ID;
- owner runtime session and owner generation;
- handle kind, handle ID, and handle generation;
- width, height, pixel format, color primaries, transfer, matrix, and range.

Before import or presentation, `ve_texture_handle_resolve` validates texture and
device identity against the expected descriptor. Wrong device,
stale generation, expired lease, and metadata mismatch return typed failures.
Mobile adapters must reacquire or recreate the texture path instead of copying
frames to CPU memory as product evidence.

## Memory Ownership

The default ABI pattern is caller-owned output buffers:

- Caller passes `ve_buffer_t { data, capacity }`.
- Rust writes UTF-8 JSON diagnostics and sets `len`.
- If capacity is too small, Rust returns `VE_STATUS_BUFFER_TOO_SMALL`.
- Caller keeps ownership of the memory and may reuse it after the call.

If future ABI calls return Rust-allocated strings or byte buffers, those
functions must document ownership and the caller must free them with
`ve_buffer_free`. Mobile wrappers must not free Rust-owned memory with platform
allocators, and Rust must not retain pointers to caller-owned Java, Swift, or
ObjC memory after the C call returns.

## Cancellation

Cancellation is explicit and fail-closed:

- Backgrounding, project close, permission invalidation, device loss, and user
  cancel actions should cancel affected runtime jobs.
- Server export cancellation uses the shared runtime export service; future
  mobile preview/export cancellation should map to the same Rust-owned job or
  generation cancellation model.
- Stale completions after cancellation must not commit visible state or mark
  export/preview success.
- Cancellation diagnostics may report unsupported or degraded states, but cannot
  satisfy product success evidence.

## Explicit Release And Cascading Close

Every handle acquired by mobile code must be released explicitly:

- Release once for every acquisition or retain.
- Double release returns `VE_STATUS_DOUBLE_RELEASE`.
- Wrong owner returns `VE_STATUS_WRONG_OWNER`.
- Stale generation returns `VE_STATUS_STALE_GENERATION`.
- Wrong device or texture metadata mismatch returns texture-specific diagnostics.

Session close cascades remaining handles and reports leaks. This protects
shutdown and crash-recovery paths, but normal app code must still release
project, media, frame, texture, and artifact handles as soon as they leave use.

## Diagnostics And Unsupported States

Diagnostics are allowed to describe degraded or unsupported runtime conditions:

- permission revoked or stale permission;
- file handle invalid;
- texture device mismatch;
- unsupported texture backend;
- expired lease;
- stale owner generation;
- cancelled job;
- runtime session closed.

Diagnostics must not present fallback, mock, artifact, CPU readback, or DOM
evidence as product success. Future mobile UI may display these diagnostics, but
that UI requires independent product and design review outside Phase 18.

## Out Of Phase 18 Scope

Phase 18 does not ship:

- Android app shell, Activity/Fragment UI, permission prompt UX, Gradle
  packaging, Play Store readiness, or Java/Kotlin product flows.
- iOS app shell, SwiftUI/UIKit editor UI, entitlements, security-scoped bookmark
  UX, Xcode packaging, App Store readiness, or Swift product flows.
- Mobile GPU compositor integration beyond the texture/device handle contract.
- Mobile-specific media picker UX, background task policy, crash reporting,
  cloud synchronization, or remote render service deployment.

Those items build on this contract in later phases. They must preserve the
shared Rust runtime ownership model instead of creating platform-specific draft,
project, export, or handle semantics.
