use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{MaterialId, RationalFrameRate};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DraftCanvasConfig {
    pub aspect_ratio: CanvasAspectRatio,
    pub width: u32,
    pub height: u32,
    pub frame_rate: RationalFrameRate,
    pub background: CanvasBackground,
}

impl DraftCanvasConfig {
    pub const DEFAULT_WIDTH: u32 = 1920;
    pub const DEFAULT_HEIGHT: u32 = 1080;

    pub fn mvp_default() -> Self {
        Self {
            aspect_ratio: CanvasAspectRatio::preset(CanvasAspectRatioPreset::Ratio16x9),
            width: Self::DEFAULT_WIDTH,
            height: Self::DEFAULT_HEIGHT,
            frame_rate: RationalFrameRate::new(30, 1),
            background: CanvasBackground::Black,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CanvasAspectRatio {
    Preset { preset: CanvasAspectRatioPreset },
    Custom { numerator: u32, denominator: u32 },
}

impl CanvasAspectRatio {
    pub fn preset(preset: CanvasAspectRatioPreset) -> Self {
        Self::Preset { preset }
    }

    pub fn custom(numerator: u32, denominator: u32) -> Self {
        Self::Custom {
            numerator,
            denominator,
        }
    }

    pub fn ratio(&self) -> Option<(u32, u32)> {
        match self {
            Self::Preset { preset } => Some(preset.ratio()),
            Self::Custom {
                numerator,
                denominator,
            } => reduce_ratio(*numerator, *denominator),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum CanvasAspectRatioPreset {
    Ratio16x9,
    Ratio9x16,
    Ratio1x1,
    Ratio4x3,
    Ratio3x4,
}

impl CanvasAspectRatioPreset {
    pub fn ratio(self) -> (u32, u32) {
        match self {
            Self::Ratio16x9 => (16, 9),
            Self::Ratio9x16 => (9, 16),
            Self::Ratio1x1 => (1, 1),
            Self::Ratio4x3 => (4, 3),
            Self::Ratio3x4 => (3, 4),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum CanvasBackground {
    Black,
    SolidColor {
        color: String,
    },
    BlurFill,
    Image {
        #[serde(rename = "materialId")]
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[ts(optional = nullable)]
        material_id: Option<MaterialId>,
    },
}

impl CanvasBackground {
    pub fn capability(&self) -> CanvasBackgroundCapability {
        match self {
            Self::Black | Self::SolidColor { .. } => CanvasBackgroundCapability::Supported,
            Self::BlurFill => CanvasBackgroundCapability::Degraded,
            Self::Image { .. } => CanvasBackgroundCapability::Unsupported,
        }
    }

    pub fn image_material_id(&self) -> Option<&MaterialId> {
        match self {
            Self::Image { material_id } => material_id.as_ref(),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase")]
pub enum CanvasBackgroundCapability {
    Supported,
    Degraded,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NormalizedCanvasPoint {
    pub x: f64,
    pub y: f64,
}

impl NormalizedCanvasPoint {
    pub const CENTER: Self = Self { x: 0.0, y: 0.0 };
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, JsonSchema, TS)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CanvasPixelPoint {
    pub x: f64,
    pub y: f64,
}

pub fn canvas_pixel_to_normalized(
    width: u32,
    height: u32,
    pixel: CanvasPixelPoint,
) -> Option<NormalizedCanvasPoint> {
    if width == 0 || height == 0 {
        return None;
    }
    let half_width = f64::from(width) / 2.0;
    let half_height = f64::from(height) / 2.0;
    Some(NormalizedCanvasPoint {
        x: (pixel.x - half_width) / half_width,
        y: (half_height - pixel.y) / half_height,
    })
}

pub fn normalized_to_canvas_pixel(
    width: u32,
    height: u32,
    point: NormalizedCanvasPoint,
) -> Option<CanvasPixelPoint> {
    if width == 0 || height == 0 {
        return None;
    }
    let half_width = f64::from(width) / 2.0;
    let half_height = f64::from(height) / 2.0;
    Some(CanvasPixelPoint {
        x: half_width + point.x * half_width,
        y: half_height - point.y * half_height,
    })
}

pub fn reduce_ratio(numerator: u32, denominator: u32) -> Option<(u32, u32)> {
    if numerator == 0 || denominator == 0 {
        return None;
    }
    let divisor = greatest_common_divisor(numerator, denominator);
    Some((numerator / divisor, denominator / divisor))
}

fn greatest_common_divisor(mut left: u32, mut right: u32) -> u32 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}
