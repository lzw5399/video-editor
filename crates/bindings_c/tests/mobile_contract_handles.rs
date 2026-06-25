use std::ffi::CString;
use std::ptr;

use bindings_c::{
    ve_buffer_t, ve_color_matrix_t, ve_color_primaries_t, ve_color_range_t, ve_color_transfer_t,
    ve_handle_kind_t, ve_handle_t, ve_pixel_format_t, ve_runtime_t, ve_status_t,
    ve_texture_backend_t, ve_texture_descriptor_t,
};

#[test]
fn mobile_contract_handles_validate_owner_generation_and_release() {
    let (runtime, mut diagnostics) = runtime("mobile-contract-handles");

    let mut media = ve_handle_t::default();
    assert_eq!(
        unsafe {
            bindings_c::ve_handle_acquire(
                runtime,
                ve_handle_kind_t::VE_HANDLE_KIND_MEDIA,
                ptr::null(),
                0,
                &mut media,
                diagnostics.as_mut_buffer(),
            )
        },
        ve_status_t::VE_STATUS_OK
    );

    diagnostics.clear();
    assert_eq!(
        unsafe { bindings_c::ve_handle_retain(runtime, media, diagnostics.as_mut_buffer()) },
        ve_status_t::VE_STATUS_OK
    );

    let wrong_owner = ve_handle_t {
        owner_id: runtime.id + 1,
        ..media
    };
    diagnostics.clear();
    assert_eq!(
        unsafe { bindings_c::ve_handle_release(runtime, wrong_owner, diagnostics.as_mut_buffer()) },
        ve_status_t::VE_STATUS_WRONG_OWNER
    );

    let stale_generation = ve_handle_t {
        generation: media.generation + 1,
        ..media
    };
    diagnostics.clear();
    assert_eq!(
        unsafe {
            bindings_c::ve_handle_release(runtime, stale_generation, diagnostics.as_mut_buffer())
        },
        ve_status_t::VE_STATUS_STALE_GENERATION
    );

    diagnostics.clear();
    assert_eq!(
        unsafe { bindings_c::ve_handle_release(runtime, media, diagnostics.as_mut_buffer()) },
        ve_status_t::VE_STATUS_OK
    );
    diagnostics.clear();
    assert_eq!(
        unsafe { bindings_c::ve_handle_release(runtime, media, diagnostics.as_mut_buffer()) },
        ve_status_t::VE_STATUS_OK
    );
    diagnostics.clear();
    assert_eq!(
        unsafe { bindings_c::ve_handle_release(runtime, media, diagnostics.as_mut_buffer()) },
        ve_status_t::VE_STATUS_DOUBLE_RELEASE
    );

    let descriptor = texture_descriptor("adapter-a", "device-a");
    let mut texture = ve_handle_t::default();
    diagnostics.clear();
    assert_eq!(
        unsafe {
            bindings_c::ve_handle_acquire(
                runtime,
                ve_handle_kind_t::VE_HANDLE_KIND_TEXTURE,
                &descriptor,
                0,
                &mut texture,
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
                texture,
                &descriptor,
                diagnostics.as_mut_buffer(),
            )
        },
        ve_status_t::VE_STATUS_OK
    );

    let wrong_device = texture_descriptor("adapter-b", "device-b");
    diagnostics.clear();
    assert_eq!(
        unsafe {
            bindings_c::ve_texture_handle_resolve(
                runtime,
                texture,
                &wrong_device,
                diagnostics.as_mut_buffer(),
            )
        },
        ve_status_t::VE_STATUS_WRONG_DEVICE
    );

    let mut wrong_pixel_format = texture_descriptor("adapter-a", "device-a");
    wrong_pixel_format.pixel_format = ve_pixel_format_t::VE_PIXEL_FORMAT_RGBA8;
    diagnostics.clear();
    assert_eq!(
        unsafe {
            bindings_c::ve_texture_handle_resolve(
                runtime,
                texture,
                &wrong_pixel_format,
                diagnostics.as_mut_buffer(),
            )
        },
        ve_status_t::VE_STATUS_TEXTURE_METADATA_MISMATCH
    );

    diagnostics.clear();
    assert_eq!(
        unsafe { bindings_c::ve_handle_release(runtime, texture, diagnostics.as_mut_buffer()) },
        ve_status_t::VE_STATUS_OK
    );

    diagnostics.clear();
    assert_eq!(
        unsafe { bindings_c::ve_runtime_close(runtime, diagnostics.as_mut_buffer()) },
        ve_status_t::VE_STATUS_OK
    );
}

#[test]
fn mobile_contract_handles_report_cascading_close_for_unreleased_tokens() {
    let (runtime, mut diagnostics) = runtime("mobile-contract-cascade");

    let mut media = ve_handle_t::default();
    let mut frame = ve_handle_t::default();
    let mut artifact = ve_handle_t::default();
    for (kind, out_handle) in [
        (ve_handle_kind_t::VE_HANDLE_KIND_MEDIA, &mut media),
        (ve_handle_kind_t::VE_HANDLE_KIND_FRAME, &mut frame),
        (ve_handle_kind_t::VE_HANDLE_KIND_ARTIFACT, &mut artifact),
    ] {
        diagnostics.clear();
        assert_eq!(
            unsafe {
                bindings_c::ve_handle_acquire(
                    runtime,
                    kind,
                    ptr::null(),
                    0,
                    out_handle,
                    diagnostics.as_mut_buffer(),
                )
            },
            ve_status_t::VE_STATUS_OK
        );
    }

    diagnostics.clear();
    assert_eq!(
        unsafe { bindings_c::ve_handle_release(runtime, frame, diagnostics.as_mut_buffer()) },
        ve_status_t::VE_STATUS_OK
    );

    diagnostics.clear();
    assert_eq!(
        unsafe { bindings_c::ve_runtime_close(runtime, diagnostics.as_mut_buffer()) },
        ve_status_t::VE_STATUS_OK
    );
    let close_json = diagnostics.as_str();
    assert!(close_json.contains("\"leaks\""));
    assert!(close_json.contains("\"cascadeClose\""));
    assert!(close_json.contains(&format!("\"id\":{}", media.id)));
    assert!(close_json.contains(&format!("\"id\":{}", artifact.id)));
    assert!(!close_json.contains(&format!("\"id\":{}", frame.id)));
}

fn runtime(label: &str) -> (ve_runtime_t, JsonBuffer) {
    let mut runtime = ve_runtime_t::default();
    let mut diagnostics = JsonBuffer::new(4096);
    let label = CString::new(label).expect("label should not contain nul");
    let config = bindings_c::ve_runtime_config_t {
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
        backend: ve_texture_backend_t::VE_TEXTURE_BACKEND_METAL_TEXTURE,
        adapter_id: adapter_id.into_raw(),
        device_id: device_id.into_raw(),
        width: 1920,
        height: 1080,
        pixel_format: ve_pixel_format_t::VE_PIXEL_FORMAT_BGRA8,
        color_primaries: ve_color_primaries_t::VE_COLOR_PRIMARIES_BT709,
        color_transfer: ve_color_transfer_t::VE_COLOR_TRANSFER_SRGB,
        color_matrix: ve_color_matrix_t::VE_COLOR_MATRIX_IDENTITY,
        color_range: ve_color_range_t::VE_COLOR_RANGE_FULL,
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
