use std::num::NonZeroIsize;

use raw_window_handle::{Win32WindowHandle, WindowsDisplayHandle};

use crate::gpu::surface::{
    NativeParentWindowHandle, PreviewSurfaceDiagnosticKind, PreviewSurfaceError,
};

pub fn parent_hwnd(value: u64) -> Result<NativeParentWindowHandle, PreviewSurfaceError> {
    if value == 0 {
        return Err(PreviewSurfaceError::new(
            PreviewSurfaceDiagnosticKind::MissingParentHandle,
            "windows parent HWND must be nonzero",
        ));
    }
    Ok(NativeParentWindowHandle::WindowsHwnd(value))
}

pub fn raw_window_handle(
    handle: NativeParentWindowHandle,
) -> Result<Win32WindowHandle, PreviewSurfaceError> {
    let NativeParentWindowHandle::WindowsHwnd(value) = handle else {
        return Err(PreviewSurfaceError::new(
            PreviewSurfaceDiagnosticKind::PlatformUnavailable,
            "expected a Windows HWND parent handle",
        ));
    };
    let hwnd = isize::try_from(value)
        .ok()
        .and_then(NonZeroIsize::new)
        .ok_or_else(|| {
            PreviewSurfaceError::new(
                PreviewSurfaceDiagnosticKind::MissingParentHandle,
                "windows parent HWND must fit a nonzero isize",
            )
        })?;
    Ok(Win32WindowHandle::new(hwnd))
}

pub fn raw_display_handle() -> WindowsDisplayHandle {
    WindowsDisplayHandle::new()
}
