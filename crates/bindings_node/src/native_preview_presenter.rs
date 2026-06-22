use std::fmt;
use std::path::{Path, PathBuf};

use draft_model::{
    Draft, Material, MaterialKind, Microseconds, Segment, TextSegmentSource, TrackKind,
};
use project_store::resolve_material_uri;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NativePreviewSurfaceKind {
    WindowsHwnd,
    MacosNsView,
    Mock,
    Offscreen,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativePreviewSurfaceBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor_millis: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePreviewSurfaceAttach {
    pub kind: NativePreviewSurfaceKind,
    pub parent_handle: Option<u64>,
    pub bounds: NativePreviewSurfaceBounds,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativePreviewPresentationState {
    pub available: bool,
    pub backend: NativePreviewPresentationBackend,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unsupported_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<NativePreviewContentEvidence>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub surface_placement: Option<NativePreviewSurfacePlacementEvidence>,
}

impl NativePreviewPresentationState {
    pub fn unavailable(reason: impl Into<String>) -> Self {
        Self {
            available: false,
            backend: NativePreviewPresentationBackend::None,
            unsupported_reason: Some(reason.into()),
            evidence: None,
            surface_placement: None,
        }
    }

    pub fn native_video_bridge_diagnostic(evidence: Option<NativePreviewContentEvidence>) -> Self {
        Self {
            available: false,
            backend: NativePreviewPresentationBackend::NativeVideoBridge,
            unsupported_reason: Some(
                "native video bridge is diagnostic only; render graph GPU compositor is required for product playback"
                    .to_owned(),
            ),
            evidence,
            surface_placement: None,
        }
    }

    pub fn render_graph_gpu_available(evidence: Option<NativePreviewContentEvidence>) -> Self {
        Self {
            available: true,
            backend: NativePreviewPresentationBackend::RenderGraphGpu,
            unsupported_reason: None,
            evidence,
            surface_placement: None,
        }
    }

    pub fn with_surface_placement(
        mut self,
        surface_placement: Option<NativePreviewSurfacePlacementEvidence>,
    ) -> Self {
        self.surface_placement = surface_placement;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NativePreviewPresentationBackend {
    NativeVideoBridge,
    RenderGraphGpu,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativePreviewContentEvidence {
    pub source: NativePreviewContentEvidenceSource,
    pub digest: String,
    pub width: u32,
    pub height: u32,
    pub byte_count: usize,
    pub target_time_microseconds: u64,
    pub presented_frames: u32,
    pub submitted_draws: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub active_text_overlays: Vec<NativePreviewTextOverlayEvidence>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NativePreviewContentEvidenceSource {
    NativeVideoBridge,
    RenderGraphGpuComposited,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativePreviewTextOverlayEvidence {
    pub source: TextSegmentSource,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativePreviewSurfacePlacementEvidence {
    pub native_screen_rect: NativePreviewScreenRect,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drawable_lifecycle_diagnostic: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NativePreviewScreenRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePreviewSource {
    pub path: PathBuf,
    pub target_time: Microseconds,
    pub source_time: Microseconds,
    pub material_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativePreviewPresenterError {
    message: String,
}

impl NativePreviewPresenterError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for NativePreviewPresenterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for NativePreviewPresenterError {}

pub struct NativePreviewPresenter {
    state: NativePreviewPresenterState,
}

enum NativePreviewPresenterState {
    Detached,
    Unsupported {
        reason: String,
    },
    #[cfg(target_os = "macos")]
    Macos(macos::MacosAvPlayerLayerPresenter),
}

impl NativePreviewPresenter {
    pub fn detached() -> Self {
        Self {
            state: NativePreviewPresenterState::Detached,
        }
    }

    pub fn attach(
        &mut self,
        attach: NativePreviewSurfaceAttach,
    ) -> Result<(), NativePreviewPresenterError> {
        self.detach();
        self.state = create_platform_presenter(attach);
        match &self.state {
            NativePreviewPresenterState::Unsupported { reason } => {
                Err(NativePreviewPresenterError::new(reason.clone()))
            }
            _ => Ok(()),
        }
    }

    pub fn update_bounds(
        &mut self,
        bounds: NativePreviewSurfaceBounds,
    ) -> Result<(), NativePreviewPresenterError> {
        match &mut self.state {
            NativePreviewPresenterState::Detached => Err(NativePreviewPresenterError::new(
                "native preview presenter is not attached",
            )),
            NativePreviewPresenterState::Unsupported { reason } => {
                Err(NativePreviewPresenterError::new(reason.clone()))
            }
            #[cfg(target_os = "macos")]
            NativePreviewPresenterState::Macos(presenter) => presenter.update_bounds(bounds),
        }
    }

    pub fn update_draft(
        &mut self,
        draft: &Draft,
        bundle_path: Option<&Path>,
    ) -> Result<(), NativePreviewPresenterError> {
        let source = resolve_supported_source(draft, bundle_path)?;
        match &mut self.state {
            NativePreviewPresenterState::Detached => Err(NativePreviewPresenterError::new(
                "native preview presenter is not attached",
            )),
            NativePreviewPresenterState::Unsupported { reason } => {
                Err(NativePreviewPresenterError::new(reason.clone()))
            }
            #[cfg(target_os = "macos")]
            NativePreviewPresenterState::Macos(presenter) => presenter.update_source(source),
        }
    }

    pub fn seek(&mut self, target_time: Microseconds) -> Result<(), NativePreviewPresenterError> {
        match &mut self.state {
            NativePreviewPresenterState::Detached => Err(NativePreviewPresenterError::new(
                "native preview presenter is not attached",
            )),
            NativePreviewPresenterState::Unsupported { reason } => {
                Err(NativePreviewPresenterError::new(reason.clone()))
            }
            #[cfg(target_os = "macos")]
            NativePreviewPresenterState::Macos(presenter) => presenter.seek(target_time),
        }
    }

    pub fn play(&mut self) -> Result<(), NativePreviewPresenterError> {
        match &mut self.state {
            NativePreviewPresenterState::Detached => Err(NativePreviewPresenterError::new(
                "native preview presenter is not attached",
            )),
            NativePreviewPresenterState::Unsupported { reason } => {
                Err(NativePreviewPresenterError::new(reason.clone()))
            }
            #[cfg(target_os = "macos")]
            NativePreviewPresenterState::Macos(presenter) => presenter.play(),
        }
    }

    pub fn pause(&mut self) -> Result<(), NativePreviewPresenterError> {
        match &mut self.state {
            NativePreviewPresenterState::Detached
            | NativePreviewPresenterState::Unsupported { .. } => Ok(()),
            #[cfg(target_os = "macos")]
            NativePreviewPresenterState::Macos(presenter) => presenter.pause(),
        }
    }

    pub fn stop(&mut self) -> Result<(), NativePreviewPresenterError> {
        match &mut self.state {
            NativePreviewPresenterState::Detached
            | NativePreviewPresenterState::Unsupported { .. } => Ok(()),
            #[cfg(target_os = "macos")]
            NativePreviewPresenterState::Macos(presenter) => presenter.stop(),
        }
    }

    pub fn presentation_state(&mut self) -> NativePreviewPresentationState {
        match &mut self.state {
            NativePreviewPresenterState::Detached => NativePreviewPresentationState::unavailable(
                "native preview presenter is not attached",
            ),
            NativePreviewPresenterState::Unsupported { reason } => {
                NativePreviewPresentationState::unavailable(reason.clone())
            }
            #[cfg(target_os = "macos")]
            NativePreviewPresenterState::Macos(presenter) => presenter.presentation_state(),
        }
    }

    pub fn is_attached(&self) -> bool {
        match &self.state {
            NativePreviewPresenterState::Detached
            | NativePreviewPresenterState::Unsupported { .. } => false,
            #[cfg(target_os = "macos")]
            NativePreviewPresenterState::Macos(_) => true,
        }
    }

    pub fn detach(&mut self) {
        match &mut self.state {
            NativePreviewPresenterState::Detached
            | NativePreviewPresenterState::Unsupported { .. } => {}
            #[cfg(target_os = "macos")]
            NativePreviewPresenterState::Macos(presenter) => presenter.detach(),
        }
        self.state = NativePreviewPresenterState::Detached;
    }
}

impl Default for NativePreviewPresenter {
    fn default() -> Self {
        Self::detached()
    }
}

fn create_platform_presenter(attach: NativePreviewSurfaceAttach) -> NativePreviewPresenterState {
    match attach.kind {
        NativePreviewSurfaceKind::MacosNsView => create_macos_presenter(attach),
        NativePreviewSurfaceKind::WindowsHwnd => NativePreviewPresenterState::Unsupported {
            reason: "Windows native composited preview presenter is not implemented".to_owned(),
        },
        NativePreviewSurfaceKind::Mock | NativePreviewSurfaceKind::Offscreen => {
            NativePreviewPresenterState::Unsupported {
                reason: "mock/offscreen surfaces cannot satisfy product preview playback"
                    .to_owned(),
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn create_macos_presenter(attach: NativePreviewSurfaceAttach) -> NativePreviewPresenterState {
    match macos::MacosAvPlayerLayerPresenter::new(attach) {
        Ok(presenter) => NativePreviewPresenterState::Macos(presenter),
        Err(error) => NativePreviewPresenterState::Unsupported {
            reason: error.to_string(),
        },
    }
}

#[cfg(not(target_os = "macos"))]
fn create_macos_presenter(_attach: NativePreviewSurfaceAttach) -> NativePreviewPresenterState {
    NativePreviewPresenterState::Unsupported {
        reason: "macOS native composited preview presenter requires target_os=macos".to_owned(),
    }
}

fn resolve_supported_source(
    draft: &Draft,
    bundle_path: Option<&Path>,
) -> Result<NativePreviewSource, NativePreviewPresenterError> {
    let mut candidates: Vec<(&Segment, &Material)> = Vec::new();
    for track in &draft.tracks {
        if track.kind != TrackKind::Video || track.muted || !track.visible {
            continue;
        }
        for segment in &track.segments {
            let material = draft
                .materials
                .iter()
                .find(|material| material.material_id == segment.material_id)
                .ok_or_else(|| {
                    NativePreviewPresenterError::new(format!(
                        "video segment references missing material {}",
                        segment.material_id.as_str()
                    ))
                })?;
            if material.kind == MaterialKind::Video {
                candidates.push((segment, material));
            }
        }
    }

    if candidates.is_empty() {
        return Err(NativePreviewPresenterError::new(
            "no visible video segment is available for native preview playback",
        ));
    }
    if candidates.len() != 1 {
        return Err(NativePreviewPresenterError::new(
            "native preview playback currently requires exactly one visible video segment",
        ));
    }

    let (segment, material) = candidates[0];
    if !segment.filters.is_empty()
        || segment.transition.is_some()
        || !segment.keyframes.is_empty()
        || segment.text.is_some()
    {
        return Err(NativePreviewPresenterError::new(
            "native preview playback is not connected for filtered, transitioned, animated, or text-bearing segments",
        ));
    }

    let bundle_path = bundle_path.ok_or_else(|| {
        NativePreviewPresenterError::new("bundlePath is required to resolve preview material URIs")
    })?;
    let path = resolve_material_uri(bundle_path, &material.uri)
        .map_err(|error| NativePreviewPresenterError::new(error.to_string()))?
        .ok_or_else(|| {
            NativePreviewPresenterError::new(format!(
                "material URI is not a local file path: {}",
                material.uri
            ))
        })?;
    if !path.is_file() {
        return Err(NativePreviewPresenterError::new(format!(
            "material path does not exist or is not a file: {}",
            path.display()
        )));
    }

    Ok(NativePreviewSource {
        path,
        target_time: segment.target_timerange.start,
        source_time: segment.source_timerange.start,
        material_id: material.material_id.as_str().to_owned(),
    })
}

#[cfg(target_os = "macos")]
mod macos {
    use super::{
        NativePreviewContentEvidence, NativePreviewContentEvidenceSource,
        NativePreviewPresentationState, NativePreviewPresenterError, NativePreviewSource,
        NativePreviewSurfaceAttach, NativePreviewSurfaceBounds,
    };
    use draft_model::Microseconds;
    use objc2::MainThreadMarker;
    use objc2::rc::Retained;
    use objc2_app_kit::NSView;
    use objc2_av_foundation::{AVLayerVideoGravityResizeAspect, AVPlayer, AVPlayerLayer};
    use objc2_core_foundation::{CGPoint, CGRect, CGSize};
    use objc2_core_graphics::{
        CGBitmapContextCreate, CGColorSpace, CGContext, CGImage, CGImageAlphaInfo,
        CGImageByteOrderInfo, CGWindowImageOption, CGWindowListOption,
    };
    use objc2_core_media::CMTime;
    use objc2_foundation::{NSString, NSURL};
    use std::ffi::c_void;
    use std::path::Path;

    pub struct MacosAvPlayerLayerPresenter {
        parent_view: Retained<NSView>,
        child_view: Retained<NSView>,
        player: Option<Retained<AVPlayer>>,
        player_layer: Retained<AVPlayerLayer>,
        bounds: NativePreviewSurfaceBounds,
        source: Option<NativePreviewSource>,
        last_evidence: Option<NativePreviewContentEvidence>,
    }

    // The global binding registry requires Send. Every presenter method still enforces
    // AppKit/AVFoundation main-thread access before touching native objects.
    unsafe impl Send for MacosAvPlayerLayerPresenter {}

    impl MacosAvPlayerLayerPresenter {
        pub fn new(
            attach: NativePreviewSurfaceAttach,
        ) -> Result<Self, NativePreviewPresenterError> {
            let mtm = require_main_thread()?;
            let parent_handle = attach.parent_handle.ok_or_else(|| {
                NativePreviewPresenterError::new(
                    "macOS preview presenter requires an NSView parent handle",
                )
            })?;
            if parent_handle == 0 {
                return Err(NativePreviewPresenterError::new(
                    "macOS preview presenter received an empty NSView parent handle",
                ));
            }

            let parent_view = unsafe { Retained::retain(parent_handle as *mut NSView) }
                .ok_or_else(|| {
                    NativePreviewPresenterError::new(
                        "macOS preview presenter could not retain parent NSView",
                    )
                })?;
            let frame = ns_rect_for_content_local_bounds(&parent_view, attach.bounds);
            let child_view = NSView::initWithFrame(mtm.alloc(), frame);
            let player_layer = unsafe { AVPlayerLayer::layer() };
            unsafe {
                if let Some(gravity) = AVLayerVideoGravityResizeAspect {
                    player_layer.setVideoGravity(gravity);
                }
                player_layer.setFrame(cg_rect_for_child_bounds(attach.bounds));
            }
            child_view.setWantsLayer(true);
            child_view.setLayer(Some(&player_layer));
            parent_view.addSubview(&child_view);

            Ok(Self {
                parent_view,
                child_view,
                player: None,
                player_layer,
                bounds: attach.bounds,
                source: None,
                last_evidence: None,
            })
        }

        pub fn update_bounds(
            &mut self,
            bounds: NativePreviewSurfaceBounds,
        ) -> Result<(), NativePreviewPresenterError> {
            let _mtm = require_main_thread()?;
            self.bounds = bounds;
            self.child_view
                .setFrame(ns_rect_for_content_local_bounds(&self.parent_view, bounds));
            self.player_layer.setFrame(cg_rect_for_child_bounds(bounds));
            Ok(())
        }

        pub fn update_source(
            &mut self,
            source: NativePreviewSource,
        ) -> Result<(), NativePreviewPresenterError> {
            let mtm = require_main_thread()?;
            if self
                .source
                .as_ref()
                .map(|current| current.path.as_path() == source.path.as_path())
                .unwrap_or(false)
            {
                self.source = Some(source);
                return Ok(());
            }

            let url = file_url_for_path(&source.path)?;
            let player = unsafe { AVPlayer::playerWithURL(&url, mtm) };
            unsafe {
                self.player_layer.setPlayer(Some(&player));
            }
            self.player = Some(player);
            self.source = Some(source);
            self.last_evidence = None;
            Ok(())
        }

        pub fn seek(
            &mut self,
            target_time: Microseconds,
        ) -> Result<(), NativePreviewPresenterError> {
            let _mtm = require_main_thread()?;
            let Some(player) = self.player.as_ref() else {
                return Err(NativePreviewPresenterError::new(
                    "native preview presenter has no playable source",
                ));
            };
            let source_time = self.source_time_for_target(target_time);
            unsafe {
                player.seekToTime(cmtime_from_microseconds(source_time));
            }
            Ok(())
        }

        pub fn play(&mut self) -> Result<(), NativePreviewPresenterError> {
            let _mtm = require_main_thread()?;
            let Some(player) = self.player.as_ref() else {
                return Err(NativePreviewPresenterError::new(
                    "native preview presenter has no playable source",
                ));
            };
            unsafe {
                player.play();
            }
            Ok(())
        }

        pub fn pause(&mut self) -> Result<(), NativePreviewPresenterError> {
            let _mtm = require_main_thread()?;
            if let Some(player) = self.player.as_ref() {
                unsafe {
                    player.pause();
                }
            }
            Ok(())
        }

        pub fn stop(&mut self) -> Result<(), NativePreviewPresenterError> {
            self.pause()?;
            if let Some(player) = self.player.as_ref() {
                unsafe {
                    player.seekToTime(cmtime_from_microseconds(Microseconds::ZERO));
                }
            }
            Ok(())
        }

        pub fn presentation_state(&mut self) -> NativePreviewPresentationState {
            if let Err(error) = require_main_thread() {
                return NativePreviewPresentationState::unavailable(error.to_string());
            }
            let Some(player) = self.player.as_ref() else {
                return NativePreviewPresentationState::unavailable(
                    "native preview presenter has no playable source",
                );
            };
            if unsafe { self.player_layer.isReadyForDisplay() } {
                if let Some(evidence) = self.capture_composited_evidence(player) {
                    self.last_evidence = Some(evidence);
                }
            }
            NativePreviewPresentationState::native_video_bridge_diagnostic(
                self.last_evidence.clone(),
            )
        }

        pub fn detach(&mut self) {
            if require_main_thread().is_err() {
                return;
            }
            unsafe {
                self.player_layer.setPlayer(None);
                self.child_view.removeFromSuperview();
            }
            self.player = None;
            self.source = None;
            self.last_evidence = None;
        }

        fn source_time_for_target(&self, target_time: Microseconds) -> Microseconds {
            let Some(source) = self.source.as_ref() else {
                return target_time;
            };
            let relative = target_time.get().saturating_sub(source.target_time.get());
            Microseconds::new(source.source_time.get().saturating_add(relative))
        }

        fn target_time_from_player(&self, player: &AVPlayer) -> Microseconds {
            let source_time = unsafe { microseconds_from_cmtime(player.currentTime()) };
            let Some(source) = self.source.as_ref() else {
                return source_time;
            };
            let relative = source_time.get().saturating_sub(source.source_time.get());
            Microseconds::new(source.target_time.get().saturating_add(relative))
        }

        fn capture_composited_evidence(
            &self,
            player: &AVPlayer,
        ) -> Option<NativePreviewContentEvidence> {
            let window = self.child_view.window()?;
            let window_id = u32::try_from(window.windowNumber()).ok()?;
            if window_id == 0 {
                return None;
            }

            let capture_rect = child_view_capture_rect(&self.child_view, &window)?;
            let image = capture_window_region(capture_rect, window_id)?;
            let source_width = CGImage::width(Some(&image)) as u32;
            let source_height = CGImage::height(Some(&image)) as u32;
            if source_width == 0 || source_height == 0 {
                return None;
            }

            let width = source_width.min(960);
            let height = source_height.min(540);
            let bytes_per_pixel = 4usize;
            let bytes_per_row = width as usize * bytes_per_pixel;
            let byte_count = bytes_per_row.checked_mul(height as usize)?;
            let mut pixels = vec![0_u8; byte_count];
            let color_space = CGColorSpace::new_device_rgb()?;
            let bitmap_info =
                CGImageAlphaInfo::PremultipliedFirst.0 | CGImageByteOrderInfo::Order32Little.0;
            let context = unsafe {
                CGBitmapContextCreate(
                    pixels.as_mut_ptr().cast::<c_void>(),
                    width as usize,
                    height as usize,
                    8,
                    bytes_per_row,
                    Some(&color_space),
                    bitmap_info,
                )?
            };
            CGContext::draw_image(
                Some(&context),
                CGRect::new(CGPoint::ZERO, CGSize::new(width as f64, height as f64)),
                Some(&image),
            );
            if pixels.iter().all(|byte| *byte == 0) {
                return None;
            }

            let target_time = self.target_time_from_player(player);
            Some(NativePreviewContentEvidence {
                source: NativePreviewContentEvidenceSource::NativeVideoBridge,
                digest: blake3::hash(&pixels).to_hex().to_string(),
                width,
                height,
                byte_count,
                target_time_microseconds: target_time.get(),
                presented_frames: 0,
                submitted_draws: 0,
                active_text_overlays: Vec::new(),
            })
        }
    }

    fn child_view_capture_rect(
        child_view: &NSView,
        window: &objc2_app_kit::NSWindow,
    ) -> Option<CGRect> {
        let bounds = child_view.bounds();
        let window_rect = child_view.convertRect_toView(bounds, None);
        let screen_rect = window.convertRectToScreen(window_rect);
        let x = screen_rect.origin.x.max(0.0);
        let y = screen_rect.origin.y.max(0.0);
        let width = screen_rect.size.width.max(1.0);
        let height = screen_rect.size.height.max(1.0);

        Some(CGRect::new(
            CGPoint::new(x, y.max(0.0)),
            CGSize::new(width, height),
        ))
    }

    #[allow(deprecated)]
    fn capture_window_region(
        rect: CGRect,
        window_id: u32,
    ) -> Option<objc2_core_foundation::CFRetained<CGImage>> {
        objc2_core_graphics::CGWindowListCreateImage(
            rect,
            CGWindowListOption::OptionIncludingWindow,
            window_id,
            CGWindowImageOption::BoundsIgnoreFraming | CGWindowImageOption::BestResolution,
        )
    }

    fn require_main_thread() -> Result<MainThreadMarker, NativePreviewPresenterError> {
        MainThreadMarker::new().ok_or_else(|| {
            NativePreviewPresenterError::new(
                "macOS native preview presenter must be used on the main thread",
            )
        })
    }

    fn file_url_for_path(path: &Path) -> Result<Retained<NSURL>, NativePreviewPresenterError> {
        let path = path.to_str().ok_or_else(|| {
            NativePreviewPresenterError::new("material path is not valid UTF-8 for NSURL")
        })?;
        let path = NSString::from_str(path);
        Ok(NSURL::fileURLWithPath(&path))
    }

    fn ns_rect_for_content_local_bounds(
        parent_view: &NSView,
        bounds: NativePreviewSurfaceBounds,
    ) -> CGRect {
        let parent_bounds = parent_view.bounds();
        let width = bounds.width as f64;
        let height = bounds.height as f64;
        let x = bounds.x as f64;
        let y = if parent_view.isFlipped() {
            bounds.y as f64
        } else {
            parent_bounds.size.height - bounds.y as f64 - height
        };
        CGRect::new(
            CGPoint::new(x, y),
            CGSize::new(width.max(1.0), height.max(1.0)),
        )
    }

    fn cg_rect_for_child_bounds(bounds: NativePreviewSurfaceBounds) -> CGRect {
        let width = bounds.width as f64;
        let height = bounds.height as f64;
        CGRect::new(CGPoint::ZERO, CGSize::new(width.max(1.0), height.max(1.0)))
    }

    fn cmtime_from_microseconds(value: Microseconds) -> CMTime {
        unsafe { CMTime::new(value.get().min(i64::MAX as u64) as i64, 1_000_000) }
    }

    unsafe fn microseconds_from_cmtime(time: CMTime) -> Microseconds {
        let seconds = unsafe { time.seconds() };
        if seconds.is_finite() && seconds > 0.0 {
            Microseconds::new((seconds * 1_000_000.0).round().max(0.0) as u64)
        } else {
            Microseconds::new(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use draft_model::{
        DraftCanvasConfig, DraftMetadata, DraftSchemaVersion, MainTrackMagnet, MaterialId,
        MaterialMetadata, MaterialStatus, SegmentId, SourceTimerange, TargetTimerange, Track,
        TrackId,
    };
    use tempfile::tempdir;

    #[test]
    fn source_resolution_requires_bundle_path_for_relative_uri() {
        let draft = single_video_draft("media/clip.mp4");
        let error = resolve_supported_source(&draft, None).expect_err("bundle path is required");

        assert!(error.message().contains("bundlePath"));
    }

    #[test]
    fn native_player_presentation_serializes_as_native_video_not_gpu_compositor() {
        let state = NativePreviewPresentationState::native_video_bridge_diagnostic(Some(
            NativePreviewContentEvidence {
                source: NativePreviewContentEvidenceSource::NativeVideoBridge,
                digest: "digest".to_owned(),
                width: 16,
                height: 9,
                byte_count: 16 * 9 * 4,
                target_time_microseconds: 123_000,
                presented_frames: 0,
                submitted_draws: 0,
                active_text_overlays: Vec::new(),
            },
        ));

        let json = serde_json::to_value(state).expect("native preview state serializes");

        assert_eq!(json["available"], false);
        assert_eq!(json["backend"], "nativeVideoBridge");
        assert_eq!(json["evidence"]["source"], "nativeVideoBridge");
        assert!(
            json["unsupportedReason"]
                .as_str()
                .expect("unsupported reason")
                .contains("diagnostic only")
        );
    }

    #[test]
    fn source_resolution_rejects_multiple_visible_video_segments() {
        let mut draft = single_video_draft("/tmp/clip.mp4");
        let second = Segment::new(
            SegmentId::from("segment-2"),
            MaterialId::from("video-material"),
            SourceTimerange::new(0, 1_000_000),
            TargetTimerange::new(1_000_000, 1_000_000),
        );
        draft.tracks[0].segments.push(second);
        let bundle = tempdir().expect("tempdir");
        let error =
            resolve_supported_source(&draft, Some(bundle.path())).expect_err("single segment only");

        assert!(
            error
                .message()
                .contains("exactly one visible video segment")
        );
    }

    #[test]
    fn source_resolution_accepts_single_local_video_segment() {
        let bundle = tempdir().expect("tempdir");
        let media_dir = bundle.path().join("media");
        std::fs::create_dir_all(&media_dir).expect("media dir");
        std::fs::write(media_dir.join("clip.mp4"), b"not decoded by this test").expect("fixture");
        let draft = single_video_draft("media/clip.mp4");
        let source = resolve_supported_source(&draft, Some(bundle.path())).expect("source");

        assert_eq!(source.path, media_dir.join("clip.mp4"));
        assert_eq!(source.material_id, "video-material");
    }

    fn single_video_draft(uri: &str) -> Draft {
        let material = Material {
            material_id: MaterialId::from("video-material"),
            kind: MaterialKind::Video,
            uri: uri.to_owned(),
            display_name: "clip.mp4".to_owned(),
            metadata: MaterialMetadata::empty(),
            status: MaterialStatus::Available,
        };
        let segment = Segment {
            segment_id: SegmentId::from("segment-1"),
            material_id: material.material_id.clone(),
            source_timerange: SourceTimerange::new(0, 1_000_000),
            target_timerange: TargetTimerange::new(0, 1_000_000),
            main_track_magnet: MainTrackMagnet::disabled(),
            keyframes: Vec::new(),
            filters: Vec::new(),
            transition: None,
            text: None,
            volume: Default::default(),
            audio: Default::default(),
            visual: Default::default(),
        };
        let mut track = Track::new(TrackId::from("video-track-1"), TrackKind::Video, "Video 1");
        track.segments.push(segment);
        Draft {
            schema_version: DraftSchemaVersion::current(),
            draft_id: "draft-1".into(),
            metadata: DraftMetadata::new("draft"),
            canvas_config: DraftCanvasConfig::mvp_default(),
            materials: vec![material],
            tracks: vec![track],
        }
    }
}
