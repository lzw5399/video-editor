use std::ffi::c_void;
use std::ptr::NonNull;

use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSApplication, NSBackingStoreType, NSScreenSaverWindowLevel, NSView, NSWindow,
    NSWindowOcclusionState, NSWindowStyleMask,
};
use objc2_core_foundation::{CGPoint, CGRect, CGSize};
use objc2_foundation::{NSDate, NSRunLoop};
use objc2_quartz_core::{CAMetalLayer, CATransaction};
use raw_window_handle::AppKitWindowHandle;

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
    parent_window: Retained<NSWindow>,
    child_window: Retained<NSWindow>,
    child_view: Retained<NSView>,
    metal_layer: Retained<CAMetalLayer>,
    prepare_count: u64,
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
        let parent_window = ensure_parent_window_visible(&parent_view)?;
        let child_view = NSView::initWithFrame(mtm.alloc(), content_rect_for_bounds(bounds));
        let metal_layer = CAMetalLayer::new();
        child_view.setWantsLayer(true);
        child_view.setLayer(Some(&metal_layer));
        child_view.setHidden(false);
        child_view.setAlphaValue(1.0);
        child_view.setPostsFrameChangedNotifications(true);
        let child_window = unsafe {
            NSWindow::initWithContentRect_styleMask_backing_defer(
                mtm.alloc(),
                screen_rect_for_bounds(&parent_view, bounds),
                NSWindowStyleMask::Borderless,
                NSBackingStoreType::Buffered,
                false,
            )
        };
        unsafe {
            child_window.setReleasedWhenClosed(false);
        }
        child_window.setContentView(Some(&child_view));
        child_window.setOpaque(true);
        child_window.setHasShadow(false);
        child_window.setIgnoresMouseEvents(true);
        child_window.setLevel(NSScreenSaverWindowLevel);
        child_window.orderFrontRegardless();
        let child_ptr = (&*child_view) as *const NSView as *mut c_void;
        let child_ns_view = NonNull::new(child_ptr).ok_or_else(|| {
            PreviewSurfaceError::new(
                PreviewSurfaceDiagnosticKind::MissingParentHandle,
                "macOS WGPU child NSView must be non-null",
            )
        })?;
        let _ = child_ns_view;
        configure_metal_layer(&metal_layer, bounds);
        child_view.setNeedsDisplay(true);
        child_view.displayIfNeededIgnoringOpacity();
        commit_appkit_core_animation(&parent_window, Some(&child_window));
        Ok(Self {
            parent_view,
            parent_window,
            child_window,
            child_view,
            metal_layer,
            prepare_count: 1,
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
        Retained::as_ptr(&self.metal_layer).cast::<c_void>() as *mut c_void
    }

    pub fn update_bounds(
        &mut self,
        bounds: PreviewSurfaceBounds,
    ) -> Result<(), PreviewSurfaceError> {
        let _mtm = require_main_thread()?;
        ensure_window_visible(&self.parent_window);
        self.child_window
            .setFrame_display(screen_rect_for_bounds(&self.parent_view, bounds), true);
        self.child_window.setLevel(NSScreenSaverWindowLevel);
        self.child_window.orderFrontRegardless();
        self.child_view.setFrame(content_rect_for_bounds(bounds));
        self.child_view.setHidden(false);
        self.child_view.setAlphaValue(1.0);
        configure_metal_layer(&self.metal_layer, bounds);
        self.child_view.setNeedsDisplay(true);
        self.child_view.displayIfNeededIgnoringOpacity();
        commit_appkit_core_animation(&self.parent_window, Some(&self.child_window));
        Ok(())
    }

    pub fn prepare_for_present(
        &mut self,
        bounds: PreviewSurfaceBounds,
    ) -> Result<(), PreviewSurfaceError> {
        let _mtm = require_main_thread()?;
        ensure_window_visible(&self.parent_window);
        self.child_window
            .setFrame_display(screen_rect_for_bounds(&self.parent_view, bounds), true);
        self.child_window.setLevel(NSScreenSaverWindowLevel);
        self.child_window.orderFrontRegardless();
        self.child_view.setHidden(false);
        self.child_view.setAlphaValue(1.0);
        self.child_view.setFrame(content_rect_for_bounds(bounds));
        configure_metal_layer(&self.metal_layer, bounds);
        self.child_view.displayIfNeededIgnoringOpacity();
        self.prepare_count = self.prepare_count.saturating_add(1);
        commit_appkit_core_animation(&self.parent_window, Some(&self.child_window));
        Ok(())
    }

    pub fn drawable_lifecycle_diagnostic(&self) -> String {
        let parent_window_visible = self.parent_window.isVisible();
        let parent_window_occlusion_visible = self
            .parent_window
            .occlusionState()
            .contains(NSWindowOcclusionState::Visible);
        let child_window_visible = self.child_window.isVisible();
        let child_window_occlusion_visible = self
            .child_window
            .occlusionState()
            .contains(NSWindowOcclusionState::Visible);
        let child_has_parent = self.child_window.parentWindow().is_some();
        let parent_view_bounds = self.parent_view.bounds();
        let child_view_frame = self.child_view.frame();
        let child_view_bounds = self.child_view.bounds();
        let parent_window_frame = self.parent_window.frame();
        let child_window_frame = self.child_window.frame();
        let layer_bounds = self.metal_layer.bounds();
        let drawable_size = self.metal_layer.drawableSize();
        format!(
            "drawableLifecycle{{attachmentStrategy=topLevelOverlayWindow, prepareCount={}, parentWindowVisible={}, parentWindowOcclusionVisible={}, childWindowVisible={}, childWindowOcclusionVisible={}, childHasParent={}, parentViewHidden={}, parentViewHiddenOrAncestor={}, childViewHidden={}, childViewHiddenOrAncestor={}, childViewAlpha={:.3}, childWindowAlpha={:.3}, layerHidden={}, parentWindowFrame={}, parentViewBounds={}, childWindowFrame={}, childViewFrame={}, childViewBounds={}, layerBounds={}, drawableSize={} }}",
            self.prepare_count,
            parent_window_visible,
            parent_window_occlusion_visible,
            child_window_visible,
            child_window_occlusion_visible,
            child_has_parent,
            self.parent_view.isHidden(),
            self.parent_view.isHiddenOrHasHiddenAncestor(),
            self.child_view.isHidden(),
            self.child_view.isHiddenOrHasHiddenAncestor(),
            self.child_view.alphaValue(),
            self.child_window.alphaValue(),
            self.metal_layer.isHidden(),
            format_rect(parent_window_frame),
            format_rect(parent_view_bounds),
            format_rect(child_window_frame),
            format_rect(child_view_frame),
            format_rect(child_view_bounds),
            format_rect(layer_bounds),
            format_size(drawable_size),
        )
    }

    pub fn detach(&mut self) {
        if MainThreadMarker::new().is_none() {
            return;
        }
        self.child_window.orderOut(None);
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

fn ensure_parent_window_visible(
    parent_view: &NSView,
) -> Result<Retained<NSWindow>, PreviewSurfaceError> {
    let mtm = require_main_thread()?;
    let app = NSApplication::sharedApplication(mtm);
    app.unhideWithoutActivation();
    #[allow(deprecated)]
    app.activateIgnoringOtherApps(true);
    let window = parent_view.window().ok_or_else(|| {
        PreviewSurfaceError::new(
            PreviewSurfaceDiagnosticKind::PlatformUnavailable,
            "macOS WGPU parent NSView is not attached to an NSWindow",
        )
    })?;
    ensure_window_visible(&window);
    Ok(window)
}

fn ensure_window_visible(window: &NSWindow) {
    if !window.isVisible() {
        window.orderFrontRegardless();
    }
    if !window
        .occlusionState()
        .contains(NSWindowOcclusionState::Visible)
    {
        window.makeKeyAndOrderFront(None);
        window.orderFrontRegardless();
    }
}

fn configure_metal_layer(metal_layer: &CAMetalLayer, bounds: PreviewSurfaceBounds) {
    let scale = (bounds.scale_factor_millis as f64 / 1000.0).max(0.001);
    let logical_size = CGSize::new(
        (bounds.width as f64 / scale).max(1.0),
        (bounds.height as f64 / scale).max(1.0),
    );
    let drawable_size = CGSize::new(bounds.width.max(1) as f64, bounds.height.max(1) as f64);
    metal_layer.setBounds(CGRect::new(CGPoint::new(0.0, 0.0), logical_size));
    metal_layer.setPosition(CGPoint::new(
        logical_size.width / 2.0,
        logical_size.height / 2.0,
    ));
    metal_layer.setContentsScale(scale);
    metal_layer.setDrawableSize(drawable_size);
    metal_layer.setPresentsWithTransaction(false);
    metal_layer.setFramebufferOnly(true);
    metal_layer.setHidden(false);
    metal_layer.setZPosition(1.0);
    metal_layer.setNeedsDisplayOnBoundsChange(true);
    metal_layer.setNeedsDisplay();
}

fn commit_appkit_core_animation(parent_window: &NSWindow, child_window: Option<&NSWindow>) {
    parent_window.displayIfNeeded();
    if let Some(child_window) = child_window {
        child_window.displayIfNeeded();
    }
    #[allow(deprecated)]
    {
        parent_window.flushWindowIfNeeded();
        if let Some(child_window) = child_window {
            child_window.flushWindowIfNeeded();
        }
    }
    CATransaction::flush();
    let run_loop = NSRunLoop::currentRunLoop();
    let limit = NSDate::dateWithTimeIntervalSinceNow(0.05);
    run_loop.runUntilDate(&limit);
    CATransaction::flush();
}

fn format_rect(rect: CGRect) -> String {
    format!(
        "{{x={:.2},y={:.2},w={:.2},h={:.2}}}",
        rect.origin.x, rect.origin.y, rect.size.width, rect.size.height
    )
}

fn format_size(size: CGSize) -> String {
    format!("{{w={:.2},h={:.2}}}", size.width, size.height)
}

fn screen_rect_for_bounds(parent_view: &NSView, bounds: PreviewSurfaceBounds) -> CGRect {
    let rect = ns_rect_for_bounds(parent_view, bounds);
    let rect_in_window = parent_view.convertRect_toView(rect, None);
    if let Some(window) = parent_view.window() {
        window.convertRectToScreen(rect_in_window)
    } else {
        rect_in_window
    }
}

fn content_rect_for_bounds(bounds: PreviewSurfaceBounds) -> CGRect {
    let scale = (bounds.scale_factor_millis as f64 / 1000.0).max(0.001);
    CGRect::new(
        CGPoint::new(0.0, 0.0),
        CGSize::new(
            (bounds.width as f64 / scale).max(1.0),
            (bounds.height as f64 / scale).max(1.0),
        ),
    )
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
