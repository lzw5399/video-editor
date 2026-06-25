use std::{
    collections::{BTreeMap, BTreeSet},
    path::PathBuf,
};

use draft_import::{
    AdaptationCategory, AdaptationReport, AdaptationReportItem, AdaptationSeverity,
    AdaptationStatus, AdaptationTargetKind, AdaptationTargetRef, DraftImportPlan,
    DraftImportPlanSchemaVersion, ExternalProvenanceRef, ImportMaterialPlan, ImportTrackPlan,
    LocalizedResource, LocalizedResourceManifest, LocalizedResourceStatus,
    ResourceLocalizationMode, ResourceLocalizationRequest, TemplateResourceKind,
    TemplateResourceRef, localize_template_resources, validate_import_plan,
};
use draft_model::{
    AudioFade, CanvasAdaptationPolicy, CanvasAspectRatio, CanvasAspectRatioPreset,
    CanvasBackground, DraftCanvasConfig, Filter, Keyframe, KeyframeEasing, KeyframeInterpolation,
    KeyframeProperty, KeyframeValue, MainTrackMagnet, Material, MaterialKind, MaterialMetadata,
    MaterialStatus, Microseconds, RationalFrameRate, RetimeMode, Segment, SegmentAnchor,
    SegmentAudio, SegmentCrop, SegmentFitMode, SegmentOpacity, SegmentPosition, SegmentRetiming,
    SegmentRotation, SegmentScale, SegmentTransform, SegmentVisual, SourceTimerange, SpeedRatio,
    TargetTimerange, TextAlignment, TextBox, TextFont, TextLayoutRegion, TextSegment, TextStyle,
    TextWrapping, Track, TrackKind, Transition,
};
use serde_json::{Map, Value};

use crate::{
    AdapterKaipaiError, DirectMaterialRef, FormulaResourceKind, FormulaResourceRef,
    FormulaSourceMedia, KaipaiFormulaBundle,
};

#[derive(Debug, Clone)]
pub struct KaipaiImportOptions {
    pub bundle_path: PathBuf,
    pub source_root: PathBuf,
    pub import_id: String,
    pub generated_at: Option<String>,
    pub resource_mode: ResourceLocalizationMode,
    pub verify_resource_sha256: bool,
}

impl KaipaiImportOptions {
    pub fn new(
        bundle_path: impl Into<PathBuf>,
        source_root: impl Into<PathBuf>,
        import_id: impl Into<String>,
    ) -> Self {
        Self {
            bundle_path: bundle_path.into(),
            source_root: source_root.into(),
            import_id: import_id.into(),
            generated_at: None,
            resource_mode: ResourceLocalizationMode::CopyRenderableResources,
            verify_resource_sha256: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KaipaiMappedFixture {
    pub plan: DraftImportPlan,
    pub report: AdaptationReport,
    pub localized_resources: LocalizedResourceManifest,
}

pub fn map_kaipai_bundle_to_import_plan(
    bundle: &KaipaiFormulaBundle,
    options: KaipaiImportOptions,
) -> Result<KaipaiMappedFixture, AdapterKaipaiError> {
    bundle.validate()?;
    let localized = localize_template_resources(ResourceLocalizationRequest {
        bundle_path: options.bundle_path.clone(),
        source_root: options.source_root.clone(),
        import_id: options.import_id.clone(),
        resources: template_resource_refs(bundle, options.verify_resource_sha256),
        mode: options.resource_mode,
    })?;

    let context = MapperContext::new(bundle, &localized.manifest);
    let formula = bundle
        .formula
        .as_object()
        .ok_or_else(|| mapper_error("formula", "formula evidence must be a JSON object"))?;
    let canvas_config = map_canvas_config(formula)?;

    let mut state = MapperState::new(context);
    state.report_items.extend(localized.diagnostics);
    state.map_formula(formula, &canvas_config)?;

    let mut tracks = state.tracks;
    tracks.sort_by_key(|track| track.z_order);
    let plan = DraftImportPlan {
        schema_version: DraftImportPlanSchemaVersion::current(),
        import_id: options.import_id.clone(),
        draft_id: format!("draft-{}", safe_stem(&options.import_id, "offline-import")).into(),
        draft_name: format!(
            "导入模板 {}",
            safe_stem(&options.import_id, "offline-import")
        ),
        canvas_config,
        materials: state.materials,
        tracks,
    };
    validate_import_plan(&plan)?;

    let generated_at = options
        .generated_at
        .clone()
        .or_else(|| bundle.provenance.captured_at.clone())
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_owned());
    let report = AdaptationReport::new("kaipaiOfflineBundle", generated_at, state.report_items);

    Ok(KaipaiMappedFixture {
        plan,
        report,
        localized_resources: localized.manifest,
    })
}

struct MapperContext<'a> {
    bundle: &'a KaipaiFormulaBundle,
    manifest: &'a LocalizedResourceManifest,
    direct_materials: BTreeMap<&'a str, &'a DirectMaterialRef>,
    resources: BTreeMap<&'a str, &'a FormulaResourceRef>,
}

impl<'a> MapperContext<'a> {
    fn new(bundle: &'a KaipaiFormulaBundle, manifest: &'a LocalizedResourceManifest) -> Self {
        Self {
            bundle,
            manifest,
            direct_materials: bundle
                .direct_materials
                .iter()
                .map(|material| (material.material_id.as_str(), material))
                .collect(),
            resources: bundle
                .resources
                .iter()
                .map(|resource| (resource.resource_id.as_str(), resource))
                .collect(),
        }
    }

    fn resource(&self, resource_id: &str) -> Option<&'a FormulaResourceRef> {
        self.resources.get(resource_id).copied()
    }

    fn direct_material(&self, material_id: &str) -> Option<&'a DirectMaterialRef> {
        self.direct_materials.get(material_id).copied()
    }

    fn localized(&self, resource_id: &str) -> Option<&'a LocalizedResource> {
        self.manifest
            .resources
            .iter()
            .find(|resource| resource.stable_id == resource_id)
    }

    fn available_resource_ref(&self, resource_id: &str) -> Option<&'a str> {
        let localized = self.localized(resource_id)?;
        if localized.status == LocalizedResourceStatus::Available {
            localized.project_relative_ref.as_deref()
        } else {
            None
        }
    }

    fn external_id(&self) -> String {
        self.bundle
            .provenance
            .template_id
            .trim_end_matches("-template")
            .to_owned()
    }
}

struct MapperState<'a> {
    context: MapperContext<'a>,
    materials: Vec<ImportMaterialPlan>,
    tracks: Vec<ImportTrackPlan>,
    report_items: Vec<AdaptationReportItem>,
    material_ids: BTreeSet<String>,
}

impl<'a> MapperState<'a> {
    fn new(context: MapperContext<'a>) -> Self {
        Self {
            context,
            materials: Vec::new(),
            tracks: Vec::new(),
            report_items: Vec::new(),
            material_ids: BTreeSet::new(),
        }
    }

    fn map_formula(
        &mut self,
        formula: &Map<String, Value>,
        canvas_config: &DraftCanvasConfig,
    ) -> Result<(), AdapterKaipaiError> {
        if !array_field(formula, "videoClipList").is_empty() {
            self.map_video_clip_list(formula, canvas_config, true)?;
            self.report_canvas_support();
            self.report_main_video_support(formula);
        }
        if !array_field(formula, "pipList").is_empty() {
            self.map_pip_list(formula, canvas_config)?;
            self.report_pip_support();
        }
        if !array_field(formula, "stickerList").is_empty() {
            self.map_sticker_list(formula, canvas_config)?;
            self.report_text_sticker_support_if_present(formula);
        }
        if formula.contains_key("bgm") {
            self.map_bgm(formula)?;
            self.report_bgm_support();
        }
        if formula.contains_key("nativeEffectList") {
            self.report_native_effects(formula)?;
        }
        Ok(())
    }

    fn report_canvas_support(&mut self) {
        self.report_items.push(report_item(
            AdaptationStatus::Supported,
            AdaptationSeverity::Info,
            AdaptationCategory::Canvas,
            AdaptationTargetKind::Canvas,
            "canvas-main",
            "Canvas, frame rate, and background color map to DraftCanvasConfig.",
            None,
            &self.context.external_id(),
            "formula.videoCanvasConfig",
        ));
    }

    fn report_main_video_support(&mut self, formula: &Map<String, Value>) {
        let details = main_video_timerange_report_details(formula);
        self.report_items.push(report_item(
            AdaptationStatus::Supported,
            AdaptationSeverity::Info,
            AdaptationCategory::Segment,
            AdaptationTargetKind::Segment,
            "segment-main-video",
            "Main video source and speed-adjusted target timeranges map to canonical integer microseconds.",
            details.as_deref(),
            &self.context.external_id(),
            "formula.videoClipList[0]",
        ));
    }

    fn report_pip_support(&mut self) {
        self.report_items.push(report_item(
            AdaptationStatus::Supported,
            AdaptationSeverity::Info,
            AdaptationCategory::Track,
            AdaptationTargetKind::Track,
            "track-pip-overlay",
            "PIP level maps to provider-neutral overlay z-order.",
            Some(
                "Kaipai level is catalog evidence only; canonical Draft track ordering carries layer order.",
            ),
            &self.context.external_id(),
            "formula.pipList[0].level",
        ));
        self.report_items.push(report_item(
            AdaptationStatus::Supported,
            AdaptationSeverity::Info,
            AdaptationCategory::Segment,
            AdaptationTargetKind::Segment,
            "segment-pip-overlay",
            "PIP bounds, fit, opacity, position, scale, and static center-anchor rotation map to SegmentVisual.",
            Some("Static center-anchor rotation is supported generically by Plan 17-07 export parity."),
            &self.context.external_id(),
            "formula.pipList[0]",
        ));
    }

    fn report_text_sticker_support_if_present(&mut self, formula: &Map<String, Value>) {
        let Some(first_text_info) =
            array_field(formula, "stickerList")
                .iter()
                .find_map(|sticker| {
                    array_field_from_value(sticker, "textEditInfoList")
                        .first()
                        .copied()
                })
        else {
            return;
        };
        self.report_items.push(report_item(
            AdaptationStatus::Supported,
            AdaptationSeverity::Info,
            AdaptationCategory::Text,
            AdaptationTargetKind::Text,
            "segment-text-sticker",
            "Text sticker content, color, stroke, shadow, layout, and wrapping map to TextSegment.",
            None,
            &self.context.external_id(),
            "formula.stickerList[0].textEditInfoList[0]",
        ));
        if optional_string_field(&first_text_info, "fontPath").is_some() {
            self.report_items.push(report_item(
                AdaptationStatus::Approximated,
                AdaptationSeverity::Warning,
                AdaptationCategory::Font,
                AdaptationTargetKind::Font,
                draft_model::BUNDLED_TEXT_FONT_REF,
                "Requested provider font is approximated with bundled Noto Sans CJK SC fallback.",
                Some("Font closure keeps a local fontRef and records localization fallback in the report."),
                &self.context.external_id(),
                "formula.stickerList[0].textEditInfoList[0].fontPath",
            ));
        }
        if let Some(effect_name) = optional_string_field(&first_text_info, "textEffect") {
            let message = format!(
                "Provider text {} effect is dropped until a local text effect exists.",
                safe_stem(effect_name, "provider")
            );
            self.report_items.push(report_item(
                AdaptationStatus::Dropped,
                AdaptationSeverity::Warning,
                AdaptationCategory::Text,
                AdaptationTargetKind::Effect,
                &format!(
                    "text-effect-{}",
                    safe_stem(effect_name, "provider-text-effect")
                ),
                &message,
                Some(
                    "Unsupported text effects must not be smuggled into generic filter parameters.",
                ),
                &self.context.external_id(),
                "formula.stickerList[0].textEditInfoList[0].textEffect",
            ));
        }
    }

    fn report_bgm_support(&mut self) {
        self.report_items.push(report_item(
            AdaptationStatus::Supported,
            AdaptationSeverity::Info,
            AdaptationCategory::Audio,
            AdaptationTargetKind::Audio,
            "segment-bgm-audio",
            "BGM material, gain, fade-in, and fade-out map to canonical SegmentAudio.",
            None,
            &self.context.external_id(),
            "formula.bgm",
        ));
    }

    fn ensure_material(
        &mut self,
        material_id: &str,
        kind: MaterialKind,
        uri: String,
        display_name: String,
        duration: Option<Microseconds>,
        canvas_config: &DraftCanvasConfig,
    ) {
        if !self.material_ids.insert(material_id.to_owned()) {
            return;
        }
        let has_video = matches!(
            kind,
            MaterialKind::Video | MaterialKind::Image | MaterialKind::Sticker
        );
        let has_audio = matches!(kind, MaterialKind::Audio);
        let width = if has_video {
            self.context
                .bundle
                .source_media
                .width
                .filter(|_| self.context.bundle.source_media.resource_id == material_id)
                .or(Some(canvas_config.width))
        } else {
            None
        };
        let height = if has_video {
            self.context
                .bundle
                .source_media
                .height
                .filter(|_| self.context.bundle.source_media.resource_id == material_id)
                .or(Some(canvas_config.height))
        } else {
            None
        };
        self.materials.push(ImportMaterialPlan {
            material: Material {
                material_id: material_id.to_owned().into(),
                kind,
                uri,
                display_name,
                metadata: MaterialMetadata {
                    duration,
                    width,
                    height,
                    frame_rate: if has_video {
                        Some(canvas_config.frame_rate.clone())
                    } else {
                        None
                    },
                    has_video,
                    has_audio,
                    audio_sample_rate: if has_audio { Some(48_000) } else { None },
                    audio_channels: if has_audio { Some(2) } else { None },
                    probe_error: None,
                },
                status: MaterialStatus::Available,
            },
        });
    }

    fn ensure_resource_material(
        &mut self,
        resource_id: &str,
        duration: Option<Microseconds>,
        canvas_config: &DraftCanvasConfig,
    ) -> Option<()> {
        let uri = self.context.available_resource_ref(resource_id)?.to_owned();
        let kind = self
            .context
            .direct_material(resource_id)
            .map(|direct| direct.kind)
            .or_else(|| {
                self.context
                    .resource(resource_id)
                    .map(|resource| resource.kind)
            })
            .map(material_kind)?;
        let display_name = self
            .context
            .direct_material(resource_id)
            .map(|direct| direct.display_name.clone())
            .or_else(|| {
                self.context
                    .resource(resource_id)
                    .map(|resource| resource.display_name.clone())
            })
            .unwrap_or_else(|| resource_id.to_owned());
        self.ensure_material(
            resource_id,
            kind,
            uri,
            display_name,
            duration,
            canvas_config,
        );
        Some(())
    }

    fn map_video_clip_list(
        &mut self,
        formula: &Map<String, Value>,
        canvas_config: &DraftCanvasConfig,
        main_family: bool,
    ) -> Result<(), AdapterKaipaiError> {
        for (index, clip) in array_field(formula, "videoClipList").iter().enumerate() {
            let path = format!("formula.videoClipList[{index}]");
            let resource_id = string_field(clip, &path, "resourceId")?;
            let segment_id = string_field(clip, &path, "segmentId")?;
            let track_id = string_field(clip, &path, "trackId")?;
            let source_start = u64_field(clip, &path, "startAtMs")?;
            let source_end = u64_field(clip, &path, "endAtMs")?;
            let source_duration = source_end.checked_sub(source_start).ok_or_else(|| {
                mapper_error(&format!("{path}.endAtMs"), "clip end must be after start")
            })?;
            let target_duration =
                optional_u64_field(clip, "durationMsWithSpeed").unwrap_or(source_duration);
            let retiming = retiming_from_durations(source_duration, target_duration, &path)?;
            self.ensure_resource_material(resource_id, Some(ms_to_us(source_end)), canvas_config);
            if !self.material_ids.contains(resource_id) {
                self.report_missing_resource(
                    resource_id,
                    AdaptationCategory::Segment,
                    AdaptationTargetKind::Segment,
                    segment_id,
                    &format!("{path}.resourceId"),
                );
                continue;
            }

            let mut segment = Segment::new(
                segment_id.to_owned(),
                resource_id.to_owned(),
                SourceTimerange::new(ms_to_us(source_start), ms_to_us(source_duration)),
                TargetTimerange::new(
                    ms_to_us(optional_u64_field(clip, "targetStartMs").unwrap_or(0)),
                    ms_to_us(target_duration),
                ),
            );
            segment.main_track_magnet = if main_family {
                MainTrackMagnet::enabled()
            } else {
                MainTrackMagnet::disabled()
            };
            segment.retiming = retiming;
            segment.visual = visual_from_formula(clip, false)?;
            segment.filters = self.map_clip_filter_list(clip, &path, segment_id)?;
            segment.transition = transition_from_formula(clip)?;
            segment.keyframes = keyframes_from_formula(clip)?;

            let mut track = Track::new(track_id.to_owned(), TrackKind::Video, "主视频");
            track.segments.push(segment);
            self.tracks.push(ImportTrackPlan { z_order: 0, track });
        }
        Ok(())
    }

    fn map_pip_list(
        &mut self,
        formula: &Map<String, Value>,
        canvas_config: &DraftCanvasConfig,
    ) -> Result<(), AdapterKaipaiError> {
        for (index, pip) in array_field(formula, "pipList").iter().enumerate() {
            let path = format!("formula.pipList[{index}]");
            let resource_id = string_field(pip, &path, "resourceId")?;
            let segment_id = string_field(pip, &path, "segmentId")?;
            let track_id = string_field(pip, &path, "trackId")?;
            let duration = u64_field(pip, &path, "duration")?;
            self.ensure_resource_material(resource_id, Some(ms_to_us(duration)), canvas_config);
            if !self.material_ids.contains(resource_id) {
                self.report_missing_resource(
                    resource_id,
                    AdaptationCategory::Sticker,
                    AdaptationTargetKind::Sticker,
                    segment_id,
                    &format!("{path}.resourceId"),
                );
                continue;
            }
            let video_clip = pip.get("videoClip").unwrap_or(&Value::Null);
            let source_start = optional_u64_field(video_clip, "startAtMs").unwrap_or(0);
            let source_end = optional_u64_field(video_clip, "endAtMs").unwrap_or(duration);
            let source_duration = source_end.saturating_sub(source_start).max(duration);

            let mut segment = Segment::new(
                segment_id.to_owned(),
                resource_id.to_owned(),
                SourceTimerange::new(ms_to_us(source_start), ms_to_us(source_duration)),
                TargetTimerange::new(
                    ms_to_us(u64_field(pip, &path, "start")?),
                    ms_to_us(duration),
                ),
            );
            segment.retiming = retiming_from_durations(source_duration, duration, &path)?;
            segment.visual = visual_from_formula(pip, true)?;
            segment.filters = self.map_clip_filter_list(pip, &path, segment_id)?;

            let mut track = Track::new(track_id.to_owned(), TrackKind::Video, "覆盖视频");
            track.segments.push(segment);
            self.tracks.push(ImportTrackPlan {
                z_order: i32_field(pip, &path, "level").unwrap_or(10),
                track,
            });
        }
        Ok(())
    }

    fn map_sticker_list(
        &mut self,
        formula: &Map<String, Value>,
        canvas_config: &DraftCanvasConfig,
    ) -> Result<(), AdapterKaipaiError> {
        for (index, sticker) in array_field(formula, "stickerList").iter().enumerate() {
            let path = format!("formula.stickerList[{index}]");
            let resource_id = string_field(sticker, &path, "resourceId")?;
            let segment_id = string_field(sticker, &path, "segmentId")?;
            let track_id = string_field(sticker, &path, "trackId")?;
            let duration = u64_field(sticker, &path, "duration")?;
            let text_infos = array_field_from_value(sticker, "textEditInfoList");
            if let Some(text_info) = text_infos.first() {
                self.ensure_material(
                    resource_id,
                    MaterialKind::Text,
                    format!("text://{resource_id}"),
                    string_field(text_info, &path, "text")?.to_owned(),
                    Some(ms_to_us(duration)),
                    canvas_config,
                );
                let mut segment = Segment::new(
                    segment_id.to_owned(),
                    resource_id.to_owned(),
                    SourceTimerange::new(0, ms_to_us(duration)),
                    TargetTimerange::new(
                        ms_to_us(u64_field(sticker, &path, "start")?),
                        ms_to_us(duration),
                    ),
                );
                segment.text = Some(text_segment_from_formula(sticker, text_info)?);
                segment.visual = visual_from_formula(sticker, true)?;

                let mut track = Track::new(track_id.to_owned(), TrackKind::Text, "文字");
                track.segments.push(segment);
                self.tracks.push(ImportTrackPlan {
                    z_order: i32_field(sticker, &path, "level").unwrap_or(30),
                    track,
                });
                continue;
            }

            self.ensure_resource_material(resource_id, Some(ms_to_us(duration)), canvas_config);
            if !self.material_ids.contains(resource_id) {
                self.report_items.push(report_item(
                    AdaptationStatus::MissingResource,
                    AdaptationSeverity::Error,
                    AdaptationCategory::Resource,
                    AdaptationTargetKind::Resource,
                    resource_id,
                    "Referenced sticker resource is absent from the sanitized offline bundle.",
                    Some("Mapper must report the missing resource and skip the dependent segment."),
                    &self.context.external_id(),
                    &format!("{path}.resourceId"),
                ));
                self.report_items.push(report_item(
                    AdaptationStatus::Dropped,
                    AdaptationSeverity::Warning,
                    AdaptationCategory::Sticker,
                    AdaptationTargetKind::Sticker,
                    segment_id,
                    "Sticker segment is dropped because its material cannot be localized.",
                    None,
                    &self.context.external_id(),
                    &path,
                ));
                continue;
            }

            let mut segment = Segment::new(
                segment_id.to_owned(),
                resource_id.to_owned(),
                SourceTimerange::new(0, ms_to_us(duration)),
                TargetTimerange::new(
                    ms_to_us(u64_field(sticker, &path, "start")?),
                    ms_to_us(duration),
                ),
            );
            segment.visual = visual_from_formula(sticker, true)?;
            segment.filters = self.map_clip_filter_list(sticker, &path, segment_id)?;

            let mut track = Track::new(track_id.to_owned(), TrackKind::Sticker, "贴纸");
            track.segments.push(segment);
            self.tracks.push(ImportTrackPlan {
                z_order: i32_field(sticker, &path, "level").unwrap_or(30),
                track,
            });
        }
        Ok(())
    }

    fn map_clip_filter_list(
        &mut self,
        clip: &Value,
        path: &str,
        segment_id: &str,
    ) -> Result<Vec<Filter>, AdapterKaipaiError> {
        let mut filters = Vec::new();
        for (index, filter) in array_field_from_value(clip, "filterList")
            .iter()
            .enumerate()
        {
            let filter_path = format!("{path}.filterList[{index}]");
            let filter_type = optional_string_field(filter, "type")
                .or_else(|| optional_string_field(filter, "kind"))
                .or_else(|| optional_string_field(filter, "name"))
                .unwrap_or("");
            match filter_type {
                "gaussianBlur" | "blur" => {
                    let radius_millis = optional_u32_field(filter, "radiusMillis").unwrap_or(1_000);
                    filters.push(Filter::gaussian_blur(radius_millis));
                    self.report_supported_filter(
                        segment_id,
                        "Gaussian blur provider concept maps to first-party FilterKind::GaussianBlur.",
                        &filter_path,
                    );
                }
                "basicColorAdjustment" | "colorAdjustment" | "color" => {
                    let brightness_millis =
                        optional_i32_field(filter, "brightnessMillis").unwrap_or(0);
                    let contrast_millis =
                        optional_u32_field(filter, "contrastMillis").unwrap_or(1_000);
                    let saturation_millis =
                        optional_u32_field(filter, "saturationMillis").unwrap_or(1_000);
                    filters.push(Filter::basic_color_adjustment(
                        brightness_millis,
                        contrast_millis,
                        saturation_millis,
                    ));
                    self.report_supported_filter(
                        segment_id,
                        "Basic color provider concept maps to first-party FilterKind::BasicColorAdjustment.",
                        &filter_path,
                    );
                }
                "opacityAdjustment" | "opacity" => {
                    let opacity_millis = optional_u32_field(filter, "opacityMillis")
                        .or_else(|| optional_f64_field(filter, "opacity").map(scale_to_millis))
                        .unwrap_or(1_000);
                    filters.push(Filter::opacity_adjustment(opacity_millis));
                    self.report_supported_filter(
                        segment_id,
                        "Opacity provider concept maps to first-party FilterKind::OpacityAdjustment.",
                        &filter_path,
                    );
                }
                _ => {
                    let effect_id = optional_string_field(filter, "effectId")
                        .or_else(|| optional_string_field(filter, "filterId"))
                        .or_else(|| optional_string_field(filter, "nativeEffectName"))
                        .unwrap_or(filter_type)
                        .trim();
                    let target_id = if effect_id.is_empty() {
                        format!("provider-filter-{segment_id}-{index}")
                    } else {
                        safe_stem(effect_id, &format!("provider-filter-{index}"))
                    };
                    self.report_items.push(report_item(
                        AdaptationStatus::Dropped,
                        AdaptationSeverity::Warning,
                        AdaptationCategory::NativeEffect,
                        AdaptationTargetKind::Filter,
                        &target_id,
                        "Provider-native effect is report-only and omitted from canonical draft filters.",
                        Some(
                            "Only first-party Phase 19 filter concepts may become render semantics.",
                        ),
                        &self.context.external_id(),
                        &filter_path,
                    ));
                }
            }
        }
        Ok(filters)
    }

    fn report_supported_filter(&mut self, segment_id: &str, message: &str, external_path: &str) {
        self.report_items.push(report_item(
            AdaptationStatus::Supported,
            AdaptationSeverity::Info,
            AdaptationCategory::Segment,
            AdaptationTargetKind::Filter,
            &format!("filter-{segment_id}"),
            message,
            Some("Filter parameters are normalized into integer first-party effect semantics."),
            &self.context.external_id(),
            external_path,
        ));
    }

    fn map_bgm(&mut self, formula: &Map<String, Value>) -> Result<(), AdapterKaipaiError> {
        let Some(bgm) = formula.get("bgm") else {
            return Ok(());
        };
        let path = "formula.bgm";
        let resource_id = string_field(bgm, path, "resourceId")?;
        let segment_id = string_field(bgm, path, "segmentId")?;
        let track_id = string_field(bgm, path, "trackId")?;
        let duration = u64_field(bgm, path, "durationMs")?;
        let canvas = DraftCanvasConfig::mvp_default();
        self.ensure_resource_material(resource_id, Some(ms_to_us(duration)), &canvas);
        if !self.material_ids.contains(resource_id) {
            self.report_missing_resource(
                resource_id,
                AdaptationCategory::Audio,
                AdaptationTargetKind::Audio,
                segment_id,
                "formula.bgm.resourceId",
            );
            return Ok(());
        }

        let mut segment = Segment::new(
            segment_id.to_owned(),
            resource_id.to_owned(),
            SourceTimerange::new(0, ms_to_us(duration)),
            TargetTimerange::new(
                ms_to_us(optional_u64_field(bgm, "startAtMs").unwrap_or(0)),
                ms_to_us(duration),
            ),
        );
        segment.audio = SegmentAudio {
            gain_millis: optional_u32_field(bgm, "volumeMillis").unwrap_or(1_000),
            fade_in_duration: AudioFade {
                duration: ms_to_us(optional_u64_field(bgm, "fadeInMs").unwrap_or(0)),
            },
            fade_out_duration: AudioFade {
                duration: ms_to_us(optional_u64_field(bgm, "fadeOutMs").unwrap_or(0)),
            },
            ..SegmentAudio::default()
        };

        let mut track = Track::new(track_id.to_owned(), TrackKind::Audio, "音频");
        track.segments.push(segment);
        self.tracks.push(ImportTrackPlan {
            z_order: 100,
            track,
        });
        Ok(())
    }

    fn report_native_effects(
        &mut self,
        formula: &Map<String, Value>,
    ) -> Result<(), AdapterKaipaiError> {
        for (index, effect) in array_field(formula, "nativeEffectList").iter().enumerate() {
            let path = format!("formula.nativeEffectList[{index}]");
            let effect_id = string_field(effect, &path, "effectId")?;
            let effect_name = string_field(effect, &path, "nativeEffectName")?;
            self.report_items.push(report_item(
                AdaptationStatus::NeedsNativeEffect,
                AdaptationSeverity::Warning,
                AdaptationCategory::NativeEffect,
                AdaptationTargetKind::Effect,
                effect_id,
                "Provider-native beauty effect requires a local implementation before it can be represented.",
                Some("Native effects are never classified as supported by fixture expectations."),
                &self.context.external_id(),
                &path,
            ));
            self.report_items.push(report_item(
                AdaptationStatus::Dropped,
                AdaptationSeverity::Warning,
                AdaptationCategory::Segment,
                AdaptationTargetKind::Filter,
                &format!("filter-{effect_id}"),
                "Native effect is omitted from the canonical draft filter stack.",
                Some("Report evidence preserves the external reference without writing native parameters to Draft."),
                &self.context.external_id(),
                &format!("{path}.nativeEffectName"),
            ));
            let _ = effect_name;
        }
        Ok(())
    }

    fn report_missing_resource(
        &mut self,
        resource_id: &str,
        category: AdaptationCategory,
        target_kind: AdaptationTargetKind,
        target_id: &str,
        external_path: &str,
    ) {
        self.report_items.push(report_item(
            AdaptationStatus::MissingResource,
            AdaptationSeverity::Error,
            AdaptationCategory::Resource,
            AdaptationTargetKind::Resource,
            resource_id,
            "Referenced resource is absent from the sanitized offline bundle.",
            None,
            &self.context.external_id(),
            external_path,
        ));
        self.report_items.push(report_item(
            AdaptationStatus::Dropped,
            AdaptationSeverity::Warning,
            category,
            target_kind,
            target_id,
            "Dependent segment is dropped because its material cannot be localized.",
            None,
            &self.context.external_id(),
            external_path,
        ));
    }
}

fn template_resource_refs(
    bundle: &KaipaiFormulaBundle,
    verify_sha256: bool,
) -> Vec<TemplateResourceRef> {
    bundle
        .resources
        .iter()
        .map(|resource| TemplateResourceRef {
            stable_id: resource.resource_id.clone(),
            kind: template_resource_kind(resource.kind),
            source_uri: resource.uri.clone(),
            sha256: verify_sha256.then(|| resource.sha256.clone()).flatten(),
            display_name: Some(resource.display_name.clone()),
        })
        .collect()
}

fn main_video_timerange_report_details(formula: &Map<String, Value>) -> Option<String> {
    let clip = array_field(formula, "videoClipList").first().copied()?;
    let source_start = optional_u64_field(clip, "startAtMs")?;
    let source_end = optional_u64_field(clip, "endAtMs")?;
    let source_duration = source_end.checked_sub(source_start)?;
    let target_duration =
        optional_u64_field(clip, "durationMsWithSpeed").unwrap_or(source_duration);
    Some(format!(
        "durationMsWithSpeed={} maps to a {} target range while preserving the {} source range.",
        target_duration,
        format_report_duration_ms(target_duration),
        format_report_duration_ms(source_duration)
    ))
}

fn format_report_duration_ms(duration_ms: u64) -> String {
    if duration_ms % 1_000 == 0 {
        format!("{}s", duration_ms / 1_000)
    } else {
        format!("{duration_ms}ms")
    }
}

fn template_resource_kind(kind: FormulaResourceKind) -> TemplateResourceKind {
    match kind {
        FormulaResourceKind::Video => TemplateResourceKind::Video,
        FormulaResourceKind::Image => TemplateResourceKind::Image,
        FormulaResourceKind::Audio => TemplateResourceKind::Audio,
        FormulaResourceKind::Font => TemplateResourceKind::Font,
        FormulaResourceKind::Sticker => TemplateResourceKind::Sticker,
    }
}

fn material_kind(kind: FormulaResourceKind) -> MaterialKind {
    match kind {
        FormulaResourceKind::Video => MaterialKind::Video,
        FormulaResourceKind::Image => MaterialKind::Image,
        FormulaResourceKind::Audio => MaterialKind::Audio,
        FormulaResourceKind::Font => MaterialKind::Text,
        FormulaResourceKind::Sticker => MaterialKind::Sticker,
    }
}

fn map_canvas_config(
    formula: &Map<String, Value>,
) -> Result<DraftCanvasConfig, AdapterKaipaiError> {
    let canvas = formula
        .get("videoCanvasConfig")
        .ok_or_else(|| mapper_error("formula.videoCanvasConfig", "canvas config is required"))?;
    let width = u32_field(canvas, "formula.videoCanvasConfig", "width")?;
    let height = u32_field(canvas, "formula.videoCanvasConfig", "height")?;
    let frame_rate = u32_field(canvas, "formula.videoCanvasConfig", "frameRate")?;
    let background = optional_string_field(canvas, "backgroundColor")
        .filter(|color| !color.trim().is_empty())
        .map(|color| CanvasBackground::SolidColor {
            color: color.to_owned(),
        })
        .unwrap_or(CanvasBackground::Black);

    Ok(DraftCanvasConfig {
        aspect_ratio: aspect_ratio(width, height),
        width,
        height,
        frame_rate: RationalFrameRate::new(frame_rate, 1),
        background,
        adaptation_policy: CanvasAdaptationPolicy::Manual,
    })
}

fn aspect_ratio(width: u32, height: u32) -> CanvasAspectRatio {
    match (width, height) {
        (1080, 1920) => CanvasAspectRatio::preset(CanvasAspectRatioPreset::Ratio9x16),
        (1920, 1080) => CanvasAspectRatio::preset(CanvasAspectRatioPreset::Ratio16x9),
        (width, height) if width == height => {
            CanvasAspectRatio::preset(CanvasAspectRatioPreset::Ratio1x1)
        }
        _ => CanvasAspectRatio::custom(width, height),
    }
}

fn visual_from_formula(
    value: &Value,
    relative_position: bool,
) -> Result<SegmentVisual, AdapterKaipaiError> {
    let path = "formula.visual";
    let mut transform = SegmentTransform::default();
    if relative_position {
        if let (Some(x), Some(y)) = (
            optional_f64_field(value, "relativeCenterX"),
            optional_f64_field(value, "relativeCenterY"),
        ) {
            transform.position = SegmentPosition {
                x: ((x - 0.5) * 2_000.0).round() as i32,
                y: ((0.5 - y) * 2_000.0).round() as i32,
            };
        }
    } else if let Some(position) = value.get("position") {
        transform.position = SegmentPosition {
            x: i32_field(position, path, "x").unwrap_or(0),
            y: i32_field(position, path, "y").unwrap_or(0),
        };
    }
    if let Some(scale) = optional_f64_field(value, "scale") {
        let scale = scale_to_millis(scale);
        transform.scale = SegmentScale {
            x_millis: scale,
            y_millis: scale,
        };
    }
    transform.rotation = SegmentRotation {
        degrees: i32_field(value, path, "rotate").unwrap_or(0),
    };
    transform.opacity = SegmentOpacity {
        value_millis: scale_to_millis(optional_f64_field(value, "alpha").unwrap_or(1.0)),
    };
    transform.anchor = SegmentAnchor::center();
    if let Some(crop) = value.get("crop") {
        transform.crop = SegmentCrop {
            left_millis: optional_u32_field(crop, "left").unwrap_or(0),
            right_millis: optional_u32_field(crop, "right").unwrap_or(0),
            top_millis: optional_u32_field(crop, "top").unwrap_or(0),
            bottom_millis: optional_u32_field(crop, "bottom").unwrap_or(0),
        };
    }

    Ok(SegmentVisual {
        transform,
        fit_mode: fit_mode(optional_string_field(value, "fitMode").unwrap_or("fit")),
        ..SegmentVisual::default()
    })
}

fn retiming_from_durations(
    source_duration_ms: u64,
    target_duration_ms: u64,
    path: &str,
) -> Result<SegmentRetiming, AdapterKaipaiError> {
    if source_duration_ms == 0 || target_duration_ms == 0 {
        return Err(mapper_error(
            path,
            "source and target durations must be positive for retime mapping",
        ));
    }
    if source_duration_ms == target_duration_ms {
        return Ok(SegmentRetiming::default());
    }
    let (numerator, denominator) =
        normalized_u32_ratio(source_duration_ms, target_duration_ms, path)?;
    Ok(SegmentRetiming {
        mode: RetimeMode::Constant {
            speed: SpeedRatio::new(numerator, denominator),
        },
        ..SegmentRetiming::default()
    })
}

fn normalized_u32_ratio(
    numerator: u64,
    denominator: u64,
    path: &str,
) -> Result<(u32, u32), AdapterKaipaiError> {
    let divisor = gcd(numerator, denominator);
    let numerator = numerator / divisor;
    let denominator = denominator / divisor;
    let numerator = u32::try_from(numerator)
        .map_err(|_| mapper_error(path, "retime speed numerator exceeds u32 range"))?;
    let denominator = u32::try_from(denominator)
        .map_err(|_| mapper_error(path, "retime speed denominator exceeds u32 range"))?;
    Ok((numerator, denominator))
}

fn gcd(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

fn text_segment_from_formula(
    sticker: &Value,
    text_info: &Value,
) -> Result<TextSegment, AdapterKaipaiError> {
    let text = string_field(
        text_info,
        "formula.stickerList[].textEditInfoList[]",
        "text",
    )?;
    let width = sticker
        .get("bounds")
        .and_then(|bounds| optional_u32_field(bounds, "widthMillis"))
        .unwrap_or(700);
    let height = sticker
        .get("bounds")
        .and_then(|bounds| optional_u32_field(bounds, "heightMillis"))
        .unwrap_or(180);
    let center_x = optional_f64_field(sticker, "relativeCenterX").unwrap_or(0.5);
    let center_y = optional_f64_field(sticker, "relativeCenterY").unwrap_or(0.5);
    let x =
        ((center_x * 1_000.0).round() as i32 - i32::try_from(width / 2).unwrap_or(0)).max(0) as u32;
    let y = ((center_y * 1_000.0).round() as i32 - i32::try_from(height / 2).unwrap_or(0)).max(0)
        as u32;

    Ok(TextSegment {
        content: text.to_owned(),
        source: Default::default(),
        style: TextStyle {
            font: TextFont::bundled_default(),
            font_size: optional_u32_field(text_info, "fontSize").unwrap_or(36),
            color: optional_string_field(text_info, "textColor")
                .unwrap_or("#FFFFFF")
                .to_owned(),
            alignment: text_alignment(
                optional_string_field(text_info, "alignment").unwrap_or("center"),
            ),
            stroke: optional_string_field(text_info, "textStrokeColor").map(|color| {
                draft_model::TextStroke {
                    color: color.to_owned(),
                    width: optional_u32_field(text_info, "textStrokeWidth").unwrap_or(0),
                }
            }),
            shadow: matches!(text_info.get("showShadow"), Some(Value::Bool(true))).then(|| {
                draft_model::TextShadow {
                    color: optional_string_field(text_info, "shadowColor")
                        .unwrap_or("#000000")
                        .to_owned(),
                    offset_x: 0,
                    offset_y: 0,
                    blur: optional_u32_field(text_info, "shadowBlur").unwrap_or(0),
                }
            }),
            ..TextStyle::default()
        },
        text_box: TextBox {
            width_millis: width,
            height_millis: height,
        },
        layout_region: TextLayoutRegion {
            x_millis: x,
            y_millis: y,
            width_millis: width,
            height_millis: height,
        },
        wrapping: TextWrapping::Auto,
        bubble: None,
        effect: None,
    })
}

fn transition_from_formula(value: &Value) -> Result<Option<Transition>, AdapterKaipaiError> {
    let Some(transition) = value.get("transition") else {
        return Ok(None);
    };
    let name = optional_string_field(transition, "name")
        .or_else(|| optional_string_field(transition, "type"))
        .unwrap_or("");
    if !matches!(name, "fade" | "dissolve") {
        return Ok(None);
    }
    Ok(Some(Transition::dissolve(ms_to_us(
        optional_u64_field(transition, "durationMs").unwrap_or(300),
    ))))
}

fn keyframes_from_formula(value: &Value) -> Result<Vec<Keyframe>, AdapterKaipaiError> {
    let mut keyframes = Vec::new();
    for (index, keyframe) in array_field_from_value(value, "keyframes")
        .iter()
        .enumerate()
    {
        let path = format!("formula.keyframes[{index}]");
        let property = match string_field(keyframe, &path, "property")? {
            "positionX" => KeyframeProperty::VisualPositionX,
            "positionY" => KeyframeProperty::VisualPositionY,
            "scaleX" => KeyframeProperty::VisualScaleX,
            "scaleY" => KeyframeProperty::VisualScaleY,
            "opacity" => KeyframeProperty::VisualOpacity,
            _ => continue,
        };
        let value = if matches!(
            property,
            KeyframeProperty::VisualOpacity
                | KeyframeProperty::VisualScaleX
                | KeyframeProperty::VisualScaleY
        ) {
            KeyframeValue::Uint {
                value: optional_f64_field(keyframe, "value")
                    .map(scale_to_millis)
                    .or_else(|| optional_u32_field(keyframe, "value"))
                    .unwrap_or(0),
            }
        } else {
            KeyframeValue::Int {
                value: i32_field(keyframe, &path, "value").unwrap_or(0),
            }
        };
        keyframes.push(Keyframe {
            at: ms_to_us(u64_field(keyframe, &path, "atMs")?),
            property,
            value,
            interpolation: KeyframeInterpolation::Linear,
            easing: KeyframeEasing::None,
        });
    }
    Ok(keyframes)
}

fn fit_mode(value: &str) -> SegmentFitMode {
    match value {
        "fill" => SegmentFitMode::Fill,
        "stretch" => SegmentFitMode::Stretch,
        _ => SegmentFitMode::Fit,
    }
}

fn text_alignment(value: &str) -> TextAlignment {
    match value {
        "left" => TextAlignment::Left,
        "right" => TextAlignment::Right,
        _ => TextAlignment::Center,
    }
}

fn report_item(
    status: AdaptationStatus,
    severity: AdaptationSeverity,
    category: AdaptationCategory,
    target_kind: AdaptationTargetKind,
    target_id: &str,
    message: &str,
    details: Option<&str>,
    external_id: &str,
    external_path: &str,
) -> AdaptationReportItem {
    AdaptationReportItem {
        status,
        severity,
        category,
        target: Some(AdaptationTargetRef {
            kind: target_kind,
            id: Some(target_id.to_owned()),
        }),
        message: message.to_owned(),
        details: details.map(str::to_owned),
        provenance: vec![ExternalProvenanceRef {
            source_kind: "kaipaiOfflineBundle".to_owned(),
            external_id: Some(external_id.to_owned()),
            external_path: Some(external_path.to_owned()),
            note: Some("adapter evidence only; not canonical render semantics".to_owned()),
        }],
    }
}

fn array_field<'a>(formula: &'a Map<String, Value>, key: &str) -> Vec<&'a Value> {
    formula
        .get(key)
        .and_then(Value::as_array)
        .map(|items| items.iter().collect())
        .unwrap_or_default()
}

fn array_field_from_value<'a>(value: &'a Value, key: &str) -> Vec<&'a Value> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(|items| items.iter().collect())
        .unwrap_or_default()
}

fn string_field<'a>(
    value: &'a Value,
    path: &str,
    field: &str,
) -> Result<&'a str, AdapterKaipaiError> {
    optional_string_field(value, field)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| mapper_error(&format!("{path}.{field}"), "string field is required"))
}

fn optional_string_field<'a>(value: &'a Value, field: &str) -> Option<&'a str> {
    value.get(field).and_then(Value::as_str)
}

fn u64_field(value: &Value, path: &str, field: &str) -> Result<u64, AdapterKaipaiError> {
    optional_u64_field(value, field).ok_or_else(|| {
        mapper_error(
            &format!("{path}.{field}"),
            "unsigned integer field is required",
        )
    })
}

fn u32_field(value: &Value, path: &str, field: &str) -> Result<u32, AdapterKaipaiError> {
    optional_u32_field(value, field)
        .ok_or_else(|| mapper_error(&format!("{path}.{field}"), "u32 field is required"))
}

fn i32_field(value: &Value, path: &str, field: &str) -> Result<i32, AdapterKaipaiError> {
    value
        .get(field)
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
        .ok_or_else(|| mapper_error(&format!("{path}.{field}"), "i32 field is required"))
}

fn optional_u64_field(value: &Value, field: &str) -> Option<u64> {
    value.get(field).and_then(Value::as_u64)
}

fn optional_u32_field(value: &Value, field: &str) -> Option<u32> {
    optional_u64_field(value, field).and_then(|value| u32::try_from(value).ok())
}

fn optional_i32_field(value: &Value, field: &str) -> Option<i32> {
    value
        .get(field)
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
}

fn optional_f64_field(value: &Value, field: &str) -> Option<f64> {
    value.get(field).and_then(Value::as_f64)
}

fn ms_to_us(value: u64) -> Microseconds {
    Microseconds::new(value.saturating_mul(1_000))
}

fn scale_to_millis(value: f64) -> u32 {
    (value * 1_000.0).round().clamp(0.0, f64::from(u32::MAX)) as u32
}

fn safe_stem(value: &str, fallback: &str) -> String {
    let mut stem = String::new();
    for character in value.trim().chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
            stem.push(character.to_ascii_lowercase());
        } else if !stem.ends_with('-') {
            stem.push('-');
        }
    }
    let stem = stem.trim_matches('-');
    if stem.is_empty() {
        fallback.to_owned()
    } else {
        stem.to_owned()
    }
}

fn mapper_error(path: &str, reason: &'static str) -> AdapterKaipaiError {
    AdapterKaipaiError::Mapper {
        path: path.to_owned(),
        reason,
    }
}

#[allow(dead_code)]
fn _source_media_duration(source: &FormulaSourceMedia) -> Option<Microseconds> {
    source.duration_ms.map(ms_to_us)
}
