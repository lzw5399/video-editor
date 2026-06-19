use draft_model::{
    CommandDelta, DirtyDomain, DirtyRange, DirtyRangeSource, MaterialId, TargetTimerange,
};
use render_graph::{RenderGraphNodeFingerprint, deterministic_fingerprint};
use serde::{Deserialize, Serialize};

pub const PREVIEW_CACHE_ARTIFACT_SCHEMA_VERSION: u32 = 2;
pub const PREVIEW_CACHE_GENERATOR_VERSION: &str = "preview-cache-generator-v2";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreviewCacheKey {
    pub key_id: String,
    pub profile: PreviewCacheProfile,
    pub target_timerange: TargetTimerange,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub graph_node_keys: Vec<String>,
    pub semantic_fingerprint: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub input_fingerprint: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub output_profile_fingerprint: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub runtime_capability_fingerprint: String,
    pub material_dependencies: Vec<MaterialId>,
    #[serde(default, skip_serializing_if = "is_zero")]
    pub artifact_schema_version: u32,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub generator_version: String,
}

impl PreviewCacheKey {
    pub fn from_node_fingerprints(
        profile: PreviewCacheProfile,
        target_timerange: TargetTimerange,
        node_fingerprints: &[RenderGraphNodeFingerprint],
        material_dependencies: impl IntoIterator<Item = MaterialId>,
    ) -> Self {
        let mut graph_node_keys = node_fingerprints
            .iter()
            .map(|fingerprint| fingerprint.node_id.stable_key())
            .collect::<Vec<_>>();
        graph_node_keys.sort();
        graph_node_keys.dedup();

        let semantic_fingerprint = aggregate_fingerprints(
            "preview-cache-semantic",
            node_fingerprints
                .iter()
                .map(|fingerprint| fingerprint.semantic_fingerprint.as_str()),
        );
        let input_fingerprint = aggregate_fingerprints(
            "preview-cache-input",
            node_fingerprints
                .iter()
                .map(|fingerprint| fingerprint.input_fingerprint.as_str()),
        );
        let output_profile_fingerprint = aggregate_fingerprints(
            "preview-cache-output-profile",
            node_fingerprints
                .iter()
                .map(|fingerprint| fingerprint.output_profile_fingerprint.as_str()),
        );
        let runtime_capability_fingerprint = aggregate_fingerprints(
            "preview-cache-runtime",
            node_fingerprints
                .iter()
                .map(|fingerprint| fingerprint.runtime_capability_fingerprint.as_str()),
        );

        let mut material_dependencies = material_dependencies.into_iter().collect::<Vec<_>>();
        material_dependencies.sort_by(|first, second| first.as_str().cmp(second.as_str()));
        material_dependencies.dedup();

        let key_id = deterministic_fingerprint(
            "preview-cache-key-v2",
            &PreviewCacheKeyIdInput {
                profile,
                target_timerange: &target_timerange,
                graph_node_keys: &graph_node_keys,
                semantic_fingerprint: &semantic_fingerprint,
                input_fingerprint: &input_fingerprint,
                output_profile_fingerprint: &output_profile_fingerprint,
                runtime_capability_fingerprint: &runtime_capability_fingerprint,
                material_dependencies: &material_dependencies,
                artifact_schema_version: PREVIEW_CACHE_ARTIFACT_SCHEMA_VERSION,
                generator_version: PREVIEW_CACHE_GENERATOR_VERSION,
            },
        );

        Self {
            key_id: format!("{}-{key_id}", profile.as_str()),
            profile,
            target_timerange,
            graph_node_keys,
            semantic_fingerprint,
            input_fingerprint,
            output_profile_fingerprint,
            runtime_capability_fingerprint,
            material_dependencies,
            artifact_schema_version: PREVIEW_CACHE_ARTIFACT_SCHEMA_VERSION,
            generator_version: PREVIEW_CACHE_GENERATOR_VERSION.to_owned(),
        }
    }

    fn has_complete_v2_facts(&self) -> bool {
        self.artifact_schema_version >= PREVIEW_CACHE_ARTIFACT_SCHEMA_VERSION
            && self.generator_version == PREVIEW_CACHE_GENERATOR_VERSION
            && !self.graph_node_keys.is_empty()
            && !self.semantic_fingerprint.is_empty()
            && !self.input_fingerprint.is_empty()
            && !self.output_profile_fingerprint.is_empty()
            && !self.runtime_capability_fingerprint.is_empty()
    }
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
    pub dirty_ranges: Vec<DirtyRange>,
    pub changed_material_ids: Vec<MaterialId>,
    pub changed_graph_node_keys: Vec<String>,
    pub changed_domains: Vec<DirtyDomain>,
    pub runtime_capability_fingerprint: Option<String>,
    pub output_profile_fingerprint: Option<String>,
    pub full_draft: bool,
    pub reason: String,
}

impl PreviewInvalidationRequest {
    pub fn new(
        dirty_ranges: impl IntoIterator<Item = DirtyRange>,
        changed_material_ids: impl IntoIterator<Item = MaterialId>,
        changed_graph_node_keys: impl IntoIterator<Item = String>,
        changed_domains: impl IntoIterator<Item = DirtyDomain>,
        reason: impl Into<String>,
    ) -> Self {
        let mut request = Self {
            dirty_ranges: dirty_ranges.into_iter().collect(),
            changed_material_ids: changed_material_ids.into_iter().collect(),
            changed_graph_node_keys: changed_graph_node_keys.into_iter().collect(),
            changed_domains: changed_domains.into_iter().collect(),
            runtime_capability_fingerprint: None,
            output_profile_fingerprint: None,
            full_draft: false,
            reason: reason.into(),
        };
        request.normalize();
        request
    }

    pub fn from_command_delta(delta: &CommandDelta) -> Self {
        let changed_domains = if delta.invalidation.consumer_domains.is_empty() {
            consumer_domains_for_dirty_domains(delta.changed_domains.iter().copied())
        } else {
            consumer_domains_for_dirty_domains(
                delta
                    .changed_domains
                    .iter()
                    .chain(delta.invalidation.consumer_domains.iter())
                    .copied(),
            )
        };
        let mut material_ids = delta.invalidation.material_ids.clone();
        material_ids.extend(
            delta
                .changed_entities
                .iter()
                .filter_map(|entity| match entity {
                    draft_model::ChangedEntity::Material { material_id } => {
                        Some(material_id.clone())
                    }
                    _ => None,
                }),
        );

        let mut request = Self {
            dirty_ranges: delta.changed_ranges.clone(),
            changed_material_ids: material_ids,
            changed_graph_node_keys: delta.invalidation.graph_node_ids.clone(),
            changed_domains,
            runtime_capability_fingerprint: None,
            output_profile_fingerprint: None,
            full_draft: delta.invalidation.full_draft,
            reason: delta.reason.clone(),
        };
        request.normalize();
        request
    }

    pub fn full_draft(reason: impl Into<String>) -> Self {
        Self {
            dirty_ranges: Vec::new(),
            changed_material_ids: Vec::new(),
            changed_graph_node_keys: Vec::new(),
            changed_domains: consumer_domains_for_dirty_domains([
                DirtyDomain::Preview,
                DirtyDomain::ExportPrep,
                DirtyDomain::Audio,
                DirtyDomain::Thumbnail,
                DirtyDomain::Waveform,
                DirtyDomain::Proxy,
                DirtyDomain::GraphSnapshot,
                DirtyDomain::PreviewCache,
            ]),
            runtime_capability_fingerprint: None,
            output_profile_fingerprint: None,
            full_draft: true,
            reason: reason.into(),
        }
    }

    pub fn with_runtime_capability_fingerprint(mut self, fingerprint: impl Into<String>) -> Self {
        self.runtime_capability_fingerprint = Some(fingerprint.into());
        self
    }

    pub fn with_output_profile_fingerprint(mut self, fingerprint: impl Into<String>) -> Self {
        self.output_profile_fingerprint = Some(fingerprint.into());
        self
    }

    fn normalize(&mut self) {
        self.changed_material_ids
            .sort_by(|first, second| first.as_str().cmp(second.as_str()));
        self.changed_material_ids.dedup();
        self.changed_graph_node_keys.sort();
        self.changed_graph_node_keys.dedup();
        sort_consumer_domains(&mut self.changed_domains);
        self.changed_domains.dedup();
        match merge_dirty_ranges(std::mem::take(&mut self.dirty_ranges)) {
            Some(merged_ranges) => {
                self.dirty_ranges = merged_ranges;
            }
            None => {
                self.dirty_ranges.clear();
                self.full_draft = true;
                if !self.changed_domains.contains(&DirtyDomain::PreviewCache) {
                    self.changed_domains.push(DirtyDomain::PreviewCache);
                }
                sort_consumer_domains(&mut self.changed_domains);
            }
        }
    }

    fn has_v2_facts(&self) -> bool {
        !self.changed_graph_node_keys.is_empty()
            || self.runtime_capability_fingerprint.is_some()
            || self.output_profile_fingerprint.is_some()
            || self.full_draft
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportPrepDirtyFacts {
    pub dirty_ranges: Vec<DirtyRange>,
    pub changed_material_ids: Vec<MaterialId>,
    pub changed_graph_node_keys: Vec<String>,
    pub changed_domains: Vec<DirtyDomain>,
    pub runtime_capability_fingerprint: Option<String>,
    pub output_profile_fingerprint: Option<String>,
    pub full_draft: bool,
    pub reason: String,
}

impl ExportPrepDirtyFacts {
    pub fn from_invalidation_request(request: &PreviewInvalidationRequest) -> Self {
        Self {
            dirty_ranges: request.dirty_ranges.clone(),
            changed_material_ids: request.changed_material_ids.clone(),
            changed_graph_node_keys: request.changed_graph_node_keys.clone(),
            changed_domains: request.changed_domains.clone(),
            runtime_capability_fingerprint: request.runtime_capability_fingerprint.clone(),
            output_profile_fingerprint: request.output_profile_fingerprint.clone(),
            full_draft: request.full_draft,
            reason: request.reason.clone(),
        }
    }

    pub fn from_command_delta(delta: &CommandDelta) -> Self {
        Self::from_invalidation_request(&PreviewInvalidationRequest::from_command_delta(delta))
    }
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
    accepted_edit_ranges_invalidation([range], reason)
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
    PreviewInvalidationRequest::new([], material_ids, [], [DirtyDomain::PreviewCache], reason)
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
    PreviewInvalidationRequest::new(
        changed_ranges
            .into_iter()
            .map(|target_timerange| DirtyRange {
                target_timerange,
                source: DirtyRangeSource::Current,
            }),
        [],
        [],
        [DirtyDomain::PreviewCache],
        reason,
    )
}

pub fn consumer_domains_for_dirty_domains(
    domains: impl IntoIterator<Item = DirtyDomain>,
) -> Vec<DirtyDomain> {
    let mut consumers = Vec::new();
    for domain in domains {
        match domain {
            DirtyDomain::Track => push_all(
                &mut consumers,
                &[
                    DirtyDomain::Preview,
                    DirtyDomain::ExportPrep,
                    DirtyDomain::GraphSnapshot,
                    DirtyDomain::PreviewCache,
                ],
            ),
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
    if request.full_draft {
        return true;
    }

    if request
        .dirty_ranges
        .iter()
        .any(|range| timeranges_overlap(&entry.key.target_timerange, &range.target_timerange))
        && request_affects_preview_cache(request)
    {
        return true;
    }

    if request.changed_material_ids.iter().any(|material_id| {
        entry
            .key
            .material_dependencies
            .iter()
            .any(|dependency| dependency == material_id)
    }) {
        return true;
    }

    if request.changed_graph_node_keys.iter().any(|changed_key| {
        entry
            .key
            .graph_node_keys
            .iter()
            .any(|entry_key| entry_key == changed_key)
    }) {
        return true;
    }

    fingerprint_or_profile_mismatch(&entry.key, request)
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

fn merge_dirty_ranges(mut ranges: Vec<DirtyRange>) -> Option<Vec<DirtyRange>> {
    ranges.sort_by_key(|range| {
        (
            range.target_timerange.start,
            range.target_timerange.duration,
            range.source as u8,
        )
    });

    let mut merged: Vec<DirtyRange> = Vec::new();
    for range in ranges {
        range.target_timerange.checked_end()?;
        let Some(current) = merged.last_mut() else {
            merged.push(range);
            continue;
        };

        if current.target_timerange.checked_end()?.get() >= range.target_timerange.start.get() {
            current.target_timerange = current.target_timerange.union(&range.target_timerange)?;
            current.source = merge_dirty_range_source(current.source, range.source);
        } else {
            merged.push(range);
        }
    }

    Some(merged)
}

fn merge_dirty_range_source(first: DirtyRangeSource, second: DirtyRangeSource) -> DirtyRangeSource {
    if first == second {
        return first;
    }
    if matches!(first, DirtyRangeSource::FullDraft) || matches!(second, DirtyRangeSource::FullDraft)
    {
        return DirtyRangeSource::FullDraft;
    }
    if matches!(first, DirtyRangeSource::MaterialWide)
        || matches!(second, DirtyRangeSource::MaterialWide)
    {
        return DirtyRangeSource::MaterialWide;
    }
    DirtyRangeSource::PreviousAndCurrent
}

fn aggregate_fingerprints<'a>(
    namespace: &str,
    values: impl IntoIterator<Item = &'a str>,
) -> String {
    let mut values = values
        .into_iter()
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    match values.as_slice() {
        [] => String::new(),
        [single] => single.clone(),
        _ => deterministic_fingerprint(namespace, &values),
    }
}

fn request_affects_preview_cache(request: &PreviewInvalidationRequest) -> bool {
    request.changed_domains.is_empty()
        || request.changed_domains.contains(&DirtyDomain::Preview)
        || request.changed_domains.contains(&DirtyDomain::PreviewCache)
}

fn fingerprint_or_profile_mismatch(
    key: &PreviewCacheKey,
    request: &PreviewInvalidationRequest,
) -> bool {
    if request.has_v2_facts() && !key.has_complete_v2_facts() {
        return true;
    }

    if request
        .runtime_capability_fingerprint
        .as_ref()
        .is_some_and(|fingerprint| &key.runtime_capability_fingerprint != fingerprint)
    {
        return true;
    }

    request
        .output_profile_fingerprint
        .as_ref()
        .is_some_and(|fingerprint| &key.output_profile_fingerprint != fingerprint)
}

fn is_zero(value: &u32) -> bool {
    *value == 0
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PreviewCacheKeyIdInput<'a> {
    profile: PreviewCacheProfile,
    target_timerange: &'a TargetTimerange,
    graph_node_keys: &'a [String],
    semantic_fingerprint: &'a str,
    input_fingerprint: &'a str,
    output_profile_fingerprint: &'a str,
    runtime_capability_fingerprint: &'a str,
    material_dependencies: &'a [MaterialId],
    artifact_schema_version: u32,
    generator_version: &'a str,
}
