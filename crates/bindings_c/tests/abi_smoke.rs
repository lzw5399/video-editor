use std::ffi::CString;
use std::ptr;

use bindings_c::{
    ve_buffer_t, ve_error_code_t, ve_handle_kind_t, ve_handle_t, ve_runtime_config_t,
    ve_runtime_t, ve_status_t,
};

#[test]
fn abi_smoke_creates_opens_and_releases_runtime_handles_with_json_diagnostics() {
    let mut runtime = ve_runtime_t::default();
    let mut diagnostics = JsonBuffer::new(4096);
    let label = CString::new("abi-smoke").expect("label should not contain nul");
    let config = ve_runtime_config_t {
        diagnostic_label: label.as_ptr(),
    };

    let status = unsafe {
        bindings_c::ve_runtime_create(&config, &mut runtime, diagnostics.as_mut_buffer())
    };
    assert_eq!(status, ve_status_t::VE_STATUS_OK);
    assert!(runtime.id > 0);
    assert_eq!(runtime.generation, 1);
    assert!(diagnostics.as_str().contains("\"ok\":true"));

    let temp_dir = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = temp_dir.path().join("abi-smoke.veproj");
    let draft = draft_model::Draft::new("draft-abi-smoke", "ABI smoke");
    project_store::create_project_bundle(
        &project_store::StdPlatformFileSystem,
        &bundle_path,
        &draft,
    )
    .expect("project bundle should be created");

    let path = CString::new(bundle_path.to_string_lossy().as_bytes())
        .expect("bundle path should not contain nul");
    let mut project = ve_handle_t::default();
    diagnostics.clear();
    let status = unsafe {
        bindings_c::ve_project_open(
            runtime,
            path.as_ptr(),
            &mut project,
            diagnostics.as_mut_buffer(),
        )
    };
    assert_eq!(status, ve_status_t::VE_STATUS_OK);
    assert_eq!(project.kind, ve_handle_kind_t::VE_HANDLE_KIND_PROJECT_SESSION);
    assert_eq!(project.owner_id, runtime.id);
    assert!(diagnostics.as_str().contains("\"projectSession\""));

    diagnostics.clear();
    let status = unsafe { bindings_c::ve_handle_retain(runtime, project, diagnostics.as_mut_buffer()) };
    assert_eq!(status, ve_status_t::VE_STATUS_OK);

    diagnostics.clear();
    let status = unsafe { bindings_c::ve_handle_release(runtime, project, diagnostics.as_mut_buffer()) };
    assert_eq!(status, ve_status_t::VE_STATUS_OK);
    assert!(diagnostics.as_str().contains("\"explicit\""));

    diagnostics.clear();
    let status = unsafe { bindings_c::ve_handle_release(runtime, project, diagnostics.as_mut_buffer()) };
    assert_eq!(status, ve_status_t::VE_STATUS_OK);

    diagnostics.clear();
    let status = unsafe { bindings_c::ve_handle_release(runtime, project, diagnostics.as_mut_buffer()) };
    assert_eq!(status, ve_status_t::VE_STATUS_DOUBLE_RELEASE);
    assert!(diagnostics.as_str().contains("\"doubleRelease\""));

    diagnostics.clear();
    let status = unsafe { bindings_c::ve_runtime_close(runtime, diagnostics.as_mut_buffer()) };
    assert_eq!(status, ve_status_t::VE_STATUS_OK);
}

#[test]
fn abi_smoke_rejects_invalid_inputs_without_panicking() {
    let mut runtime = ve_runtime_t::default();
    let mut diagnostics = JsonBuffer::new(4096);

    let status =
        unsafe { bindings_c::ve_runtime_create(ptr::null(), ptr::null_mut(), diagnostics.as_mut_buffer()) };
    assert_eq!(status, ve_status_t::VE_STATUS_INVALID_ARGUMENT);
    assert!(diagnostics.as_str().contains("\"invalidArgument\""));

    diagnostics.clear();
    let status =
        unsafe { bindings_c::ve_runtime_create(ptr::null(), &mut runtime, diagnostics.as_mut_buffer()) };
    assert_eq!(status, ve_status_t::VE_STATUS_OK);

    let bad_utf8 = [0xff_u8, 0x00];
    let mut project = ve_handle_t::default();
    diagnostics.clear();
    let status = unsafe {
        bindings_c::ve_project_open(
            runtime,
            bad_utf8.as_ptr().cast(),
            &mut project,
            diagnostics.as_mut_buffer(),
        )
    };
    assert_eq!(status, ve_status_t::VE_STATUS_INVALID_UTF8);

    let mut tiny = JsonBuffer::new(8);
    let status = unsafe { bindings_c::ve_last_error_json(runtime, tiny.as_mut_buffer()) };
    assert_eq!(status, ve_status_t::VE_STATUS_BUFFER_TOO_SMALL);

    let fabricated = ve_handle_t {
        kind: ve_handle_kind_t::VE_HANDLE_KIND_MEDIA,
        id: 999_999,
        owner_id: runtime.id,
        owner_generation: runtime.generation,
        generation: 1,
    };
    diagnostics.clear();
    let status =
        unsafe { bindings_c::ve_handle_release(runtime, fabricated, diagnostics.as_mut_buffer()) };
    assert_eq!(status, ve_status_t::VE_STATUS_UNKNOWN_HANDLE);

    let mut media = ve_handle_t::default();
    diagnostics.clear();
    let status = unsafe {
        bindings_c::ve_handle_acquire(
            runtime,
            ve_handle_kind_t::VE_HANDLE_KIND_MEDIA,
            ptr::null(),
            0,
            &mut media,
            diagnostics.as_mut_buffer(),
        )
    };
    assert_eq!(status, ve_status_t::VE_STATUS_OK);

    let wrong_owner = ve_runtime_t {
        id: runtime.id + 10,
        generation: runtime.generation,
    };
    diagnostics.clear();
    let status =
        unsafe { bindings_c::ve_handle_release(wrong_owner, media, diagnostics.as_mut_buffer()) };
    assert_eq!(status, ve_status_t::VE_STATUS_UNKNOWN_RUNTIME_SESSION);

    let stale = ve_handle_t {
        generation: media.generation + 1,
        ..media
    };
    diagnostics.clear();
    let status = unsafe { bindings_c::ve_handle_release(runtime, stale, diagnostics.as_mut_buffer()) };
    assert_eq!(status, ve_status_t::VE_STATUS_STALE_GENERATION);

    diagnostics.clear();
    let status = unsafe { bindings_c::ve_handle_release(runtime, media, diagnostics.as_mut_buffer()) };
    assert_eq!(status, ve_status_t::VE_STATUS_OK);

    diagnostics.clear();
    let status = unsafe { bindings_c::ve_handle_release(runtime, media, diagnostics.as_mut_buffer()) };
    assert_eq!(status, ve_status_t::VE_STATUS_DOUBLE_RELEASE);

    diagnostics.clear();
    let status = unsafe { bindings_c::ve_runtime_close(runtime, diagnostics.as_mut_buffer()) };
    assert_eq!(status, ve_status_t::VE_STATUS_OK);
}

#[test]
fn abi_smoke_depends_on_editor_runtime_and_not_bindings_node() {
    let manifest = std::fs::read_to_string("crates/bindings_c/Cargo.toml")
        .expect("bindings_c manifest should be readable");
    assert!(manifest.contains("editor_runtime"));
    assert!(!manifest.contains("bindings_node"));
}

struct JsonBuffer {
    bytes: Vec<u8>,
    buffer: ve_buffer_t,
}

impl JsonBuffer {
    fn new(capacity: usize) -> Self {
        let mut bytes = vec![0_u8; capacity];
        let buffer = ve_buffer_t {
            data: bytes.as_mut_ptr(),
            len: 0,
            capacity,
        };
        Self { bytes, buffer }
    }

    fn as_mut_buffer(&mut self) -> *mut ve_buffer_t {
        self.buffer.data = self.bytes.as_mut_ptr();
        &mut self.buffer
    }

    fn clear(&mut self) {
        self.bytes.fill(0);
        self.buffer.len = 0;
    }

    fn as_str(&self) -> &str {
        std::str::from_utf8(&self.bytes[..self.buffer.len])
            .expect("diagnostic JSON should be utf8")
    }
}
