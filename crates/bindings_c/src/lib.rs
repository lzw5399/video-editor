//! Portable C ABI adapter over `editor_runtime`.
//!
//! This crate owns C transport validation only: raw pointers, UTF-8, bounded
//! diagnostic buffers, and stable C status values. Runtime/session/handle
//! semantics stay in `editor_runtime`.

use std::ffi::CStr;
use std::os::raw::c_char;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use editor_runtime::{
    EDITOR_RUNTIME_CONTRACT_VERSION, HandleAcquireRequest, HandleKind, HandleRegistry,
    HandleReleaseState, HandleToken, ProjectSessionService, RuntimeError, RuntimeErrorKind,
    RuntimeSession, RuntimeSessionConfig, RuntimeSessionRegistry, TextureHandleDescriptor,
    TextureResolveExpectation,
};
use media_runtime::{
    ColorMatrix, ColorPrimaries, ColorRange, ColorTransfer, FrameDimensions, RuntimeDeviceId,
    TextureBackend, VideoColorMetadata, VideoPixelFormat,
};
use serde::Serialize;

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ve_status_t {
    VE_STATUS_OK = 0,
    VE_STATUS_INVALID_ARGUMENT = 1,
    VE_STATUS_BUFFER_TOO_SMALL = 2,
    VE_STATUS_INVALID_UTF8 = 3,
    VE_STATUS_INVALID_REQUEST = 4,
    VE_STATUS_UNKNOWN_RUNTIME_SESSION = 5,
    VE_STATUS_UNKNOWN_PROJECT_SESSION = 6,
    VE_STATUS_UNKNOWN_HANDLE = 7,
    VE_STATUS_WRONG_KIND = 8,
    VE_STATUS_WRONG_OWNER = 9,
    VE_STATUS_WRONG_DEVICE = 10,
    VE_STATUS_TEXTURE_METADATA_MISMATCH = 11,
    VE_STATUS_STALE_GENERATION = 12,
    VE_STATUS_LEASE_EXPIRED = 13,
    VE_STATUS_DOUBLE_RELEASE = 14,
    VE_STATUS_PROJECT_STORE = 15,
    VE_STATUS_SCHEDULER = 16,
    VE_STATUS_PANIC = 255,
}

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ve_error_code_t {
    VE_ERROR_NONE = 0,
    VE_ERROR_INVALID_ARGUMENT = 1,
    VE_ERROR_BUFFER_TOO_SMALL = 2,
    VE_ERROR_INVALID_UTF8 = 3,
    VE_ERROR_INVALID_REQUEST = 4,
    VE_ERROR_UNKNOWN_RUNTIME_SESSION = 5,
    VE_ERROR_UNKNOWN_PROJECT_SESSION = 6,
    VE_ERROR_UNKNOWN_HANDLE = 7,
    VE_ERROR_WRONG_KIND = 8,
    VE_ERROR_WRONG_OWNER = 9,
    VE_ERROR_WRONG_DEVICE = 10,
    VE_ERROR_TEXTURE_METADATA_MISMATCH = 11,
    VE_ERROR_STALE_GENERATION = 12,
    VE_ERROR_LEASE_EXPIRED = 13,
    VE_ERROR_DOUBLE_RELEASE = 14,
    VE_ERROR_PROJECT_STORE = 15,
    VE_ERROR_SCHEDULER = 16,
    VE_ERROR_PANIC = 255,
}

#[allow(non_camel_case_types)]
pub type ve_handle_kind_t = u32;
pub const VE_HANDLE_KIND_INVALID: ve_handle_kind_t = 0;
pub const VE_HANDLE_KIND_RUNTIME_SESSION: ve_handle_kind_t = 1;
pub const VE_HANDLE_KIND_PROJECT_SESSION: ve_handle_kind_t = 2;
pub const VE_HANDLE_KIND_MEDIA: ve_handle_kind_t = 3;
pub const VE_HANDLE_KIND_FRAME: ve_handle_kind_t = 4;
pub const VE_HANDLE_KIND_TEXTURE: ve_handle_kind_t = 5;
pub const VE_HANDLE_KIND_ARTIFACT: ve_handle_kind_t = 6;

#[allow(non_camel_case_types)]
pub type ve_texture_backend_t = u32;
pub const VE_TEXTURE_BACKEND_D3D11_TEXTURE_2D: ve_texture_backend_t = 1;
pub const VE_TEXTURE_BACKEND_D3D12_RESOURCE: ve_texture_backend_t = 2;
pub const VE_TEXTURE_BACKEND_METAL_TEXTURE: ve_texture_backend_t = 3;
pub const VE_TEXTURE_BACKEND_CORE_VIDEO_PIXEL_BUFFER: ve_texture_backend_t = 4;

#[allow(non_camel_case_types)]
pub type ve_pixel_format_t = u32;
pub const VE_PIXEL_FORMAT_NV12: ve_pixel_format_t = 1;
pub const VE_PIXEL_FORMAT_BGRA8: ve_pixel_format_t = 2;
pub const VE_PIXEL_FORMAT_RGBA8: ve_pixel_format_t = 3;
pub const VE_PIXEL_FORMAT_P010: ve_pixel_format_t = 4;
pub const VE_PIXEL_FORMAT_YUV420P: ve_pixel_format_t = 5;
pub const VE_PIXEL_FORMAT_UNKNOWN: ve_pixel_format_t = 255;

#[allow(non_camel_case_types)]
pub type ve_color_primaries_t = u32;
pub const VE_COLOR_PRIMARIES_BT709: ve_color_primaries_t = 1;
pub const VE_COLOR_PRIMARIES_BT2020: ve_color_primaries_t = 2;
pub const VE_COLOR_PRIMARIES_DISPLAY_P3: ve_color_primaries_t = 3;
pub const VE_COLOR_PRIMARIES_UNKNOWN: ve_color_primaries_t = 255;

#[allow(non_camel_case_types)]
pub type ve_color_transfer_t = u32;
pub const VE_COLOR_TRANSFER_BT709: ve_color_transfer_t = 1;
pub const VE_COLOR_TRANSFER_SRGB: ve_color_transfer_t = 2;
pub const VE_COLOR_TRANSFER_PQ: ve_color_transfer_t = 3;
pub const VE_COLOR_TRANSFER_HLG: ve_color_transfer_t = 4;
pub const VE_COLOR_TRANSFER_UNKNOWN: ve_color_transfer_t = 255;

#[allow(non_camel_case_types)]
pub type ve_color_matrix_t = u32;
pub const VE_COLOR_MATRIX_BT709: ve_color_matrix_t = 1;
pub const VE_COLOR_MATRIX_BT2020_NON_CONSTANT: ve_color_matrix_t = 2;
pub const VE_COLOR_MATRIX_IDENTITY: ve_color_matrix_t = 3;
pub const VE_COLOR_MATRIX_UNKNOWN: ve_color_matrix_t = 255;

#[allow(non_camel_case_types)]
pub type ve_color_range_t = u32;
pub const VE_COLOR_RANGE_LIMITED: ve_color_range_t = 1;
pub const VE_COLOR_RANGE_FULL: ve_color_range_t = 2;
pub const VE_COLOR_RANGE_UNKNOWN: ve_color_range_t = 255;

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ve_runtime_config_t {
    pub diagnostic_label: *const c_char,
}

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ve_runtime_t {
    pub id: u64,
    pub generation: u64,
}

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ve_handle_t {
    pub kind: ve_handle_kind_t,
    pub id: u64,
    pub owner_id: u64,
    pub owner_generation: u64,
    pub generation: u64,
}

impl Default for ve_handle_t {
    fn default() -> Self {
        Self {
            kind: VE_HANDLE_KIND_INVALID,
            id: 0,
            owner_id: 0,
            owner_generation: 0,
            generation: 0,
        }
    }
}

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ve_buffer_t {
    pub data: *mut u8,
    pub len: usize,
    pub capacity: usize,
}

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ve_texture_descriptor_t {
    pub backend: ve_texture_backend_t,
    pub adapter_id: *const c_char,
    pub device_id: *const c_char,
    pub width: u32,
    pub height: u32,
    pub pixel_format: ve_pixel_format_t,
    pub color_primaries: ve_color_primaries_t,
    pub color_transfer: ve_color_transfer_t,
    pub color_matrix: ve_color_matrix_t,
    pub color_range: ve_color_range_t,
}

pub fn contract_version() -> &'static str {
    EDITOR_RUNTIME_CONTRACT_VERSION
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ve_runtime_create(
    config: *const ve_runtime_config_t,
    out_runtime: *mut ve_runtime_t,
    out_json: *mut ve_buffer_t,
) -> ve_status_t {
    ffi_entry(out_json, || {
        if out_runtime.is_null() {
            return (
                None,
                ve_status_t::VE_STATUS_INVALID_ARGUMENT,
                Diagnostic::error(
                    ve_status_t::VE_STATUS_INVALID_ARGUMENT,
                    "runtime output pointer is null",
                ),
            );
        }
        let label = if config.is_null() {
            None
        } else {
            let config = unsafe { &*config };
            match optional_c_string(config.diagnostic_label) {
                Ok(value) => value,
                Err((status, diagnostic)) => return (None, status, diagnostic),
            }
        };
        let mut runtime_sessions = RuntimeSessionRegistry::default();
        let session = match runtime_sessions.create_session(RuntimeSessionConfig {
            diagnostic_label: label,
        }) {
            Ok(session) => session,
            Err(error) => {
                let status = status_from_runtime_error(&error);
                return (None, status, Diagnostic::from_runtime_error(status, &error));
            }
        };
        let mut state = lock_state();
        let runtime = state.insert_runtime(session);
        unsafe {
            *out_runtime = runtime;
        }
        (
            Some(runtime),
            ve_status_t::VE_STATUS_OK,
            Diagnostic::runtime_created(runtime),
        )
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ve_runtime_close(
    runtime: ve_runtime_t,
    out_json: *mut ve_buffer_t,
) -> ve_status_t {
    ffi_entry(out_json, || {
        let mut state = lock_state();
        match state.close_runtime(runtime) {
            Ok(report) => (
                Some(runtime),
                ve_status_t::VE_STATUS_OK,
                Diagnostic::runtime_closed(runtime, report),
            ),
            Err((status, diagnostic)) => (Some(runtime), status, diagnostic),
        }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ve_project_open(
    runtime: ve_runtime_t,
    bundle_path: *const c_char,
    out_handle: *mut ve_handle_t,
    out_json: *mut ve_buffer_t,
) -> ve_status_t {
    ffi_entry(out_json, || {
        if out_handle.is_null() {
            return (
                Some(runtime),
                ve_status_t::VE_STATUS_INVALID_ARGUMENT,
                Diagnostic::error(
                    ve_status_t::VE_STATUS_INVALID_ARGUMENT,
                    "project handle output pointer is null",
                ),
            );
        }
        let bundle_path = match required_c_string(bundle_path, "project bundle path") {
            Ok(path) => path,
            Err((status, diagnostic)) => return (Some(runtime), status, diagnostic),
        };
        let mut state = lock_state();
        let Some(entry) = state.open_runtime_mut(runtime) else {
            return state.unknown_runtime(runtime);
        };
        let opened = match entry
            .projects
            .open_project_session(entry.session.id.clone(), &bundle_path)
        {
            Ok(opened) => opened,
            Err(error) => {
                let status = status_from_runtime_error(&error);
                return (
                    Some(runtime),
                    status,
                    Diagnostic::from_runtime_error(status, &error),
                );
            }
        };
        let token = match entry.handles.acquire(HandleAcquireRequest::project_session(
            entry.session.id.clone(),
        )) {
            Ok(token) => token,
            Err(error) => {
                let _ = entry.projects.close_project_session(&opened.handle);
                let status = status_from_runtime_error(&error);
                return (
                    Some(runtime),
                    status,
                    Diagnostic::from_runtime_error(status, &error),
                );
            }
        };
        if let Err(error) = entry
            .projects
            .bind_portable_handle(token.clone(), opened.handle.clone())
        {
            let _ = entry.handles.release(&entry.session.id, &token);
            let _ = entry.projects.close_project_session(&opened.handle);
            let status = status_from_runtime_error(&error);
            return (
                Some(runtime),
                status,
                Diagnostic::from_runtime_error(status, &error),
            );
        }
        let handle = handle_from_token(runtime, &token);
        unsafe {
            *out_handle = handle;
        }
        (
            Some(runtime),
            ve_status_t::VE_STATUS_OK,
            Diagnostic::project_opened(runtime, handle, opened.draft_name),
        )
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ve_handle_acquire(
    runtime: ve_runtime_t,
    kind: ve_handle_kind_t,
    texture: *const ve_texture_descriptor_t,
    lease_duration_us: u64,
    out_handle: *mut ve_handle_t,
    out_json: *mut ve_buffer_t,
) -> ve_status_t {
    ffi_entry(out_json, || {
        if out_handle.is_null() {
            return (
                Some(runtime),
                ve_status_t::VE_STATUS_INVALID_ARGUMENT,
                Diagnostic::error(
                    ve_status_t::VE_STATUS_INVALID_ARGUMENT,
                    "handle output pointer is null",
                ),
            );
        }
        let mut state = lock_state();
        let Some(entry) = state.open_runtime_mut(runtime) else {
            return state.unknown_runtime(runtime);
        };
        let request = match acquire_request(
            kind,
            texture,
            &entry.session.id,
            entry.now_us(),
            lease_duration_us,
        ) {
            Ok(request) => request,
            Err((status, diagnostic)) => return (Some(runtime), status, diagnostic),
        };
        match entry.handles.acquire(request) {
            Ok(token) => {
                let handle = handle_from_token(runtime, &token);
                unsafe {
                    *out_handle = handle;
                }
                (
                    Some(runtime),
                    ve_status_t::VE_STATUS_OK,
                    Diagnostic::handle_event("handleAcquired", runtime, handle),
                )
            }
            Err(error) => {
                let status = status_from_runtime_error(&error);
                (
                    Some(runtime),
                    status,
                    Diagnostic::from_runtime_error(status, &error),
                )
            }
        }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ve_handle_retain(
    runtime: ve_runtime_t,
    handle: ve_handle_t,
    out_json: *mut ve_buffer_t,
) -> ve_status_t {
    ffi_entry(out_json, || {
        let mut state = lock_state();
        let Some(entry) = state.open_runtime_mut(runtime) else {
            return state.unknown_runtime(runtime);
        };
        let token = match token_from_handle(runtime, handle, &entry.session.id) {
            Ok(token) => token,
            Err((status, diagnostic)) => return (Some(runtime), status, diagnostic),
        };
        let now_us = entry.now_us();
        match entry.handles.retain(&entry.session.id, &token, now_us) {
            Ok(_) => (
                Some(runtime),
                ve_status_t::VE_STATUS_OK,
                Diagnostic::handle_event("handleRetained", runtime, handle),
            ),
            Err(error) => {
                let status = status_from_runtime_error(&error);
                (
                    Some(runtime),
                    status,
                    Diagnostic::from_runtime_error(status, &error),
                )
            }
        }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ve_handle_release(
    runtime: ve_runtime_t,
    handle: ve_handle_t,
    out_json: *mut ve_buffer_t,
) -> ve_status_t {
    ffi_entry(out_json, || {
        let mut state = lock_state();
        let Some(entry) = state.open_runtime_mut(runtime) else {
            return state.unknown_runtime(runtime);
        };
        let token = match token_from_handle(runtime, handle, &entry.session.id) {
            Ok(token) => token,
            Err((status, diagnostic)) => return (Some(runtime), status, diagnostic),
        };
        match entry.handles.release(&entry.session.id, &token) {
            Ok(report) => {
                if report.kind == HandleKind::ProjectSession && report.remaining_retain_count == 0 {
                    if let Err(error) = entry.projects.close_portable_handle_binding(&token) {
                        let status = status_from_runtime_error(&error);
                        return (
                            Some(runtime),
                            status,
                            Diagnostic::from_runtime_error(status, &error),
                        );
                    }
                }
                (
                    Some(runtime),
                    ve_status_t::VE_STATUS_OK,
                    Diagnostic::handle_released(runtime, handle, report.release_state),
                )
            }
            Err(error) => {
                let status = status_from_runtime_error(&error);
                (
                    Some(runtime),
                    status,
                    Diagnostic::from_runtime_error(status, &error),
                )
            }
        }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ve_texture_handle_resolve(
    runtime: ve_runtime_t,
    handle: ve_handle_t,
    texture: *const ve_texture_descriptor_t,
    out_json: *mut ve_buffer_t,
) -> ve_status_t {
    ffi_entry(out_json, || {
        let descriptor = match texture_descriptor(texture) {
            Ok(descriptor) => descriptor,
            Err((status, diagnostic)) => return (Some(runtime), status, diagnostic),
        };
        let mut state = lock_state();
        let Some(entry) = state.open_runtime_mut(runtime) else {
            return state.unknown_runtime(runtime);
        };
        let token = match token_from_handle(runtime, handle, &entry.session.id) {
            Ok(token) => token,
            Err((status, diagnostic)) => return (Some(runtime), status, diagnostic),
        };
        let now_us = entry.now_us();
        match entry.handles.resolve_texture(
            &entry.session.id,
            &token,
            &TextureResolveExpectation { descriptor },
            now_us,
        ) {
            Ok(_) => (
                Some(runtime),
                ve_status_t::VE_STATUS_OK,
                Diagnostic::handle_event("textureResolved", runtime, handle),
            ),
            Err(error) => {
                let status = status_from_runtime_error(&error);
                (
                    Some(runtime),
                    status,
                    Diagnostic::from_runtime_error(status, &error),
                )
            }
        }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ve_last_error_json(
    runtime: ve_runtime_t,
    out_json: *mut ve_buffer_t,
) -> ve_status_t {
    let diagnostic = {
        let state = lock_state();
        state
            .last_error(runtime)
            .unwrap_or_else(|| Diagnostic::ok("noError", "no error recorded"))
    };
    match write_json(out_json, &diagnostic) {
        Ok(()) => ve_status_t::VE_STATUS_OK,
        Err(status) => {
            record_last_error(
                Some(runtime),
                Diagnostic::error(status, "diagnostic buffer failed"),
            );
            status
        }
    }
}

#[derive(Debug)]
struct AbiState {
    ticket: u64,
    runtimes: Vec<RuntimeSlot>,
    last_error: Option<Diagnostic>,
}

impl Default for AbiState {
    fn default() -> Self {
        Self {
            ticket: 1,
            runtimes: Vec::new(),
            last_error: None,
        }
    }
}

#[derive(Debug)]
enum RuntimeSlot {
    Open(AbiRuntime),
    Closed {
        runtime: ve_runtime_t,
        last_error: Option<Diagnostic>,
    },
}

#[derive(Debug)]
struct AbiRuntime {
    runtime: ve_runtime_t,
    session: RuntimeSession,
    started_at: Instant,
    projects: ProjectSessionService,
    handles: HandleRegistry,
    last_error: Option<Diagnostic>,
}

impl AbiRuntime {
    fn now_us(&self) -> u64 {
        u64::try_from(self.started_at.elapsed().as_micros()).unwrap_or(u64::MAX)
    }
}

impl AbiState {
    fn insert_runtime(&mut self, session: RuntimeSession) -> ve_runtime_t {
        let runtime = ve_runtime_t {
            id: self.ticket,
            generation: session.id.generation(),
        };
        self.ticket = self.ticket.saturating_add(1);
        self.runtimes.push(RuntimeSlot::Open(AbiRuntime {
            runtime,
            session,
            started_at: Instant::now(),
            projects: ProjectSessionService::default(),
            handles: HandleRegistry::default(),
            last_error: None,
        }));
        runtime
    }

    fn open_runtime_mut(&mut self, runtime: ve_runtime_t) -> Option<&mut AbiRuntime> {
        self.runtimes.iter_mut().find_map(|slot| match slot {
            RuntimeSlot::Open(entry)
                if entry.runtime.id == runtime.id
                    && entry.runtime.generation == runtime.generation =>
            {
                Some(entry)
            }
            _ => None,
        })
    }

    fn close_runtime(
        &mut self,
        runtime: ve_runtime_t,
    ) -> Result<Vec<HandleDiagnostic>, (ve_status_t, Diagnostic)> {
        for slot in &mut self.runtimes {
            match slot {
                RuntimeSlot::Open(entry)
                    if entry.runtime.id == runtime.id
                        && entry.runtime.generation == runtime.generation =>
                {
                    let _ = entry.projects.close_all_project_sessions();
                    let report = entry.handles.close_runtime_session(&entry.session.id);
                    let diagnostics = report
                        .leak_diagnostics
                        .iter()
                        .map(|leak| {
                            let mut diagnostic =
                                handle_diagnostic(handle_from_token(runtime, &leak.token));
                            diagnostic.release_state = Some(match leak.release_state {
                                HandleReleaseState::Explicit => "explicit",
                                HandleReleaseState::CascadeClose => "cascadeClose",
                            });
                            diagnostic
                        })
                        .collect::<Vec<_>>();
                    *slot = RuntimeSlot::Closed {
                        runtime,
                        last_error: None,
                    };
                    return Ok(diagnostics);
                }
                RuntimeSlot::Closed {
                    runtime: closed, ..
                } if closed.id == runtime.id && closed.generation == runtime.generation => {
                    return Err((
                        ve_status_t::VE_STATUS_DOUBLE_RELEASE,
                        Diagnostic::error(
                            ve_status_t::VE_STATUS_DOUBLE_RELEASE,
                            "runtime session was already closed",
                        ),
                    ));
                }
                _ => {}
            }
        }
        Err((
            ve_status_t::VE_STATUS_UNKNOWN_RUNTIME_SESSION,
            Diagnostic::error(
                ve_status_t::VE_STATUS_UNKNOWN_RUNTIME_SESSION,
                "runtime session not found",
            ),
        ))
    }

    fn unknown_runtime(
        &mut self,
        runtime: ve_runtime_t,
    ) -> (Option<ve_runtime_t>, ve_status_t, Diagnostic) {
        (
            Some(runtime),
            ve_status_t::VE_STATUS_UNKNOWN_RUNTIME_SESSION,
            Diagnostic::error(
                ve_status_t::VE_STATUS_UNKNOWN_RUNTIME_SESSION,
                "runtime session not found",
            ),
        )
    }

    fn last_error(&self, runtime: ve_runtime_t) -> Option<Diagnostic> {
        self.runtimes
            .iter()
            .find_map(|slot| match slot {
                RuntimeSlot::Open(entry)
                    if entry.runtime.id == runtime.id
                        && entry.runtime.generation == runtime.generation =>
                {
                    entry.last_error.clone()
                }
                RuntimeSlot::Closed {
                    runtime: closed,
                    last_error,
                } if closed.id == runtime.id && closed.generation == runtime.generation => {
                    last_error.clone()
                }
                _ => None,
            })
            .or_else(|| self.last_error.clone())
    }

    fn record_error(&mut self, runtime: Option<ve_runtime_t>, diagnostic: Diagnostic) {
        if let Some(runtime) = runtime {
            for slot in &mut self.runtimes {
                match slot {
                    RuntimeSlot::Open(entry)
                        if entry.runtime.id == runtime.id
                            && entry.runtime.generation == runtime.generation =>
                    {
                        entry.last_error = Some(diagnostic);
                        return;
                    }
                    RuntimeSlot::Closed {
                        runtime: closed,
                        last_error,
                    } if closed.id == runtime.id && closed.generation == runtime.generation => {
                        *last_error = Some(diagnostic);
                        return;
                    }
                    _ => {}
                }
            }
        }
        self.last_error = Some(diagnostic);
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Diagnostic {
    ok: bool,
    code: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    runtime: Option<ve_runtime_t_json>,
    #[serde(skip_serializing_if = "Option::is_none")]
    handle: Option<HandleDiagnostic>,
    #[serde(skip_serializing_if = "Option::is_none")]
    project_session: Option<HandleDiagnostic>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    leaks: Vec<HandleDiagnostic>,
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
struct ve_runtime_t_json {
    id: u64,
    generation: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct HandleDiagnostic {
    kind: &'static str,
    id: u64,
    owner_id: u64,
    owner_generation: u64,
    generation: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    release_state: Option<&'static str>,
}

impl Diagnostic {
    fn ok(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            ok: true,
            code,
            message: message.into(),
            runtime: None,
            handle: None,
            project_session: None,
            leaks: Vec::new(),
        }
    }

    fn error(status: ve_status_t, message: impl Into<String>) -> Self {
        Self {
            ok: false,
            code: status_code(status),
            message: message.into(),
            runtime: None,
            handle: None,
            project_session: None,
            leaks: Vec::new(),
        }
    }

    fn from_runtime_error(status: ve_status_t, error: &RuntimeError) -> Self {
        Self::error(status, &error.message)
    }

    fn runtime_created(runtime: ve_runtime_t) -> Self {
        let mut diagnostic = Self::ok("ok", "runtime session created");
        diagnostic.runtime = Some(runtime_diagnostic(runtime));
        diagnostic
    }

    fn runtime_closed(runtime: ve_runtime_t, leaks: Vec<HandleDiagnostic>) -> Self {
        let mut diagnostic = Self::ok("ok", "runtime session closed");
        diagnostic.runtime = Some(runtime_diagnostic(runtime));
        diagnostic.leaks = leaks;
        diagnostic
    }

    fn project_opened(runtime: ve_runtime_t, handle: ve_handle_t, draft_name: String) -> Self {
        let mut diagnostic = Self::ok(
            "projectOpened",
            format!("project session opened: {draft_name}"),
        );
        diagnostic.runtime = Some(runtime_diagnostic(runtime));
        diagnostic.project_session = Some(handle_diagnostic(handle));
        diagnostic
    }

    fn handle_event(code: &'static str, runtime: ve_runtime_t, handle: ve_handle_t) -> Self {
        let mut diagnostic = Self::ok(code, code);
        diagnostic.runtime = Some(runtime_diagnostic(runtime));
        diagnostic.handle = Some(handle_diagnostic(handle));
        diagnostic
    }

    fn handle_released(
        runtime: ve_runtime_t,
        handle: ve_handle_t,
        release_state: HandleReleaseState,
    ) -> Self {
        let mut handle = handle_diagnostic(handle);
        handle.release_state = Some(match release_state {
            HandleReleaseState::Explicit => "explicit",
            HandleReleaseState::CascadeClose => "cascadeClose",
        });
        let mut diagnostic = Self::ok("handleReleased", "handle released");
        diagnostic.runtime = Some(runtime_diagnostic(runtime));
        diagnostic.handle = Some(handle);
        diagnostic
    }
}

fn ffi_entry<F>(out_json: *mut ve_buffer_t, op: F) -> ve_status_t
where
    F: FnOnce() -> (Option<ve_runtime_t>, ve_status_t, Diagnostic),
{
    match catch_unwind(AssertUnwindSafe(op)) {
        Ok((runtime, status, diagnostic)) => finish(runtime, status, diagnostic, out_json),
        Err(_) => finish(
            None,
            ve_status_t::VE_STATUS_PANIC,
            Diagnostic::error(
                ve_status_t::VE_STATUS_PANIC,
                "panic caught at C ABI boundary",
            ),
            out_json,
        ),
    }
}

fn finish(
    runtime: Option<ve_runtime_t>,
    status: ve_status_t,
    diagnostic: Diagnostic,
    out_json: *mut ve_buffer_t,
) -> ve_status_t {
    if status != ve_status_t::VE_STATUS_OK {
        record_last_error(runtime, diagnostic.clone());
    }
    match write_json(out_json, &diagnostic) {
        Ok(()) => status,
        Err(write_status) => {
            record_last_error(
                runtime,
                Diagnostic::error(write_status, "diagnostic buffer failed"),
            );
            status
        }
    }
}

fn write_json(out_json: *mut ve_buffer_t, diagnostic: &Diagnostic) -> Result<(), ve_status_t> {
    if out_json.is_null() {
        return Ok(());
    }
    let bytes =
        serde_json::to_vec(diagnostic).map_err(|_| ve_status_t::VE_STATUS_INVALID_REQUEST)?;
    let out = unsafe { &mut *out_json };
    if out.data.is_null() || out.capacity == 0 {
        out.len = bytes.len();
        return Err(ve_status_t::VE_STATUS_INVALID_ARGUMENT);
    }
    let required = bytes.len().saturating_add(1);
    if required > out.capacity {
        out.len = bytes.len();
        return Err(ve_status_t::VE_STATUS_BUFFER_TOO_SMALL);
    }
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), out.data, bytes.len());
        *out.data.add(bytes.len()) = 0;
    }
    out.len = bytes.len();
    Ok(())
}

fn record_last_error(runtime: Option<ve_runtime_t>, diagnostic: Diagnostic) {
    lock_state().record_error(runtime, diagnostic);
}

fn lock_state() -> std::sync::MutexGuard<'static, AbiState> {
    abi_state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn abi_state() -> &'static Mutex<AbiState> {
    static STATE: OnceLock<Mutex<AbiState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(AbiState::default()))
}

fn optional_c_string(ptr: *const c_char) -> Result<Option<String>, (ve_status_t, Diagnostic)> {
    if ptr.is_null() {
        return Ok(None);
    }
    required_c_string(ptr, "optional string").map(Some)
}

fn required_c_string(
    ptr: *const c_char,
    name: &'static str,
) -> Result<String, (ve_status_t, Diagnostic)> {
    if ptr.is_null() {
        return Err((
            ve_status_t::VE_STATUS_INVALID_ARGUMENT,
            Diagnostic::error(
                ve_status_t::VE_STATUS_INVALID_ARGUMENT,
                format!("{name} is null"),
            ),
        ));
    }
    let value = unsafe { CStr::from_ptr(ptr) };
    value.to_str().map(str::to_owned).map_err(|_| {
        (
            ve_status_t::VE_STATUS_INVALID_UTF8,
            Diagnostic::error(
                ve_status_t::VE_STATUS_INVALID_UTF8,
                format!("{name} is not valid UTF-8"),
            ),
        )
    })
}

fn acquire_request(
    kind: ve_handle_kind_t,
    texture: *const ve_texture_descriptor_t,
    owner: &editor_runtime::RuntimeSessionId,
    now_us: u64,
    lease_duration_us: u64,
) -> Result<HandleAcquireRequest, (ve_status_t, Diagnostic)> {
    let request = match kind {
        VE_HANDLE_KIND_RUNTIME_SESSION | VE_HANDLE_KIND_PROJECT_SESSION => {
            return Err((
                ve_status_t::VE_STATUS_INVALID_ARGUMENT,
                Diagnostic::error(
                    ve_status_t::VE_STATUS_INVALID_ARGUMENT,
                    "runtime and project session handles are created only by lifecycle APIs",
                ),
            ));
        }
        VE_HANDLE_KIND_MEDIA => HandleAcquireRequest::media(owner.clone()),
        VE_HANDLE_KIND_FRAME => HandleAcquireRequest::frame(owner.clone()),
        VE_HANDLE_KIND_TEXTURE => {
            HandleAcquireRequest::texture(owner.clone(), texture_descriptor(texture)?)
        }
        VE_HANDLE_KIND_ARTIFACT => HandleAcquireRequest::artifact(owner.clone()),
        VE_HANDLE_KIND_INVALID => {
            return Err((
                ve_status_t::VE_STATUS_INVALID_ARGUMENT,
                Diagnostic::error(
                    ve_status_t::VE_STATUS_INVALID_ARGUMENT,
                    "handle kind is invalid",
                ),
            ));
        }
        _ => {
            return Err((
                ve_status_t::VE_STATUS_INVALID_ARGUMENT,
                Diagnostic::error(
                    ve_status_t::VE_STATUS_INVALID_ARGUMENT,
                    "handle kind is invalid",
                ),
            ));
        }
    };
    Ok(if lease_duration_us > 0 {
        request.with_lease_expires_at_us(now_us.saturating_add(lease_duration_us))
    } else {
        request
    })
}

fn texture_descriptor(
    ptr: *const ve_texture_descriptor_t,
) -> Result<TextureHandleDescriptor, (ve_status_t, Diagnostic)> {
    if ptr.is_null() {
        return Err((
            ve_status_t::VE_STATUS_INVALID_ARGUMENT,
            Diagnostic::error(
                ve_status_t::VE_STATUS_INVALID_ARGUMENT,
                "texture descriptor is null",
            ),
        ));
    }
    let texture = unsafe { &*ptr };
    let backend = texture_backend(texture.backend)?;
    let adapter_id = required_c_string(texture.adapter_id, "texture adapter id")?;
    let device_id = required_c_string(texture.device_id, "texture device id")?;
    Ok(TextureHandleDescriptor {
        backend,
        device: RuntimeDeviceId {
            backend,
            adapter_id,
            device_id,
        },
        dimensions: FrameDimensions {
            width: texture.width,
            height: texture.height,
        },
        pixel_format: pixel_format(texture.pixel_format)?,
        color: VideoColorMetadata {
            primaries: color_primaries(texture.color_primaries)?,
            transfer: color_transfer(texture.color_transfer)?,
            matrix: color_matrix(texture.color_matrix)?,
            range: color_range(texture.color_range)?,
            diagnostics: Vec::new(),
        },
    })
}

fn token_from_handle(
    runtime: ve_runtime_t,
    handle: ve_handle_t,
    owner: &editor_runtime::RuntimeSessionId,
) -> Result<HandleToken, (ve_status_t, Diagnostic)> {
    if handle.id == 0 {
        return Err((
            ve_status_t::VE_STATUS_INVALID_ARGUMENT,
            Diagnostic::error(ve_status_t::VE_STATUS_INVALID_ARGUMENT, "handle id is zero"),
        ));
    }
    if handle.owner_id != runtime.id {
        return Err((
            ve_status_t::VE_STATUS_WRONG_OWNER,
            Diagnostic::error(
                ve_status_t::VE_STATUS_WRONG_OWNER,
                "handle owner runtime does not match",
            ),
        ));
    }
    let kind = handle_kind(handle.kind)?;
    let owner_session = owner.with_generation(handle.owner_generation);
    Ok(HandleToken::from_raw_parts(
        kind,
        format!("{}-{}", token_prefix(kind), handle.id),
        owner_session,
        handle.generation,
    ))
}

fn handle_from_token(runtime: ve_runtime_t, token: &HandleToken) -> ve_handle_t {
    ve_handle_t {
        kind: handle_kind_c(token.kind()),
        id: parse_token_id(token.as_str()),
        owner_id: runtime.id,
        owner_generation: token.owner_session().generation(),
        generation: token.generation(),
    }
}

fn handle_diagnostic(handle: ve_handle_t) -> HandleDiagnostic {
    HandleDiagnostic {
        kind: handle_kind_name(handle.kind),
        id: handle.id,
        owner_id: handle.owner_id,
        owner_generation: handle.owner_generation,
        generation: handle.generation,
        release_state: None,
    }
}

fn runtime_diagnostic(runtime: ve_runtime_t) -> ve_runtime_t_json {
    ve_runtime_t_json {
        id: runtime.id,
        generation: runtime.generation,
    }
}

fn parse_token_id(token: &str) -> u64 {
    token
        .rsplit_once('-')
        .and_then(|(_, id)| id.parse::<u64>().ok())
        .unwrap_or(0)
}

fn handle_kind(kind: ve_handle_kind_t) -> Result<HandleKind, (ve_status_t, Diagnostic)> {
    match kind {
        VE_HANDLE_KIND_RUNTIME_SESSION => Ok(HandleKind::RuntimeSession),
        VE_HANDLE_KIND_PROJECT_SESSION => Ok(HandleKind::ProjectSession),
        VE_HANDLE_KIND_MEDIA => Ok(HandleKind::Media),
        VE_HANDLE_KIND_FRAME => Ok(HandleKind::Frame),
        VE_HANDLE_KIND_TEXTURE => Ok(HandleKind::Texture),
        VE_HANDLE_KIND_ARTIFACT => Ok(HandleKind::Artifact),
        VE_HANDLE_KIND_INVALID => Err(invalid_abi_value("handle kind")),
        _ => Err(invalid_abi_value("handle kind")),
    }
}

fn handle_kind_c(kind: HandleKind) -> ve_handle_kind_t {
    match kind {
        HandleKind::RuntimeSession => VE_HANDLE_KIND_RUNTIME_SESSION,
        HandleKind::ProjectSession => VE_HANDLE_KIND_PROJECT_SESSION,
        HandleKind::Media => VE_HANDLE_KIND_MEDIA,
        HandleKind::Frame => VE_HANDLE_KIND_FRAME,
        HandleKind::Texture => VE_HANDLE_KIND_TEXTURE,
        HandleKind::Artifact => VE_HANDLE_KIND_ARTIFACT,
    }
}

fn handle_kind_name(kind: ve_handle_kind_t) -> &'static str {
    match kind {
        VE_HANDLE_KIND_RUNTIME_SESSION => "runtimeSession",
        VE_HANDLE_KIND_PROJECT_SESSION => "projectSession",
        VE_HANDLE_KIND_MEDIA => "media",
        VE_HANDLE_KIND_FRAME => "frame",
        VE_HANDLE_KIND_TEXTURE => "texture",
        VE_HANDLE_KIND_ARTIFACT => "artifact",
        VE_HANDLE_KIND_INVALID => "invalid",
        _ => "invalid",
    }
}

fn token_prefix(kind: HandleKind) -> &'static str {
    match kind {
        HandleKind::RuntimeSession => "runtimeHandle",
        HandleKind::ProjectSession => "projectSessionHandle",
        HandleKind::Media => "mediaHandle",
        HandleKind::Frame => "frameHandle",
        HandleKind::Texture => "textureHandle",
        HandleKind::Artifact => "artifactHandle",
    }
}

fn texture_backend(
    value: ve_texture_backend_t,
) -> Result<TextureBackend, (ve_status_t, Diagnostic)> {
    match value {
        VE_TEXTURE_BACKEND_D3D11_TEXTURE_2D => Ok(TextureBackend::D3d11Texture2D),
        VE_TEXTURE_BACKEND_D3D12_RESOURCE => Ok(TextureBackend::D3d12Resource),
        VE_TEXTURE_BACKEND_METAL_TEXTURE => Ok(TextureBackend::MetalTexture),
        VE_TEXTURE_BACKEND_CORE_VIDEO_PIXEL_BUFFER => Ok(TextureBackend::CoreVideoPixelBuffer),
        _ => Err(invalid_abi_value("texture backend")),
    }
}

fn pixel_format(value: ve_pixel_format_t) -> Result<VideoPixelFormat, (ve_status_t, Diagnostic)> {
    match value {
        VE_PIXEL_FORMAT_NV12 => Ok(VideoPixelFormat::Nv12),
        VE_PIXEL_FORMAT_BGRA8 => Ok(VideoPixelFormat::Bgra8),
        VE_PIXEL_FORMAT_RGBA8 => Ok(VideoPixelFormat::Rgba8),
        VE_PIXEL_FORMAT_P010 => Ok(VideoPixelFormat::P010),
        VE_PIXEL_FORMAT_YUV420P => Ok(VideoPixelFormat::Yuv420P),
        VE_PIXEL_FORMAT_UNKNOWN => Ok(VideoPixelFormat::Unknown),
        _ => Err(invalid_abi_value("pixel format")),
    }
}

fn color_primaries(
    value: ve_color_primaries_t,
) -> Result<ColorPrimaries, (ve_status_t, Diagnostic)> {
    match value {
        VE_COLOR_PRIMARIES_BT709 => Ok(ColorPrimaries::Bt709),
        VE_COLOR_PRIMARIES_BT2020 => Ok(ColorPrimaries::Bt2020),
        VE_COLOR_PRIMARIES_DISPLAY_P3 => Ok(ColorPrimaries::DisplayP3),
        VE_COLOR_PRIMARIES_UNKNOWN => Ok(ColorPrimaries::Unknown),
        _ => Err(invalid_abi_value("color primaries")),
    }
}

fn color_transfer(value: ve_color_transfer_t) -> Result<ColorTransfer, (ve_status_t, Diagnostic)> {
    match value {
        VE_COLOR_TRANSFER_BT709 => Ok(ColorTransfer::Bt709),
        VE_COLOR_TRANSFER_SRGB => Ok(ColorTransfer::Srgb),
        VE_COLOR_TRANSFER_PQ => Ok(ColorTransfer::Pq),
        VE_COLOR_TRANSFER_HLG => Ok(ColorTransfer::Hlg),
        VE_COLOR_TRANSFER_UNKNOWN => Ok(ColorTransfer::Unknown),
        _ => Err(invalid_abi_value("color transfer")),
    }
}

fn color_matrix(value: ve_color_matrix_t) -> Result<ColorMatrix, (ve_status_t, Diagnostic)> {
    match value {
        VE_COLOR_MATRIX_BT709 => Ok(ColorMatrix::Bt709),
        VE_COLOR_MATRIX_BT2020_NON_CONSTANT => Ok(ColorMatrix::Bt2020NonConstant),
        VE_COLOR_MATRIX_IDENTITY => Ok(ColorMatrix::Identity),
        VE_COLOR_MATRIX_UNKNOWN => Ok(ColorMatrix::Unknown),
        _ => Err(invalid_abi_value("color matrix")),
    }
}

fn color_range(value: ve_color_range_t) -> Result<ColorRange, (ve_status_t, Diagnostic)> {
    match value {
        VE_COLOR_RANGE_LIMITED => Ok(ColorRange::Limited),
        VE_COLOR_RANGE_FULL => Ok(ColorRange::Full),
        VE_COLOR_RANGE_UNKNOWN => Ok(ColorRange::Unknown),
        _ => Err(invalid_abi_value("color range")),
    }
}

fn invalid_abi_value(name: &'static str) -> (ve_status_t, Diagnostic) {
    (
        ve_status_t::VE_STATUS_INVALID_ARGUMENT,
        Diagnostic::error(
            ve_status_t::VE_STATUS_INVALID_ARGUMENT,
            format!("{name} is invalid"),
        ),
    )
}

fn status_from_runtime_error(error: &RuntimeError) -> ve_status_t {
    match error.kind {
        RuntimeErrorKind::InvalidRequest => ve_status_t::VE_STATUS_INVALID_REQUEST,
        RuntimeErrorKind::UnknownRuntimeSession => ve_status_t::VE_STATUS_UNKNOWN_RUNTIME_SESSION,
        RuntimeErrorKind::UnknownProjectSession => ve_status_t::VE_STATUS_UNKNOWN_PROJECT_SESSION,
        RuntimeErrorKind::UnknownHandle => ve_status_t::VE_STATUS_UNKNOWN_HANDLE,
        RuntimeErrorKind::WrongKind => ve_status_t::VE_STATUS_WRONG_KIND,
        RuntimeErrorKind::WrongOwner => ve_status_t::VE_STATUS_WRONG_OWNER,
        RuntimeErrorKind::WrongDevice => ve_status_t::VE_STATUS_WRONG_DEVICE,
        RuntimeErrorKind::TextureMetadataMismatch => {
            ve_status_t::VE_STATUS_TEXTURE_METADATA_MISMATCH
        }
        RuntimeErrorKind::StaleGeneration => ve_status_t::VE_STATUS_STALE_GENERATION,
        RuntimeErrorKind::LeaseExpired => ve_status_t::VE_STATUS_LEASE_EXPIRED,
        RuntimeErrorKind::DoubleRelease => ve_status_t::VE_STATUS_DOUBLE_RELEASE,
        RuntimeErrorKind::ProjectStore => ve_status_t::VE_STATUS_PROJECT_STORE,
        RuntimeErrorKind::Scheduler => ve_status_t::VE_STATUS_SCHEDULER,
    }
}

fn status_code(status: ve_status_t) -> &'static str {
    match status {
        ve_status_t::VE_STATUS_OK => "ok",
        ve_status_t::VE_STATUS_INVALID_ARGUMENT => "invalidArgument",
        ve_status_t::VE_STATUS_BUFFER_TOO_SMALL => "bufferTooSmall",
        ve_status_t::VE_STATUS_INVALID_UTF8 => "invalidUtf8",
        ve_status_t::VE_STATUS_INVALID_REQUEST => "invalidRequest",
        ve_status_t::VE_STATUS_UNKNOWN_RUNTIME_SESSION => "unknownRuntimeSession",
        ve_status_t::VE_STATUS_UNKNOWN_PROJECT_SESSION => "unknownProjectSession",
        ve_status_t::VE_STATUS_UNKNOWN_HANDLE => "unknownHandle",
        ve_status_t::VE_STATUS_WRONG_KIND => "wrongKind",
        ve_status_t::VE_STATUS_WRONG_OWNER => "wrongOwner",
        ve_status_t::VE_STATUS_WRONG_DEVICE => "wrongDevice",
        ve_status_t::VE_STATUS_TEXTURE_METADATA_MISMATCH => "textureMetadataMismatch",
        ve_status_t::VE_STATUS_STALE_GENERATION => "staleGeneration",
        ve_status_t::VE_STATUS_LEASE_EXPIRED => "leaseExpired",
        ve_status_t::VE_STATUS_DOUBLE_RELEASE => "doubleRelease",
        ve_status_t::VE_STATUS_PROJECT_STORE => "projectStore",
        ve_status_t::VE_STATUS_SCHEDULER => "scheduler",
        ve_status_t::VE_STATUS_PANIC => "panic",
    }
}
