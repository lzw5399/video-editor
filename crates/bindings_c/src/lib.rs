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
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ve_handle_kind_t {
    VE_HANDLE_KIND_INVALID = 0,
    VE_HANDLE_KIND_RUNTIME_SESSION = 1,
    VE_HANDLE_KIND_PROJECT_SESSION = 2,
    VE_HANDLE_KIND_MEDIA = 3,
    VE_HANDLE_KIND_FRAME = 4,
    VE_HANDLE_KIND_TEXTURE = 5,
    VE_HANDLE_KIND_ARTIFACT = 6,
}

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ve_texture_backend_t {
    VE_TEXTURE_BACKEND_D3D11_TEXTURE_2D = 1,
    VE_TEXTURE_BACKEND_D3D12_RESOURCE = 2,
    VE_TEXTURE_BACKEND_METAL_TEXTURE = 3,
    VE_TEXTURE_BACKEND_CORE_VIDEO_PIXEL_BUFFER = 4,
}

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ve_pixel_format_t {
    VE_PIXEL_FORMAT_NV12 = 1,
    VE_PIXEL_FORMAT_BGRA8 = 2,
    VE_PIXEL_FORMAT_RGBA8 = 3,
    VE_PIXEL_FORMAT_P010 = 4,
    VE_PIXEL_FORMAT_YUV420P = 5,
    VE_PIXEL_FORMAT_UNKNOWN = 255,
}

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ve_color_primaries_t {
    VE_COLOR_PRIMARIES_BT709 = 1,
    VE_COLOR_PRIMARIES_BT2020 = 2,
    VE_COLOR_PRIMARIES_DISPLAY_P3 = 3,
    VE_COLOR_PRIMARIES_UNKNOWN = 255,
}

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ve_color_transfer_t {
    VE_COLOR_TRANSFER_BT709 = 1,
    VE_COLOR_TRANSFER_SRGB = 2,
    VE_COLOR_TRANSFER_PQ = 3,
    VE_COLOR_TRANSFER_HLG = 4,
    VE_COLOR_TRANSFER_UNKNOWN = 255,
}

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ve_color_matrix_t {
    VE_COLOR_MATRIX_BT709 = 1,
    VE_COLOR_MATRIX_BT2020_NON_CONSTANT = 2,
    VE_COLOR_MATRIX_IDENTITY = 3,
    VE_COLOR_MATRIX_UNKNOWN = 255,
}

#[allow(non_camel_case_types)]
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ve_color_range_t {
    VE_COLOR_RANGE_LIMITED = 1,
    VE_COLOR_RANGE_FULL = 2,
    VE_COLOR_RANGE_UNKNOWN = 255,
}

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
            kind: ve_handle_kind_t::VE_HANDLE_KIND_INVALID,
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
                let status = status_from_runtime_error(&error);
                return (
                    Some(runtime),
                    status,
                    Diagnostic::from_runtime_error(status, &error),
                );
            }
        };
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
    lease_expires_at_us: u64,
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
        let request = match acquire_request(kind, texture, &entry.session.id, lease_expires_at_us) {
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
        match entry.handles.retain(&entry.session.id, &token, 0) {
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
            Ok(report) => (
                Some(runtime),
                ve_status_t::VE_STATUS_OK,
                Diagnostic::handle_released(runtime, handle, report.release_state),
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
        match entry.handles.resolve_texture(
            &entry.session.id,
            &token,
            &TextureResolveExpectation { descriptor },
            0,
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn ve_buffer_free(buffer: *mut ve_buffer_t) {
    if buffer.is_null() {
        return;
    }
    let buffer = unsafe { &mut *buffer };
    if buffer.data.is_null() || buffer.capacity == 0 {
        buffer.len = 0;
        buffer.capacity = 0;
        return;
    }
    unsafe {
        drop(Vec::from_raw_parts(
            buffer.data,
            buffer.len.min(buffer.capacity),
            buffer.capacity,
        ));
    }
    buffer.data = ptr::null_mut();
    buffer.len = 0;
    buffer.capacity = 0;
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
    projects: ProjectSessionService,
    handles: HandleRegistry,
    last_error: Option<Diagnostic>,
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
            write_status
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
    lease_expires_at_us: u64,
) -> Result<HandleAcquireRequest, (ve_status_t, Diagnostic)> {
    let request = match kind {
        ve_handle_kind_t::VE_HANDLE_KIND_RUNTIME_SESSION => {
            HandleAcquireRequest::runtime_session(owner.clone())
        }
        ve_handle_kind_t::VE_HANDLE_KIND_PROJECT_SESSION => {
            HandleAcquireRequest::project_session(owner.clone())
        }
        ve_handle_kind_t::VE_HANDLE_KIND_MEDIA => HandleAcquireRequest::media(owner.clone()),
        ve_handle_kind_t::VE_HANDLE_KIND_FRAME => HandleAcquireRequest::frame(owner.clone()),
        ve_handle_kind_t::VE_HANDLE_KIND_TEXTURE => {
            HandleAcquireRequest::texture(owner.clone(), texture_descriptor(texture)?)
        }
        ve_handle_kind_t::VE_HANDLE_KIND_ARTIFACT => HandleAcquireRequest::artifact(owner.clone()),
        ve_handle_kind_t::VE_HANDLE_KIND_INVALID => {
            return Err((
                ve_status_t::VE_STATUS_INVALID_ARGUMENT,
                Diagnostic::error(
                    ve_status_t::VE_STATUS_INVALID_ARGUMENT,
                    "handle kind is invalid",
                ),
            ));
        }
    };
    Ok(if lease_expires_at_us > 0 {
        request.with_lease_expires_at_us(lease_expires_at_us)
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
    let backend = texture_backend(texture.backend);
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
        pixel_format: pixel_format(texture.pixel_format),
        color: VideoColorMetadata {
            primaries: color_primaries(texture.color_primaries),
            transfer: color_transfer(texture.color_transfer),
            matrix: color_matrix(texture.color_matrix),
            range: color_range(texture.color_range),
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
    let kind = match handle_kind(handle.kind) {
        Some(kind) => kind,
        None => {
            return Err((
                ve_status_t::VE_STATUS_INVALID_ARGUMENT,
                Diagnostic::error(
                    ve_status_t::VE_STATUS_INVALID_ARGUMENT,
                    "handle kind is invalid",
                ),
            ));
        }
    };
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

fn handle_kind(kind: ve_handle_kind_t) -> Option<HandleKind> {
    match kind {
        ve_handle_kind_t::VE_HANDLE_KIND_RUNTIME_SESSION => Some(HandleKind::RuntimeSession),
        ve_handle_kind_t::VE_HANDLE_KIND_PROJECT_SESSION => Some(HandleKind::ProjectSession),
        ve_handle_kind_t::VE_HANDLE_KIND_MEDIA => Some(HandleKind::Media),
        ve_handle_kind_t::VE_HANDLE_KIND_FRAME => Some(HandleKind::Frame),
        ve_handle_kind_t::VE_HANDLE_KIND_TEXTURE => Some(HandleKind::Texture),
        ve_handle_kind_t::VE_HANDLE_KIND_ARTIFACT => Some(HandleKind::Artifact),
        ve_handle_kind_t::VE_HANDLE_KIND_INVALID => None,
    }
}

fn handle_kind_c(kind: HandleKind) -> ve_handle_kind_t {
    match kind {
        HandleKind::RuntimeSession => ve_handle_kind_t::VE_HANDLE_KIND_RUNTIME_SESSION,
        HandleKind::ProjectSession => ve_handle_kind_t::VE_HANDLE_KIND_PROJECT_SESSION,
        HandleKind::Media => ve_handle_kind_t::VE_HANDLE_KIND_MEDIA,
        HandleKind::Frame => ve_handle_kind_t::VE_HANDLE_KIND_FRAME,
        HandleKind::Texture => ve_handle_kind_t::VE_HANDLE_KIND_TEXTURE,
        HandleKind::Artifact => ve_handle_kind_t::VE_HANDLE_KIND_ARTIFACT,
    }
}

fn handle_kind_name(kind: ve_handle_kind_t) -> &'static str {
    match kind {
        ve_handle_kind_t::VE_HANDLE_KIND_RUNTIME_SESSION => "runtimeSession",
        ve_handle_kind_t::VE_HANDLE_KIND_PROJECT_SESSION => "projectSession",
        ve_handle_kind_t::VE_HANDLE_KIND_MEDIA => "media",
        ve_handle_kind_t::VE_HANDLE_KIND_FRAME => "frame",
        ve_handle_kind_t::VE_HANDLE_KIND_TEXTURE => "texture",
        ve_handle_kind_t::VE_HANDLE_KIND_ARTIFACT => "artifact",
        ve_handle_kind_t::VE_HANDLE_KIND_INVALID => "invalid",
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

fn texture_backend(value: ve_texture_backend_t) -> TextureBackend {
    match value {
        ve_texture_backend_t::VE_TEXTURE_BACKEND_D3D11_TEXTURE_2D => TextureBackend::D3d11Texture2D,
        ve_texture_backend_t::VE_TEXTURE_BACKEND_D3D12_RESOURCE => TextureBackend::D3d12Resource,
        ve_texture_backend_t::VE_TEXTURE_BACKEND_METAL_TEXTURE => TextureBackend::MetalTexture,
        ve_texture_backend_t::VE_TEXTURE_BACKEND_CORE_VIDEO_PIXEL_BUFFER => {
            TextureBackend::CoreVideoPixelBuffer
        }
    }
}

fn pixel_format(value: ve_pixel_format_t) -> VideoPixelFormat {
    match value {
        ve_pixel_format_t::VE_PIXEL_FORMAT_NV12 => VideoPixelFormat::Nv12,
        ve_pixel_format_t::VE_PIXEL_FORMAT_BGRA8 => VideoPixelFormat::Bgra8,
        ve_pixel_format_t::VE_PIXEL_FORMAT_RGBA8 => VideoPixelFormat::Rgba8,
        ve_pixel_format_t::VE_PIXEL_FORMAT_P010 => VideoPixelFormat::P010,
        ve_pixel_format_t::VE_PIXEL_FORMAT_YUV420P => VideoPixelFormat::Yuv420P,
        ve_pixel_format_t::VE_PIXEL_FORMAT_UNKNOWN => VideoPixelFormat::Unknown,
    }
}

fn color_primaries(value: ve_color_primaries_t) -> ColorPrimaries {
    match value {
        ve_color_primaries_t::VE_COLOR_PRIMARIES_BT709 => ColorPrimaries::Bt709,
        ve_color_primaries_t::VE_COLOR_PRIMARIES_BT2020 => ColorPrimaries::Bt2020,
        ve_color_primaries_t::VE_COLOR_PRIMARIES_DISPLAY_P3 => ColorPrimaries::DisplayP3,
        ve_color_primaries_t::VE_COLOR_PRIMARIES_UNKNOWN => ColorPrimaries::Unknown,
    }
}

fn color_transfer(value: ve_color_transfer_t) -> ColorTransfer {
    match value {
        ve_color_transfer_t::VE_COLOR_TRANSFER_BT709 => ColorTransfer::Bt709,
        ve_color_transfer_t::VE_COLOR_TRANSFER_SRGB => ColorTransfer::Srgb,
        ve_color_transfer_t::VE_COLOR_TRANSFER_PQ => ColorTransfer::Pq,
        ve_color_transfer_t::VE_COLOR_TRANSFER_HLG => ColorTransfer::Hlg,
        ve_color_transfer_t::VE_COLOR_TRANSFER_UNKNOWN => ColorTransfer::Unknown,
    }
}

fn color_matrix(value: ve_color_matrix_t) -> ColorMatrix {
    match value {
        ve_color_matrix_t::VE_COLOR_MATRIX_BT709 => ColorMatrix::Bt709,
        ve_color_matrix_t::VE_COLOR_MATRIX_BT2020_NON_CONSTANT => ColorMatrix::Bt2020NonConstant,
        ve_color_matrix_t::VE_COLOR_MATRIX_IDENTITY => ColorMatrix::Identity,
        ve_color_matrix_t::VE_COLOR_MATRIX_UNKNOWN => ColorMatrix::Unknown,
    }
}

fn color_range(value: ve_color_range_t) -> ColorRange {
    match value {
        ve_color_range_t::VE_COLOR_RANGE_LIMITED => ColorRange::Limited,
        ve_color_range_t::VE_COLOR_RANGE_FULL => ColorRange::Full,
        ve_color_range_t::VE_COLOR_RANGE_UNKNOWN => ColorRange::Unknown,
    }
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
