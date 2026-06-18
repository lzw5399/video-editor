use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;

use draft_model::{MaterialId, Microseconds, RationalFrameRate};

use crate::{
    CpuVideoFrame, FrameValidationError, FrameValidationErrorKind, PlaybackGeneration,
    PreviewFrameInput, PreviewFrameProvider, PreviewFrameProviderError,
};

const PROVIDER_NAME: &str = "software-video-frame-provider";
const SUPPORTED_H264_CODEC: &str = "h264";

#[derive(Debug, Default, Clone)]
pub struct DecodedVideoFrameCache {
    entries: BTreeMap<MaterialId, DecodedVideoFrameSet>,
}

impl DecodedVideoFrameCache {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }

    pub fn insert_h264_frames(
        &mut self,
        material_id: MaterialId,
        frame_rate: RationalFrameRate,
        total_frame_count: u64,
        frames: Vec<(u64, CpuVideoFrame)>,
    ) -> Result<(), FrameValidationError> {
        self.insert_codec_frames(
            material_id,
            SUPPORTED_H264_CODEC,
            frame_rate,
            total_frame_count,
            frames,
        )
    }

    pub fn insert_codec_frames(
        &mut self,
        material_id: MaterialId,
        codec: impl Into<String>,
        frame_rate: RationalFrameRate,
        total_frame_count: u64,
        frames: Vec<(u64, CpuVideoFrame)>,
    ) -> Result<(), FrameValidationError> {
        if material_id.is_empty() {
            return Err(FrameValidationError::new_public(
                FrameValidationErrorKind::MissingMaterialId,
                "decoded cache material id must be present",
            ));
        }
        if frame_rate.numerator == 0 || frame_rate.denominator == 0 {
            return Err(FrameValidationError::new_public(
                FrameValidationErrorKind::InvalidDimensions,
                "decoded cache frame rate must be nonzero",
            ));
        }
        if total_frame_count == 0 {
            return Err(FrameValidationError::new_public(
                FrameValidationErrorKind::InvalidDimensions,
                "decoded cache total frame count must be nonzero",
            ));
        }

        let mut decoded_frames = BTreeMap::new();
        for (frame_index, frame) in frames {
            frame.validate()?;
            if frame.material_id != material_id {
                return Err(FrameValidationError::new_public(
                    FrameValidationErrorKind::MissingMaterialId,
                    "decoded frame material id must match cache material id",
                ));
            }
            if frame_index >= total_frame_count {
                return Err(FrameValidationError::new_public(
                    FrameValidationErrorKind::InvalidPixelLength,
                    "decoded frame index exceeds cache frame count",
                ));
            }
            decoded_frames.insert(frame_index, frame);
        }

        self.entries.insert(
            material_id,
            DecodedVideoFrameSet {
                codec: codec.into(),
                frame_rate,
                total_frame_count,
                frames: decoded_frames,
            },
        );
        Ok(())
    }

    fn get(&self, material_id: &MaterialId) -> Option<&DecodedVideoFrameSet> {
        self.entries.get(material_id)
    }
}

#[derive(Debug, Clone)]
struct DecodedVideoFrameSet {
    codec: String,
    frame_rate: RationalFrameRate,
    total_frame_count: u64,
    frames: BTreeMap<u64, CpuVideoFrame>,
}

#[derive(Debug, Clone)]
pub struct SoftwareVideoFrameProvider {
    cache: DecodedVideoFrameCache,
    _process_invocation_counter: Option<Arc<AtomicUsize>>,
}

impl SoftwareVideoFrameProvider {
    pub fn new(cache: DecodedVideoFrameCache) -> Self {
        Self {
            cache,
            _process_invocation_counter: None,
        }
    }

    pub fn with_process_invocation_counter(mut self, counter: Arc<AtomicUsize>) -> Self {
        self._process_invocation_counter = Some(counter);
        self
    }
}

impl PreviewFrameProvider for SoftwareVideoFrameProvider {
    fn provider_name(&self) -> &'static str {
        PROVIDER_NAME
    }

    fn frame_for(
        &mut self,
        material_id: &MaterialId,
        source_position: Microseconds,
        playback_generation: PlaybackGeneration,
    ) -> Result<PreviewFrameInput, PreviewFrameProviderError> {
        let Some(entry) = self.cache.get(material_id) else {
            return Err(PreviewFrameProviderError::unavailable(
                self.provider_name(),
                material_id.clone(),
                source_position,
                playback_generation,
                "no decoded video frames loaded for material",
            ));
        };

        if entry.codec != SUPPORTED_H264_CODEC {
            return Err(PreviewFrameProviderError::unsupported_codec(
                self.provider_name(),
                material_id.clone(),
                entry.codec.clone(),
                "software realtime preview cache supports generated H.264 MP4/MOV frames only",
            ));
        }

        let frame_index = frame_index_for_source_position(source_position, &entry.frame_rate)
            .map_err(|error| {
                PreviewFrameProviderError::invalid_frame(
                    self.provider_name(),
                    Some(material_id.clone()),
                    Some(source_position),
                    Some(playback_generation),
                    error,
                )
            })?;

        if frame_index >= entry.total_frame_count {
            return Err(PreviewFrameProviderError::out_of_range(
                self.provider_name(),
                material_id.clone(),
                source_position,
                playback_generation,
                format!(
                    "source position maps to frame {frame_index}, but cache has {} frames",
                    entry.total_frame_count
                ),
            ));
        }

        let Some(frame) = entry.frames.get(&frame_index) else {
            return Err(PreviewFrameProviderError::unavailable(
                self.provider_name(),
                material_id.clone(),
                source_position,
                playback_generation,
                format!("decoded frame {frame_index} is not present in the session cache"),
            ));
        };

        let mut frame = frame.clone();
        frame.source_position = source_position;
        frame.playback_generation = playback_generation;
        Ok(PreviewFrameInput::CpuRgba(frame))
    }
}

fn frame_index_for_source_position(
    source_position: Microseconds,
    frame_rate: &RationalFrameRate,
) -> Result<u64, FrameValidationError> {
    if frame_rate.numerator == 0 || frame_rate.denominator == 0 {
        return Err(FrameValidationError::new_public(
            FrameValidationErrorKind::InvalidDimensions,
            "frame rate must be nonzero",
        ));
    }

    let numerator = u128::from(source_position.get()) * u128::from(frame_rate.numerator);
    let denominator = 1_000_000_u128 * u128::from(frame_rate.denominator);
    u64::try_from(numerator / denominator).map_err(|_| {
        FrameValidationError::new_public(
            FrameValidationErrorKind::InvalidPixelLength,
            "source position maps beyond u64 frame index",
        )
    })
}
