use std::ffi::c_void;
use std::ptr::NonNull;

use raw_window_handle::AppKitWindowHandle;

use crate::gpu::surface::{
    NativeParentWindowHandle, PreviewSurfaceDiagnosticKind, PreviewSurfaceError,
};

pub fn parent_ns_view(value: u64) -> Result<NativeParentWindowHandle, PreviewSurfaceError> {
    if value == 0 {
        return Err(PreviewSurfaceError::new(
            PreviewSurfaceDiagnosticKind::MissingParentHandle,
            "macOS parent NSView must be nonzero",
        ));
    }
    Ok(NativeParentWindowHandle::MacosNsView(value))
}

pub fn raw_window_handle(
    handle: NativeParentWindowHandle,
) -> Result<AppKitWindowHandle, PreviewSurfaceError> {
    let NativeParentWindowHandle::MacosNsView(value) = handle else {
        return Err(PreviewSurfaceError::new(
            PreviewSurfaceDiagnosticKind::PlatformUnavailable,
            "expected a macOS NSView parent handle",
        ));
    };
    let ns_view = NonNull::new(value as *mut c_void).ok_or_else(|| {
        PreviewSurfaceError::new(
            PreviewSurfaceDiagnosticKind::MissingParentHandle,
            "macOS parent NSView must be non-null",
        )
    })?;
    Ok(AppKitWindowHandle::new(ns_view))
}
