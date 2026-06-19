use std::ffi::c_void;
use std::ptr::NonNull;

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{NSView, NSWindowOrderingMode};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use raw_window_handle::AppKitWindowHandle;
use raw_window_metal::Layer;

use crate::gpu::surface::{
    NativeParentWindowHandle, PreviewSurfaceBounds, PreviewSurfaceDiagnosticKind,
    PreviewSurfaceError,
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

#[derive(Debug)]
pub struct MacosWgpuSurfaceAttachment {
    parent_view: Retained<NSView>,
    child_view: Retained<NSView>,
    metal_layer: Layer,
}

// The binding registry is shared behind a Mutex. AppKit operations are still
// guarded at each call site with MainThreadMarker.
unsafe impl Send for MacosWgpuSurfaceAttachment {}

impl MacosWgpuSurfaceAttachment {
    pub fn new(
        parent_handle: NativeParentWindowHandle,
        bounds: PreviewSurfaceBounds,
    ) -> Result<Self, PreviewSurfaceError> {
        let mtm = require_main_thread()?;
        let NativeParentWindowHandle::MacosNsView(parent_handle) = parent_handle else {
            return Err(PreviewSurfaceError::new(
                PreviewSurfaceDiagnosticKind::PlatformUnavailable,
                "expected a macOS NSView parent handle",
            ));
        };
        if parent_handle == 0 {
            return Err(PreviewSurfaceError::new(
                PreviewSurfaceDiagnosticKind::MissingParentHandle,
                "macOS parent NSView must be nonzero",
            ));
        }
        let parent_view =
            unsafe { Retained::retain(parent_handle as *mut NSView) }.ok_or_else(|| {
                PreviewSurfaceError::new(
                    PreviewSurfaceDiagnosticKind::PlatformUnavailable,
                    "macOS WGPU presenter could not retain parent NSView",
                )
            })?;
        let child_view =
            NSView::initWithFrame(mtm.alloc(), ns_rect_for_bounds(&parent_view, bounds));
        child_view.setWantsLayer(true);
        child_view.setHidden(false);
        child_view.setAlphaValue(1.0);
        parent_view.addSubview_positioned_relativeTo(
            &child_view,
            NSWindowOrderingMode::Above,
            None,
        );
        let child_ptr = (&*child_view) as *const NSView as *mut c_void;
        let child_ns_view = NonNull::new(child_ptr).ok_or_else(|| {
            PreviewSurfaceError::new(
                PreviewSurfaceDiagnosticKind::MissingParentHandle,
                "macOS WGPU child NSView must be non-null",
            )
        })?;
        let metal_layer = unsafe { Layer::from_ns_view(child_ns_view) };
        Ok(Self {
            parent_view,
            child_view,
            metal_layer,
        })
    }

    pub fn raw_window_handle(&self) -> Result<AppKitWindowHandle, PreviewSurfaceError> {
        let child_ptr = (&*self.child_view) as *const NSView as *mut c_void;
        let ns_view = NonNull::new(child_ptr).ok_or_else(|| {
            PreviewSurfaceError::new(
                PreviewSurfaceDiagnosticKind::MissingParentHandle,
                "macOS WGPU child NSView must be non-null",
            )
        })?;
        Ok(AppKitWindowHandle::new(ns_view))
    }

    pub fn core_animation_layer(&self) -> *mut c_void {
        self.metal_layer.as_ptr().as_ptr()
    }

    pub fn update_bounds(
        &mut self,
        bounds: PreviewSurfaceBounds,
    ) -> Result<(), PreviewSurfaceError> {
        let _mtm = require_main_thread()?;
        self.child_view
            .setFrame(ns_rect_for_bounds(&self.parent_view, bounds));
        Ok(())
    }

    pub fn detach(&mut self) {
        if MainThreadMarker::new().is_none() {
            return;
        }
        self.child_view.removeFromSuperview();
    }
}

impl Drop for MacosWgpuSurfaceAttachment {
    fn drop(&mut self) {
        self.detach();
    }
}

fn require_main_thread() -> Result<MainThreadMarker, PreviewSurfaceError> {
    MainThreadMarker::new().ok_or_else(|| {
        PreviewSurfaceError::new(
            PreviewSurfaceDiagnosticKind::PlatformUnavailable,
            "macOS WGPU presenter must be used on the main thread",
        )
    })
}

fn ns_rect_for_bounds(parent_view: &NSView, bounds: PreviewSurfaceBounds) -> CGRect {
    let parent_bounds = parent_view.bounds();
    let parent_height = parent_bounds.size.height;
    let scale = (bounds.scale_factor_millis as f64 / 1000.0).max(0.001);
    let width = bounds.width as f64 / scale;
    let height = bounds.height as f64 / scale;
    let x = bounds.x as f64 / scale;
    let dom_y = bounds.y as f64 / scale;
    let y = (parent_height - dom_y - height).max(0.0);
    CGRect::new(
        CGPoint::new(x, y),
        CGSize::new(width.max(1.0), height.max(1.0)),
    )
}
