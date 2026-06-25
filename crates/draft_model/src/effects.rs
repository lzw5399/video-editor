use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{Microseconds, SegmentId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum CapabilitySurface {
    Preview,
    Export,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum CapabilitySupport {
    Supported {
        reason: String,
    },
    Degraded {
        reason: String,
    },
    Unsupported {
        reason: String,
    },
    ExternalReference {
        reference: ExternalEffectReference,
        reason: String,
    },
}

impl CapabilitySupport {
    pub fn supported(reason: impl Into<String>) -> Self {
        Self::Supported {
            reason: reason.into(),
        }
    }

    pub fn degraded(reason: impl Into<String>) -> Self {
        Self::Degraded {
            reason: reason.into(),
        }
    }

    pub fn unsupported(reason: impl Into<String>) -> Self {
        Self::Unsupported {
            reason: reason.into(),
        }
    }

    pub fn external(reference: ExternalEffectReference, reason: impl Into<String>) -> Self {
        Self::ExternalReference {
            reference,
            reason: reason.into(),
        }
    }

    pub fn is_supported(&self) -> bool {
        matches!(self, Self::Supported { .. })
    }

    pub fn reason(&self) -> &str {
        match self {
            Self::Supported { reason }
            | Self::Degraded { reason }
            | Self::Unsupported { reason }
            | Self::ExternalReference { reason, .. } => reason,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapabilityReportItem {
    pub capability_id: String,
    pub display_name: String,
    pub category: CapabilityCategory,
    pub preview: CapabilitySupport,
    pub export: CapabilitySupport,
}

impl CapabilityReportItem {
    pub fn support_for(&self, surface: CapabilitySurface) -> &CapabilitySupport {
        match surface {
            CapabilitySurface::Preview => &self.preview,
            CapabilitySurface::Export => &self.export,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum CapabilityCategory {
    Effect,
    Filter,
    Transition,
    Retime,
    Mask,
    Blend,
    ExternalReference,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct EffectCapabilityRegistry {
    pub entries: Vec<CapabilityReportItem>,
}

pub type ProductionEffectCapabilityRegistry = EffectCapabilityRegistry;

impl EffectCapabilityRegistry {
    pub fn phase19_first_party() -> Self {
        let mut entries = Vec::new();
        entries.push(supported_entry(
            RetimeMode::constant_1x().capability_id(),
            "Constant speed",
            CapabilityCategory::Retime,
            "constant speed retiming keeps source and target clocks aligned",
            "constant speed retiming can be compiled without temporal interpolation",
        ));
        entries.push(CapabilityReportItem {
            capability_id: RetimeMode::SpeedCurve { points: Vec::new() }
                .capability_id()
                .to_owned(),
            display_name: "Speed curve".to_owned(),
            category: CapabilityCategory::Retime,
            preview: CapabilitySupport::degraded(
                "speed curve support is typed but source-time evaluation lands in the retiming plan",
            ),
            export: CapabilitySupport::degraded(
                "speed curve export is typed but compiler filter generation lands in the retiming plan",
            ),
        });
        entries.push(supported_entry(
            TransitionKind::Dissolve.capability_id(),
            "Dissolve transition",
            CapabilityCategory::Transition,
            "dissolve transition is the first typed transition contract",
            "dissolve transition maps to compiler-owned xfade semantics",
        ));
        entries.push(supported_entry(
            FilterKind::GaussianBlur {
                radius_millis: 1_000,
            }
            .capability_id(),
            "Gaussian blur",
            CapabilityCategory::Filter,
            "Gaussian blur is a first-party typed filter contract",
            "Gaussian blur maps to compiler-owned gblur semantics",
        ));
        entries.push(supported_entry(
            FilterKind::BasicColorAdjustment {
                brightness_millis: 0,
                contrast_millis: 1_000,
                saturation_millis: 1_000,
            }
            .capability_id(),
            "Basic color adjustment",
            CapabilityCategory::Filter,
            "basic color adjustment is a first-party typed filter contract",
            "basic color adjustment maps to compiler-owned color filter semantics",
        ));
        entries.push(supported_entry(
            FilterKind::OpacityAdjustment {
                opacity_millis: 1_000,
            }
            .capability_id(),
            "Opacity adjustment",
            CapabilityCategory::Effect,
            "opacity adjustment is a first-party typed effect contract",
            "opacity adjustment maps to compiler-owned alpha semantics",
        ));
        entries.push(supported_entry(
            MaskKind::Rectangle.capability_id(),
            "Rectangle mask",
            CapabilityCategory::Mask,
            "rectangle mask is a first-party typed mask contract",
            "rectangle mask maps to compiler-owned alpha mask semantics",
        ));
        entries.push(supported_entry(
            MaskKind::Ellipse.capability_id(),
            "Ellipse mask",
            CapabilityCategory::Mask,
            "ellipse mask is a first-party typed mask contract",
            "ellipse mask maps to compiler-owned alpha mask semantics",
        ));
        for blend in [
            BlendModeKind::Normal,
            BlendModeKind::Multiply,
            BlendModeKind::Screen,
        ] {
            entries.push(supported_entry(
                blend.capability_id(),
                blend.display_name(),
                CapabilityCategory::Blend,
                "blend mode is a first-party typed compositing contract",
                "blend mode maps to compiler-owned blend semantics",
            ));
        }
        let external = ExternalEffectReference::new("jianying", "private-effect-id");
        entries.push(CapabilityReportItem {
            capability_id: "external:jianying:private-effect-id".to_owned(),
            display_name: "External proprietary reference".to_owned(),
            category: CapabilityCategory::ExternalReference,
            preview: CapabilitySupport::external(
                external.clone(),
                "external provider references are report-only and cannot satisfy preview support",
            ),
            export: CapabilitySupport::external(
                external,
                "external provider references are report-only and cannot satisfy export support",
            ),
        });

        Self { entries }
    }

    pub fn entry(&self, capability_id: &str) -> Option<&CapabilityReportItem> {
        self.entries
            .iter()
            .find(|entry| entry.capability_id == capability_id)
    }
}

fn supported_entry(
    capability_id: impl Into<String>,
    display_name: impl Into<String>,
    category: CapabilityCategory,
    preview_reason: impl Into<String>,
    export_reason: impl Into<String>,
) -> CapabilityReportItem {
    CapabilityReportItem {
        capability_id: capability_id.into(),
        display_name: display_name.into(),
        category,
        preview: CapabilitySupport::supported(preview_reason),
        export: CapabilitySupport::supported(export_reason),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExternalEffectReference {
    pub provider: String,
    pub effect_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional = nullable)]
    pub display_name: Option<String>,
}

impl ExternalEffectReference {
    pub fn new(provider: impl Into<String>, effect_id: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            effect_id: effect_id.into(),
            display_name: None,
        }
    }

    pub fn with_display_name(mut self, display_name: impl Into<String>) -> Self {
        self.display_name = Some(display_name.into());
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum EffectKind {
    GaussianBlur,
    BasicColorAdjustment,
    OpacityAdjustment,
}

impl EffectKind {
    pub fn capability_id(&self) -> &'static str {
        match self {
            Self::GaussianBlur => "effect.gaussianBlur",
            Self::BasicColorAdjustment => "effect.basicColorAdjustment",
            Self::OpacityAdjustment => "effect.opacityAdjustment",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum FilterKind {
    GaussianBlur {
        radius_millis: u32,
    },
    BasicColorAdjustment {
        brightness_millis: i32,
        contrast_millis: u32,
        saturation_millis: u32,
    },
    OpacityAdjustment {
        opacity_millis: u32,
    },
    ExternalReference {
        reference: ExternalEffectReference,
    },
}

impl FilterKind {
    pub fn gaussian_blur(radius_millis: u32) -> Self {
        Self::GaussianBlur { radius_millis }
    }

    pub fn basic_color_adjustment(
        brightness_millis: i32,
        contrast_millis: u32,
        saturation_millis: u32,
    ) -> Self {
        Self::BasicColorAdjustment {
            brightness_millis,
            contrast_millis,
            saturation_millis,
        }
    }

    pub fn opacity_adjustment(opacity_millis: u32) -> Self {
        Self::OpacityAdjustment { opacity_millis }
    }

    pub fn external(provider: impl Into<String>, effect_id: impl Into<String>) -> Self {
        Self::ExternalReference {
            reference: ExternalEffectReference::new(provider, effect_id),
        }
    }

    pub fn display_name(&self) -> String {
        match self {
            Self::GaussianBlur { .. } => "gaussianBlur".to_owned(),
            Self::BasicColorAdjustment { .. } => "basicColorAdjustment".to_owned(),
            Self::OpacityAdjustment { .. } => "opacityAdjustment".to_owned(),
            Self::ExternalReference { reference } => reference
                .display_name
                .clone()
                .unwrap_or_else(|| format!("{}:{}", reference.provider, reference.effect_id)),
        }
    }

    pub fn capability_id(&self) -> String {
        match self {
            Self::GaussianBlur { .. } => EffectKind::GaussianBlur.capability_id().to_owned(),
            Self::BasicColorAdjustment { .. } => {
                EffectKind::BasicColorAdjustment.capability_id().to_owned()
            }
            Self::OpacityAdjustment { .. } => {
                EffectKind::OpacityAdjustment.capability_id().to_owned()
            }
            Self::ExternalReference { reference } => {
                format!("external:{}:{}", reference.provider, reference.effect_id)
            }
        }
    }

    pub fn external_reference(&self) -> Option<&ExternalEffectReference> {
        match self {
            Self::ExternalReference { reference } => Some(reference),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Filter {
    pub kind: FilterKind,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

impl Filter {
    pub fn gaussian_blur(radius_millis: u32) -> Self {
        Self {
            kind: FilterKind::gaussian_blur(radius_millis),
            enabled: true,
        }
    }

    pub fn basic_color_adjustment(
        brightness_millis: i32,
        contrast_millis: u32,
        saturation_millis: u32,
    ) -> Self {
        Self {
            kind: FilterKind::basic_color_adjustment(
                brightness_millis,
                contrast_millis,
                saturation_millis,
            ),
            enabled: true,
        }
    }

    pub fn opacity_adjustment(opacity_millis: u32) -> Self {
        Self {
            kind: FilterKind::opacity_adjustment(opacity_millis),
            enabled: true,
        }
    }

    pub fn external_reference(provider: impl Into<String>, effect_id: impl Into<String>) -> Self {
        Self {
            kind: FilterKind::external(provider, effect_id),
            enabled: true,
        }
    }

    pub fn display_name(&self) -> String {
        self.kind.display_name()
    }

    pub fn capability_id(&self) -> String {
        self.kind.capability_id()
    }

    pub fn external(&self) -> Option<&ExternalEffectReference> {
        self.kind.external_reference()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum TransitionKind {
    Dissolve,
}

impl TransitionKind {
    pub fn capability_id(&self) -> &'static str {
        match self {
            Self::Dissolve => "transition.dissolve",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Dissolve => "dissolve",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TransitionReference {
    FirstParty { transition: TransitionKind },
    ExternalReference { reference: ExternalEffectReference },
}

impl TransitionReference {
    pub fn dissolve() -> Self {
        Self::FirstParty {
            transition: TransitionKind::Dissolve,
        }
    }

    pub fn display_name(&self) -> String {
        match self {
            Self::FirstParty { transition } => transition.display_name().to_owned(),
            Self::ExternalReference { reference } => reference
                .display_name
                .clone()
                .unwrap_or_else(|| format!("{}:{}", reference.provider, reference.effect_id)),
        }
    }

    pub fn capability_id(&self) -> String {
        match self {
            Self::FirstParty { transition } => transition.capability_id().to_owned(),
            Self::ExternalReference { reference } => {
                format!("external:{}:{}", reference.provider, reference.effect_id)
            }
        }
    }

    pub fn external_reference(&self) -> Option<&ExternalEffectReference> {
        match self {
            Self::ExternalReference { reference } => Some(reference),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Transition {
    pub reference: TransitionReference,
    pub duration: Microseconds,
}

impl Transition {
    pub fn dissolve(duration: Microseconds) -> Self {
        Self {
            reference: TransitionReference::dissolve(),
            duration,
        }
    }

    pub fn external_reference(
        provider: impl Into<String>,
        effect_id: impl Into<String>,
        duration: Microseconds,
    ) -> Self {
        Self {
            reference: TransitionReference::ExternalReference {
                reference: ExternalEffectReference::new(provider, effect_id),
            },
            duration,
        }
    }

    pub fn display_name(&self) -> String {
        self.reference.display_name()
    }

    pub fn capability_id(&self) -> String {
        self.reference.capability_id()
    }

    pub fn external(&self) -> Option<&ExternalEffectReference> {
        self.reference.external_reference()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TrackTransition {
    pub from_segment_id: SegmentId,
    pub to_segment_id: SegmentId,
    pub reference: TransitionReference,
    pub duration: Microseconds,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub parameters: BTreeMap<String, String>,
}

impl TrackTransition {
    pub fn dissolve(
        from_segment_id: impl Into<SegmentId>,
        to_segment_id: impl Into<SegmentId>,
        duration: Microseconds,
    ) -> Self {
        Self {
            from_segment_id: from_segment_id.into(),
            to_segment_id: to_segment_id.into(),
            reference: TransitionReference::dissolve(),
            duration,
            parameters: BTreeMap::new(),
        }
    }

    pub fn external_reference(
        from_segment_id: impl Into<SegmentId>,
        to_segment_id: impl Into<SegmentId>,
        provider: impl Into<String>,
        effect_id: impl Into<String>,
        duration: Microseconds,
    ) -> Self {
        Self {
            from_segment_id: from_segment_id.into(),
            to_segment_id: to_segment_id.into(),
            reference: TransitionReference::ExternalReference {
                reference: ExternalEffectReference::new(provider, effect_id),
            },
            duration,
            parameters: BTreeMap::new(),
        }
    }

    pub fn display_name(&self) -> String {
        self.reference.display_name()
    }

    pub fn capability_id(&self) -> String {
        self.reference.capability_id()
    }

    pub fn external(&self) -> Option<&ExternalEffectReference> {
        self.reference.external_reference()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SpeedRatio {
    pub numerator: u32,
    pub denominator: u32,
}

impl SpeedRatio {
    pub const fn one() -> Self {
        Self {
            numerator: 1,
            denominator: 1,
        }
    }

    pub fn new(numerator: u32, denominator: u32) -> Self {
        Self {
            numerator,
            denominator,
        }
    }
}

impl Default for SpeedRatio {
    fn default() -> Self {
        Self::one()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SpeedCurvePoint {
    pub target_time: Microseconds,
    pub speed: SpeedRatio,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum RetimeMode {
    Constant { speed: SpeedRatio },
    SpeedCurve { points: Vec<SpeedCurvePoint> },
}

impl RetimeMode {
    pub fn constant_1x() -> Self {
        Self::Constant {
            speed: SpeedRatio::one(),
        }
    }

    pub fn capability_id(&self) -> &'static str {
        match self {
            Self::Constant { .. } => "retime.constantSpeed",
            Self::SpeedCurve { .. } => "retime.speedCurve",
        }
    }
}

impl Default for RetimeMode {
    fn default() -> Self {
        Self::constant_1x()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum AudioRetimePolicy {
    FollowVideoSpeed,
    PreservePitch,
    MuteUnsupported,
}

impl Default for AudioRetimePolicy {
    fn default() -> Self {
        Self::FollowVideoSpeed
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SegmentRetiming {
    pub mode: RetimeMode,
    #[serde(default)]
    pub audio_policy: AudioRetimePolicy,
}

impl SegmentRetiming {
    pub fn constant_1x() -> Self {
        Self {
            mode: RetimeMode::constant_1x(),
            audio_policy: AudioRetimePolicy::default(),
        }
    }
}

impl Default for SegmentRetiming {
    fn default() -> Self {
        Self::constant_1x()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum MaskKind {
    Rectangle,
    Ellipse,
}

impl MaskKind {
    pub fn capability_id(&self) -> &'static str {
        match self {
            Self::Rectangle => "mask.rectangle",
            Self::Ellipse => "mask.ellipse",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum BlendModeKind {
    Normal,
    Multiply,
    Screen,
}

impl BlendModeKind {
    pub fn capability_id(&self) -> &'static str {
        match self {
            Self::Normal => "blend.normal",
            Self::Multiply => "blend.multiply",
            Self::Screen => "blend.screen",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Normal => "Normal blend",
            Self::Multiply => "Multiply blend",
            Self::Screen => "Screen blend",
        }
    }
}

fn default_enabled() -> bool {
    true
}
