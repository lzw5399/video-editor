use draft_model::{DirtyDomain, DirtyRange, DraftId, MaterialId, SegmentId, TrackId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderGraphNodeId {
    pub role: RenderGraphNodeRole,
    pub draft_id: DraftId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub track_id: Option<TrackId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub segment_id: Option<SegmentId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material_id: Option<MaterialId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_id: Option<String>,
}

impl RenderGraphNodeId {
    pub fn canvas(draft_id: &DraftId) -> Self {
        Self::new(RenderGraphNodeRole::Canvas, draft_id)
    }

    pub fn material(draft_id: &DraftId, material_id: &MaterialId) -> Self {
        Self::new(RenderGraphNodeRole::Material, draft_id).with_material_id(material_id)
    }

    pub fn video_segment(
        draft_id: &DraftId,
        track_id: &TrackId,
        segment_id: &SegmentId,
        material_id: &MaterialId,
    ) -> Self {
        Self::new(RenderGraphNodeRole::VideoSegment, draft_id)
            .with_track_id(track_id)
            .with_segment_id(segment_id)
            .with_material_id(material_id)
    }

    pub fn audio_segment(
        draft_id: &DraftId,
        track_id: &TrackId,
        segment_id: &SegmentId,
        material_id: &MaterialId,
    ) -> Self {
        Self::new(RenderGraphNodeRole::AudioSegment, draft_id)
            .with_track_id(track_id)
            .with_segment_id(segment_id)
            .with_material_id(material_id)
    }

    pub fn text_overlay(
        draft_id: &DraftId,
        track_id: &TrackId,
        segment_id: &SegmentId,
        material_id: &MaterialId,
    ) -> Self {
        Self::new(RenderGraphNodeRole::TextOverlay, draft_id)
            .with_track_id(track_id)
            .with_segment_id(segment_id)
            .with_material_id(material_id)
    }

    pub fn segment_filter(
        draft_id: &DraftId,
        track_id: &TrackId,
        segment_id: &SegmentId,
        material_id: &MaterialId,
        filter_index: usize,
    ) -> Self {
        Self::new(RenderGraphNodeRole::SegmentFilter, draft_id)
            .with_track_id(track_id)
            .with_segment_id(segment_id)
            .with_material_id(material_id)
            .with_local_id(filter_index.to_string())
    }

    pub fn segment_transition(
        draft_id: &DraftId,
        track_id: &TrackId,
        segment_id: &SegmentId,
        material_id: &MaterialId,
    ) -> Self {
        Self::new(RenderGraphNodeRole::SegmentTransition, draft_id)
            .with_track_id(track_id)
            .with_segment_id(segment_id)
            .with_material_id(material_id)
    }

    pub fn sampled_frame(draft_id: &DraftId, frame_index: u64, at_microseconds: u64) -> Self {
        Self::new(RenderGraphNodeRole::SampledFrame, draft_id)
            .with_local_id(format!("{frame_index}:at:{at_microseconds}"))
    }

    pub fn stable_key(&self) -> String {
        match self.role {
            RenderGraphNodeRole::Canvas => {
                format!("draft:{}:canvas", self.draft_id.as_str())
            }
            RenderGraphNodeRole::Material => {
                format!(
                    "draft:{}:material:{}",
                    self.draft_id.as_str(),
                    self.material_id_value()
                )
            }
            RenderGraphNodeRole::VideoSegment => self.segment_role_key("video"),
            RenderGraphNodeRole::AudioSegment => self.segment_role_key("audio"),
            RenderGraphNodeRole::TextOverlay => self.segment_role_key("text"),
            RenderGraphNodeRole::SegmentFilter => {
                format!(
                    "{}:filter:{}",
                    self.segment_role_prefix(),
                    self.local_id_value()
                )
            }
            RenderGraphNodeRole::SegmentTransition => {
                format!("{}:transition", self.segment_role_prefix())
            }
            RenderGraphNodeRole::AudioMix => {
                format!(
                    "draft:{}:audio-mix:{}",
                    self.draft_id.as_str(),
                    self.local_id_value()
                )
            }
            RenderGraphNodeRole::VideoComposite => {
                format!(
                    "draft:{}:video-composite:{}",
                    self.draft_id.as_str(),
                    self.local_id_value()
                )
            }
            RenderGraphNodeRole::SampledFrame => {
                format!(
                    "draft:{}:frame:{}",
                    self.draft_id.as_str(),
                    self.local_id_value()
                )
            }
            RenderGraphNodeRole::Output => {
                format!(
                    "draft:{}:output:{}",
                    self.draft_id.as_str(),
                    self.local_id_value()
                )
            }
        }
    }

    fn new(role: RenderGraphNodeRole, draft_id: &DraftId) -> Self {
        Self {
            role,
            draft_id: draft_id.clone(),
            track_id: None,
            segment_id: None,
            material_id: None,
            local_id: None,
        }
    }

    fn with_track_id(mut self, track_id: &TrackId) -> Self {
        self.track_id = Some(track_id.clone());
        self
    }

    fn with_segment_id(mut self, segment_id: &SegmentId) -> Self {
        self.segment_id = Some(segment_id.clone());
        self
    }

    fn with_material_id(mut self, material_id: &MaterialId) -> Self {
        self.material_id = Some(material_id.clone());
        self
    }

    fn with_local_id(mut self, local_id: String) -> Self {
        self.local_id = Some(local_id);
        self
    }

    fn segment_role_key(&self, role_suffix: &str) -> String {
        format!("{}:{role_suffix}", self.segment_role_prefix())
    }

    fn segment_role_prefix(&self) -> String {
        format!(
            "draft:{}:track:{}:segment:{}",
            self.draft_id.as_str(),
            self.track_id_value(),
            self.segment_id_value()
        )
    }

    fn track_id_value(&self) -> &str {
        self.track_id
            .as_ref()
            .map(TrackId::as_str)
            .unwrap_or("none")
    }

    fn segment_id_value(&self) -> &str {
        self.segment_id
            .as_ref()
            .map(SegmentId::as_str)
            .unwrap_or("none")
    }

    fn material_id_value(&self) -> &str {
        self.material_id
            .as_ref()
            .map(MaterialId::as_str)
            .unwrap_or("none")
    }

    fn local_id_value(&self) -> &str {
        self.local_id.as_deref().unwrap_or("none")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RenderGraphNodeRole {
    Canvas,
    Material,
    VideoSegment,
    AudioSegment,
    TextOverlay,
    SegmentFilter,
    SegmentTransition,
    AudioMix,
    VideoComposite,
    SampledFrame,
    Output,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderGraphDiff {
    pub added: Vec<RenderGraphNodeId>,
    pub removed: Vec<RenderGraphNodeId>,
    pub changed: Vec<RenderGraphNodeChange>,
    pub unchanged: Vec<RenderGraphNodeId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dirty_ranges: Vec<DirtyRange>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dirty_domains: Vec<DirtyDomain>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RenderGraphNodeChange {
    pub node_id: RenderGraphNodeId,
}
