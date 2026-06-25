use std::ffi::CString;
use std::ptr;
use std::thread;
use std::time::Duration;

use bindings_c::{
    VE_COLOR_MATRIX_IDENTITY, VE_COLOR_PRIMARIES_BT709, VE_COLOR_RANGE_FULL,
    VE_COLOR_TRANSFER_SRGB, VE_HANDLE_KIND_MEDIA, VE_HANDLE_KIND_PROJECT_SESSION,
    VE_HANDLE_KIND_RUNTIME_SESSION, VE_HANDLE_KIND_TEXTURE, VE_PIXEL_FORMAT_BGRA8,
    VE_TEXTURE_BACKEND_METAL_TEXTURE, ve_buffer_t, ve_handle_t, ve_runtime_config_t, ve_runtime_t,
    ve_status_t, ve_texture_descriptor_t,
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
    std::fs::create_dir_all(&bundle_path).expect("bundle directory should be created");
    let project_json = serde_json::to_string_pretty(&draft).expect("draft should serialize");
    std::fs::write(
        bundle_path.join("project.json"),
        format!("{project_json}\n"),
    )
    .expect("project json should be written");

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
    assert_eq!(project.kind, VE_HANDLE_KIND_PROJECT_SESSION);
    assert_eq!(project.owner_id, runtime.id);
    assert!(diagnostics.as_str().contains("\"projectSession\""));

    diagnostics.clear();
    let status =
        unsafe { bindings_c::ve_handle_retain(runtime, project, diagnostics.as_mut_buffer()) };
    assert_eq!(status, ve_status_t::VE_STATUS_OK);

    diagnostics.clear();
    let status =
        unsafe { bindings_c::ve_handle_release(runtime, project, diagnostics.as_mut_buffer()) };
    assert_eq!(status, ve_status_t::VE_STATUS_OK);
    assert!(diagnostics.as_str().contains("\"explicit\""));

    diagnostics.clear();
    let status =
        unsafe { bindings_c::ve_handle_release(runtime, project, diagnostics.as_mut_buffer()) };
    assert_eq!(status, ve_status_t::VE_STATUS_OK);

    diagnostics.clear();
    let status =
        unsafe { bindings_c::ve_handle_release(runtime, project, diagnostics.as_mut_buffer()) };
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

    let status = unsafe {
        bindings_c::ve_runtime_create(ptr::null(), ptr::null_mut(), diagnostics.as_mut_buffer())
    };
    assert_eq!(status, ve_status_t::VE_STATUS_INVALID_ARGUMENT);
    assert!(diagnostics.as_str().contains("\"invalidArgument\""));

    diagnostics.clear();
    let status = unsafe {
        bindings_c::ve_runtime_create(ptr::null(), &mut runtime, diagnostics.as_mut_buffer())
    };
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
        kind: VE_HANDLE_KIND_MEDIA,
        id: 999_999,
        owner_id: runtime.id,
        owner_generation: runtime.generation,
        generation: 1,
    };
    diagnostics.clear();
    let status =
        unsafe { bindings_c::ve_handle_release(runtime, fabricated, diagnostics.as_mut_buffer()) };
    assert_eq!(status, ve_status_t::VE_STATUS_UNKNOWN_HANDLE);

    for forbidden_kind in [
        VE_HANDLE_KIND_RUNTIME_SESSION,
        VE_HANDLE_KIND_PROJECT_SESSION,
    ] {
        let mut fabricated_lifecycle_handle = ve_handle_t::default();
        diagnostics.clear();
        let status = unsafe {
            bindings_c::ve_handle_acquire(
                runtime,
                forbidden_kind,
                ptr::null(),
                0,
                &mut fabricated_lifecycle_handle,
                diagnostics.as_mut_buffer(),
            )
        };
        assert_eq!(status, ve_status_t::VE_STATUS_INVALID_ARGUMENT);
        assert_eq!(
            fabricated_lifecycle_handle,
            ve_handle_t::default(),
            "forbidden lifecycle acquire must not fabricate a handle"
        );
        assert!(diagnostics.as_str().contains("lifecycle APIs"));
    }

    let mut media = ve_handle_t::default();
    diagnostics.clear();
    let status = unsafe {
        bindings_c::ve_handle_acquire(
            runtime,
            VE_HANDLE_KIND_MEDIA,
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
    let status =
        unsafe { bindings_c::ve_handle_release(runtime, stale, diagnostics.as_mut_buffer()) };
    assert_eq!(status, ve_status_t::VE_STATUS_STALE_GENERATION);

    diagnostics.clear();
    let status =
        unsafe { bindings_c::ve_handle_release(runtime, media, diagnostics.as_mut_buffer()) };
    assert_eq!(status, ve_status_t::VE_STATUS_OK);

    diagnostics.clear();
    let status =
        unsafe { bindings_c::ve_handle_release(runtime, media, diagnostics.as_mut_buffer()) };
    assert_eq!(status, ve_status_t::VE_STATUS_DOUBLE_RELEASE);

    diagnostics.clear();
    let status = unsafe { bindings_c::ve_runtime_close(runtime, diagnostics.as_mut_buffer()) };
    assert_eq!(status, ve_status_t::VE_STATUS_OK);
}

#[test]
fn abi_smoke_enforces_lease_expiry_and_validates_integer_discriminants() {
    let (runtime, mut diagnostics) = runtime("abi-lease-and-discriminants");
    let descriptor = texture_descriptor("adapter-a", "device-a");

    let mut short_texture = ve_handle_t::default();
    assert_eq!(
        unsafe {
            bindings_c::ve_handle_acquire(
                runtime,
                VE_HANDLE_KIND_TEXTURE,
                &descriptor,
                100_000,
                &mut short_texture,
                diagnostics.as_mut_buffer(),
            )
        },
        ve_status_t::VE_STATUS_OK
    );
    diagnostics.clear();
    assert_eq!(
        unsafe {
            bindings_c::ve_texture_handle_resolve(
                runtime,
                short_texture,
                &descriptor,
                diagnostics.as_mut_buffer(),
            )
        },
        ve_status_t::VE_STATUS_OK
    );
    thread::sleep(Duration::from_millis(120));
    diagnostics.clear();
    assert_eq!(
        unsafe {
            bindings_c::ve_texture_handle_resolve(
                runtime,
                short_texture,
                &descriptor,
                diagnostics.as_mut_buffer(),
            )
        },
        ve_status_t::VE_STATUS_LEASE_EXPIRED
    );

    let mut expired_texture = ve_handle_t::default();
    assert_eq!(
        unsafe {
            bindings_c::ve_handle_acquire(
                runtime,
                VE_HANDLE_KIND_TEXTURE,
                &descriptor,
                1,
                &mut expired_texture,
                diagnostics.as_mut_buffer(),
            )
        },
        ve_status_t::VE_STATUS_OK
    );
    thread::sleep(Duration::from_millis(2));

    diagnostics.clear();
    assert_eq!(
        unsafe {
            bindings_c::ve_texture_handle_resolve(
                runtime,
                expired_texture,
                &descriptor,
                diagnostics.as_mut_buffer(),
            )
        },
        ve_status_t::VE_STATUS_LEASE_EXPIRED
    );

    let mut invalid_texture = descriptor;
    invalid_texture.backend = 999_999;
    let mut handle = ve_handle_t::default();
    diagnostics.clear();
    assert_eq!(
        unsafe {
            bindings_c::ve_handle_acquire(
                runtime,
                VE_HANDLE_KIND_TEXTURE,
                &invalid_texture,
                0,
                &mut handle,
                diagnostics.as_mut_buffer(),
            )
        },
        ve_status_t::VE_STATUS_INVALID_ARGUMENT
    );

    diagnostics.clear();
    assert_eq!(
        unsafe {
            bindings_c::ve_handle_acquire(
                runtime,
                999_999,
                ptr::null(),
                0,
                &mut handle,
                diagnostics.as_mut_buffer(),
            )
        },
        ve_status_t::VE_STATUS_INVALID_ARGUMENT
    );

    diagnostics.clear();
    assert_eq!(
        unsafe { bindings_c::ve_runtime_close(runtime, diagnostics.as_mut_buffer()) },
        ve_status_t::VE_STATUS_OK
    );
}

#[test]
fn abi_smoke_diagnostic_buffer_errors_do_not_mask_successful_side_effects() {
    let (runtime, mut diagnostics) = runtime("abi-diagnostic-side-effects");
    let mut media = ve_handle_t::default();
    assert_eq!(
        unsafe {
            bindings_c::ve_handle_acquire(
                runtime,
                VE_HANDLE_KIND_MEDIA,
                ptr::null(),
                0,
                &mut media,
                diagnostics.as_mut_buffer(),
            )
        },
        ve_status_t::VE_STATUS_OK
    );

    let mut tiny = JsonBuffer::new(4);
    assert_eq!(
        unsafe { bindings_c::ve_handle_release(runtime, media, tiny.as_mut_buffer()) },
        ve_status_t::VE_STATUS_OK
    );

    diagnostics.clear();
    assert_eq!(
        unsafe { bindings_c::ve_handle_release(runtime, media, diagnostics.as_mut_buffer()) },
        ve_status_t::VE_STATUS_DOUBLE_RELEASE
    );

    diagnostics.clear();
    assert_eq!(
        unsafe { bindings_c::ve_runtime_close(runtime, diagnostics.as_mut_buffer()) },
        ve_status_t::VE_STATUS_OK
    );
}

#[test]
fn abi_smoke_depends_on_editor_runtime_and_not_desktop_adapter() {
    let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let manifest =
        std::fs::read_to_string(manifest_path).expect("bindings_c manifest should be readable");
    assert!(manifest.contains("editor_runtime"));
    let forbidden = ["bindings", "node"].join("_");
    assert!(!manifest.contains(&forbidden));
}

#[test]
fn abi_smoke_generated_header_declares_smoke_surface() {
    let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let config = std::fs::read_to_string(manifest_dir.join("cbindgen.toml"))
        .expect("cbindgen config exists");
    assert!(config.contains("video_editor_runtime.h"));

    let header = std::fs::read_to_string(manifest_dir.join("include/video_editor_runtime.h"))
        .expect("generated C header exists");
    for symbol in [
        "ve_status_t",
        "ve_error_code_t",
        "ve_handle_t",
        "ve_buffer_t",
        "ve_runtime_config_t",
        "ve_runtime_create",
        "ve_runtime_close",
        "ve_project_open",
        "ve_handle_acquire",
        "ve_handle_retain",
        "ve_handle_release",
        "ve_texture_handle_resolve",
        "ve_last_error_json",
        "VE_HANDLE_KIND_TEXTURE",
        "VE_TEXTURE_BACKEND_METAL_TEXTURE",
    ] {
        assert!(header.contains(symbol), "header missing {symbol}");
    }
    assert!(
        !header.contains("ve_buffer_free"),
        "caller-owned diagnostic buffers must not expose a Rust free function"
    );
}

fn runtime(label: &str) -> (ve_runtime_t, JsonBuffer) {
    let mut runtime = ve_runtime_t::default();
    let mut diagnostics = JsonBuffer::new(4096);
    let label = CString::new(label).expect("label should not contain nul");
    let config = ve_runtime_config_t {
        diagnostic_label: label.as_ptr(),
    };
    assert_eq!(
        unsafe {
            bindings_c::ve_runtime_create(&config, &mut runtime, diagnostics.as_mut_buffer())
        },
        ve_status_t::VE_STATUS_OK
    );
    (runtime, diagnostics)
}

fn texture_descriptor(adapter_id: &str, device_id: &str) -> ve_texture_descriptor_t {
    let adapter_id = CString::new(adapter_id).expect("adapter id should not contain nul");
    let device_id = CString::new(device_id).expect("device id should not contain nul");
    ve_texture_descriptor_t {
        backend: VE_TEXTURE_BACKEND_METAL_TEXTURE,
        adapter_id: adapter_id.into_raw(),
        device_id: device_id.into_raw(),
        width: 1920,
        height: 1080,
        pixel_format: VE_PIXEL_FORMAT_BGRA8,
        color_primaries: VE_COLOR_PRIMARIES_BT709,
        color_transfer: VE_COLOR_TRANSFER_SRGB,
        color_matrix: VE_COLOR_MATRIX_IDENTITY,
        color_range: VE_COLOR_RANGE_FULL,
    }
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
        std::str::from_utf8(&self.bytes[..self.buffer.len]).expect("diagnostic JSON should be utf8")
    }
}
