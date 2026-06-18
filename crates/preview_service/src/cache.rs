use draft_model::{DirtyDomain, MaterialId, TargetTimerange};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewCacheKey {
    pub key_id: String,
    pub profile: PreviewCacheProfile,
    pub target_timerange: TargetTimerange,
    pub semantic_fingerprint: String,
    pub material_dependencies: Vec<MaterialId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PreviewCacheProfile {
    FramePng,
    SegmentMp4,
}

impl PreviewCacheProfile {
    pub fn extension(self) -> &'static str {
        match self {
            Self::FramePng => "png",
            Self::SegmentMp4 => "mp4",
        }
    }

    pub fn mime_type(self) -> &'static str {
        match self {
            Self::FramePng => "image/png",
            Self::SegmentMp4 => "video/mp4",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::FramePng => "frame-png",
            Self::SegmentMp4 => "segment-mp4",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewArtifact {
    pub profile: PreviewCacheProfile,
    pub path: String,
    pub mime_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewCacheEntry {
    pub key: PreviewCacheKey,
    pub artifact: PreviewArtifact,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewInvalidationRequest {
    pub changed_ranges: Vec<TargetTimerange>,
    pub changed_material_ids: Vec<MaterialId>,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewInvalidationResult {
    pub retained: Vec<PreviewCacheEntry>,
    pub invalidated: Vec<PreviewCacheEntry>,
}

pub fn changed_range_invalidation(
    range: TargetTimerange,
    reason: impl Into<String>,
) -> PreviewInvalidationRequest {
    PreviewInvalidationRequest {
        changed_ranges: vec![range],
        changed_material_ids: Vec::new(),
        reason: reason.into(),
    }
}

pub fn changed_material_invalidation(
    material_id: MaterialId,
    reason: impl Into<String>,
) -> PreviewInvalidationRequest {
    changed_materials_invalidation([material_id], reason)
}

pub fn changed_materials_invalidation(
    material_ids: impl IntoIterator<Item = MaterialId>,
    reason: impl Into<String>,
) -> PreviewInvalidationRequest {
    PreviewInvalidationRequest {
        changed_ranges: Vec::new(),
        changed_material_ids: material_ids.into_iter().collect(),
        reason: reason.into(),
    }
}

pub fn accepted_timeline_edit_invalidation(
    changed_ranges: impl IntoIterator<Item = TargetTimerange>,
) -> PreviewInvalidationRequest {
    accepted_edit_ranges_invalidation(changed_ranges, "timeline edit")
}

pub fn accepted_text_edit_invalidation(
    changed_ranges: impl IntoIterator<Item = TargetTimerange>,
) -> PreviewInvalidationRequest {
    accepted_edit_ranges_invalidation(changed_ranges, "text edit")
}

pub fn accepted_audio_edit_invalidation(
    changed_ranges: impl IntoIterator<Item = TargetTimerange>,
) -> PreviewInvalidationRequest {
    accepted_edit_ranges_invalidation(changed_ranges, "audio edit")
}

pub fn accepted_edit_ranges_invalidation(
    changed_ranges: impl IntoIterator<Item = TargetTimerange>,
    reason: impl Into<String>,
) -> PreviewInvalidationRequest {
    PreviewInvalidationRequest {
        changed_ranges: changed_ranges.into_iter().collect(),
        changed_material_ids: Vec::new(),
        reason: reason.into(),
    }
}

pub fn consumer_domains_for_dirty_domains(
    domains: impl IntoIterator<Item = DirtyDomain>,
) -> Vec<DirtyDomain> {
    let mut consumers = Vec::new();
    for domain in domains {
        match domain {
            DirtyDomain::Timing => push_all(
                &mut consumers,
                &[
                    DirtyDomain::Preview,
                    DirtyDomain::ExportPrep,
                    DirtyDomain::Audio,
                    DirtyDomain::Thumbnail,
                    DirtyDomain::Proxy,
                    DirtyDomain::GraphSnapshot,
                    DirtyDomain::PreviewCache,
                ],
            ),
            DirtyDomain::Visual => push_all(
                &mut consumers,
                &[
                    DirtyDomain::Preview,
                    DirtyDomain::ExportPrep,
                    DirtyDomain::Thumbnail,
                    DirtyDomain::Proxy,
                    DirtyDomain::GraphSnapshot,
                    DirtyDomain::PreviewCache,
                ],
            ),
            DirtyDomain::Text => push_all(
                &mut consumers,
                &[
                    DirtyDomain::Preview,
                    DirtyDomain::ExportPrep,
                    DirtyDomain::Thumbnail,
                    DirtyDomain::GraphSnapshot,
                    DirtyDomain::PreviewCache,
                ],
            ),
            DirtyDomain::Audio => push_all(
                &mut consumers,
                &[
                    DirtyDomain::Preview,
                    DirtyDomain::ExportPrep,
                    DirtyDomain::Audio,
                    DirtyDomain::Waveform,
                    DirtyDomain::GraphSnapshot,
                    DirtyDomain::PreviewCache,
                ],
            ),
            DirtyDomain::Material | DirtyDomain::RuntimeCapabilities => push_all(
                &mut consumers,
                &[
                    DirtyDomain::Preview,
                    DirtyDomain::ExportPrep,
                    DirtyDomain::Audio,
                    DirtyDomain::Thumbnail,
                    DirtyDomain::Waveform,
                    DirtyDomain::Proxy,
                    DirtyDomain::GraphSnapshot,
                    DirtyDomain::PreviewCache,
                ],
            ),
            DirtyDomain::Canvas | DirtyDomain::OutputProfile => push_all(
                &mut consumers,
                &[
                    DirtyDomain::Preview,
                    DirtyDomain::ExportPrep,
                    DirtyDomain::Thumbnail,
                    DirtyDomain::Proxy,
                    DirtyDomain::GraphSnapshot,
                    DirtyDomain::PreviewCache,
                ],
            ),
            DirtyDomain::Effect | DirtyDomain::Filter | DirtyDomain::Transition => push_all(
                &mut consumers,
                &[
                    DirtyDomain::Preview,
                    DirtyDomain::ExportPrep,
                    DirtyDomain::Thumbnail,
                    DirtyDomain::Proxy,
                    DirtyDomain::GraphSnapshot,
                    DirtyDomain::PreviewCache,
                ],
            ),
            DirtyDomain::Preview
            | DirtyDomain::ExportPrep
            | DirtyDomain::Thumbnail
            | DirtyDomain::Waveform
            | DirtyDomain::Proxy
            | DirtyDomain::GraphSnapshot
            | DirtyDomain::PreviewCache => push_domain(&mut consumers, domain),
        }
    }
    sort_consumer_domains(&mut consumers);
    consumers
}

pub fn invalidate_preview_cache(
    entries: &[PreviewCacheEntry],
    request: &PreviewInvalidationRequest,
) -> PreviewInvalidationResult {
    let mut retained = Vec::new();
    let mut invalidated = Vec::new();

    for entry in entries {
        if should_invalidate(entry, request) {
            invalidated.push(entry.clone());
        } else {
            retained.push(entry.clone());
        }
    }

    PreviewInvalidationResult {
        retained,
        invalidated,
    }
}

fn should_invalidate(entry: &PreviewCacheEntry, request: &PreviewInvalidationRequest) -> bool {
    request
        .changed_ranges
        .iter()
        .any(|range| timeranges_overlap(&entry.key.target_timerange, range))
        || request.changed_material_ids.iter().any(|material_id| {
            entry
                .key
                .material_dependencies
                .iter()
                .any(|dependency| dependency == material_id)
        })
}

fn timeranges_overlap(first: &TargetTimerange, second: &TargetTimerange) -> bool {
    let first_start = first.start.get();
    let Some(first_end) = first_start.checked_add(first.duration.get()) else {
        return false;
    };
    let second_start = second.start.get();
    let Some(second_end) = second_start.checked_add(second.duration.get()) else {
        return false;
    };
    first_start < second_end && second_start < first_end
}

fn push_all(domains: &mut Vec<DirtyDomain>, additions: &[DirtyDomain]) {
    for domain in additions {
        push_domain(domains, *domain);
    }
}

fn push_domain(domains: &mut Vec<DirtyDomain>, domain: DirtyDomain) {
    if !domains.contains(&domain) {
        domains.push(domain);
    }
}

fn sort_consumer_domains(domains: &mut [DirtyDomain]) {
    domains.sort_by_key(|domain| match domain {
        DirtyDomain::Preview => 0,
        DirtyDomain::ExportPrep => 1,
        DirtyDomain::Audio => 2,
        DirtyDomain::Thumbnail => 3,
        DirtyDomain::Waveform => 4,
        DirtyDomain::Proxy => 5,
        DirtyDomain::GraphSnapshot => 6,
        DirtyDomain::PreviewCache => 7,
        _ => 8,
    });
}
