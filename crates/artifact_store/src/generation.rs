use std::path::{Path, PathBuf};

use draft_model::MaterialId;
use media_runtime::CancelToken;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ArtifactStoreError;
use crate::blob_store::{BlobRecord, BlobStore, BlobWriteIntent};
use crate::dependencies::{
    ArtifactDependency, DependencyFingerprint, DependencyUpsert, upsert_artifact_dependencies,
};
use crate::jobs::{
    ArtifactGenerationJob, ArtifactGenerationRequest, ArtifactKind, GenerationChunk,
    GenerationJobStatus, GenerationProgress, complete_generation_chunk, create_generation_job,
    generation_cancel_requested, list_generation_jobs, next_pending_chunk, start_generation_chunk,
};
use crate::resource_index::ResourceId;
use crate::schema::{ArtifactStore, open_artifact_store};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GeneratedArtifactMime {
    VideoMp4,
    ImagePng,
    ApplicationJson,
    ApplicationOctetStream,
}

impl GeneratedArtifactMime {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::VideoMp4 => "video/mp4",
            Self::ImagePng => "image/png",
            Self::ApplicationJson => "application/json",
            Self::ApplicationOctetStream => "application/octet-stream",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedArtifact {
    pub mime: GeneratedArtifactMime,
    pub extension: String,
    pub bytes: Vec<u8>,
}

impl GeneratedArtifact {
    pub fn new(mime: GeneratedArtifactMime, extension: impl Into<String>, bytes: Vec<u8>) -> Self {
        Self {
            mime,
            extension: extension.into(),
            bytes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GeneratedArtifactRecord {
    pub artifact_id: String,
    pub kind: ArtifactKind,
    pub mime: GeneratedArtifactMime,
    pub blob_relative_path: String,
    pub blob_fingerprint: String,
    pub byte_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ArtifactGenerationOutcome {
    pub artifact: GeneratedArtifactRecord,
    pub job: ArtifactGenerationJob,
    pub completed_chunks: Vec<GenerationChunk>,
}

#[derive(Debug)]
pub struct GenerationWorkerContext {
    bundle_path: PathBuf,
    pub job_id: String,
    pub chunk_index: u32,
    pub artifact_id: String,
    pub kind: ArtifactKind,
    pub cancel_token: CancelToken,
}

impl GenerationWorkerContext {
    pub fn bundle_path(&self) -> &Path {
        &self.bundle_path
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel_token.is_cancelled()
    }

    pub fn cancel_requested(&self) -> Result<bool, ArtifactStoreError> {
        if self.cancel_token.is_cancelled() {
            return Ok(true);
        }
        let store = open_artifact_store(&self.bundle_path)?;
        generation_cancel_requested(&store, &self.job_id)
    }
}

pub trait ArtifactGenerator {
    fn generate_proxy(
        &mut self,
        context: &GenerationWorkerContext,
        request: &ProxyGenerationRequest,
    ) -> Result<GeneratedArtifact, ArtifactStoreError>;

    fn generate_thumbnail(
        &mut self,
        context: &GenerationWorkerContext,
        request: &ThumbnailGenerationRequest,
    ) -> Result<GeneratedArtifact, ArtifactStoreError>;

    fn generate_waveform(
        &mut self,
        context: &GenerationWorkerContext,
        request: &WaveformGenerationRequest,
    ) -> Result<GeneratedArtifact, ArtifactStoreError>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProxyGenerationRequest {
    pub job_id: String,
    pub artifact_id: String,
    pub resource_id: ResourceId,
    pub material_id: MaterialId,
    pub source_ref: String,
    pub source_fingerprint: String,
    pub runtime_capability_fingerprint: String,
    pub output_profile_fingerprint: String,
    pub generation_parameters_json: Value,
    pub target_start_us: Option<u64>,
    pub target_duration_us: Option<u64>,
    pub expected_mime: GeneratedArtifactMime,
    pub extension: String,
}

impl ProxyGenerationRequest {
    pub fn into_generation_request(self) -> ArtifactGenerationRequest {
        let stable_key = stable_key(
            ArtifactKind::Proxy,
            &self.artifact_id,
            self.material_id.as_str(),
            &self.output_profile_fingerprint,
        );
        ArtifactGenerationRequest {
            job_id: self.job_id,
            artifact_id: Some(self.artifact_id),
            kind: ArtifactKind::Proxy,
            stable_key,
            generation_parameters_json: self.generation_parameters_json,
            source_fingerprint: Some(self.source_fingerprint),
            runtime_capability_fingerprint: Some(self.runtime_capability_fingerprint),
            output_profile_fingerprint: Some(self.output_profile_fingerprint),
            graph_fingerprint: None,
            chunks: vec![GenerationProgress::new(
                self.target_start_us,
                self.target_duration_us,
                Some(0),
            )],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThumbnailGenerationRequest {
    pub job_id: String,
    pub artifact_id: String,
    pub resource_id: ResourceId,
    pub material_id: MaterialId,
    pub source_ref: String,
    pub source_fingerprint: String,
    pub runtime_capability_fingerprint: String,
    pub output_profile_fingerprint: String,
    pub generation_parameters_json: Value,
    pub target_time_us: u64,
    pub expected_mime: GeneratedArtifactMime,
    pub extension: String,
}

impl ThumbnailGenerationRequest {
    pub fn into_generation_request(self) -> ArtifactGenerationRequest {
        let stable_key = stable_key(
            ArtifactKind::Thumbnail,
            &self.artifact_id,
            self.material_id.as_str(),
            &self.output_profile_fingerprint,
        );
        ArtifactGenerationRequest {
            job_id: self.job_id,
            artifact_id: Some(self.artifact_id),
            kind: ArtifactKind::Thumbnail,
            stable_key,
            generation_parameters_json: self.generation_parameters_json,
            source_fingerprint: Some(self.source_fingerprint),
            runtime_capability_fingerprint: Some(self.runtime_capability_fingerprint),
            output_profile_fingerprint: Some(self.output_profile_fingerprint),
            graph_fingerprint: None,
            chunks: vec![GenerationProgress::new(
                Some(self.target_time_us),
                Some(1),
                Some(0),
            )],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WaveformGenerationRequest {
    pub job_id: String,
    pub artifact_id: String,
    pub resource_id: ResourceId,
    pub material_id: MaterialId,
    pub source_ref: String,
    pub source_fingerprint: String,
    pub runtime_capability_fingerprint: String,
    pub output_profile_fingerprint: String,
    pub generation_parameters_json: Value,
    pub source_start_us: u64,
    pub duration_us: u64,
    pub samples_per_second: u32,
    pub expected_mime: GeneratedArtifactMime,
    pub extension: String,
}

impl WaveformGenerationRequest {
    pub fn into_generation_request(self) -> ArtifactGenerationRequest {
        let stable_key = stable_key(
            ArtifactKind::Waveform,
            &self.artifact_id,
            self.material_id.as_str(),
            &self.output_profile_fingerprint,
        );
        ArtifactGenerationRequest {
            job_id: self.job_id,
            artifact_id: Some(self.artifact_id),
            kind: ArtifactKind::Waveform,
            stable_key,
            generation_parameters_json: self.generation_parameters_json,
            source_fingerprint: Some(self.source_fingerprint),
            runtime_capability_fingerprint: Some(self.runtime_capability_fingerprint),
            output_profile_fingerprint: Some(self.output_profile_fingerprint),
            graph_fingerprint: None,
            chunks: vec![GenerationProgress::new(
                Some(self.source_start_us),
                Some(self.duration_us),
                Some(0),
            )],
        }
    }
}

pub fn generate_proxy_artifact(
    bundle_path: impl AsRef<Path>,
    generator: &mut impl ArtifactGenerator,
    request: ProxyGenerationRequest,
) -> Result<ArtifactGenerationOutcome, ArtifactStoreError> {
    generate_artifact(
        bundle_path.as_ref(),
        generator,
        request.clone().into_generation_request(),
        request.expected_mime,
        |generator, context| generator.generate_proxy(context, &request),
    )
}

pub fn generate_thumbnail_artifact(
    bundle_path: impl AsRef<Path>,
    generator: &mut impl ArtifactGenerator,
    request: ThumbnailGenerationRequest,
) -> Result<ArtifactGenerationOutcome, ArtifactStoreError> {
    generate_artifact(
        bundle_path.as_ref(),
        generator,
        request.clone().into_generation_request(),
        request.expected_mime,
        |generator, context| generator.generate_thumbnail(context, &request),
    )
}

pub fn generate_waveform_artifact(
    bundle_path: impl AsRef<Path>,
    generator: &mut impl ArtifactGenerator,
    request: WaveformGenerationRequest,
) -> Result<ArtifactGenerationOutcome, ArtifactStoreError> {
    generate_artifact(
        bundle_path.as_ref(),
        generator,
        request.clone().into_generation_request(),
        request.expected_mime,
        |generator, context| generator.generate_waveform(context, &request),
    )
}

fn generate_artifact<G, F>(
    bundle_path: &Path,
    generator: &mut G,
    request: ArtifactGenerationRequest,
    expected_mime: GeneratedArtifactMime,
    mut generate: F,
) -> Result<ArtifactGenerationOutcome, ArtifactStoreError>
where
    G: ArtifactGenerator,
    F: FnMut(&mut G, &GenerationWorkerContext) -> Result<GeneratedArtifact, ArtifactStoreError>,
{
    let mut store = open_artifact_store(bundle_path)?;
    if let Some(existing) = existing_job(&store, &request.job_id)? {
        if generation_cancel_requested(&store, &request.job_id)? {
            return generation_cancelled(&request.job_id);
        }
        if existing.status == GenerationJobStatus::Completed {
            return outcome_from_ready_artifact(&store, existing, expected_mime);
        }
    } else {
        create_generation_job(&mut store, request.clone())?;
    }

    if generation_cancel_requested(&store, &request.job_id)? {
        return generation_cancelled(&request.job_id);
    }

    let Some(chunk) = next_pending_chunk(&store, &request.job_id)? else {
        let job = existing_job(&store, &request.job_id)?
            .ok_or_else(|| generation_error(&request.job_id, "job disappeared"))?;
        return outcome_from_ready_artifact(&store, job, expected_mime);
    };

    start_generation_chunk(&mut store, &request.job_id, chunk.chunk_index)?;
    if generation_cancel_requested(&store, &request.job_id)? {
        return generation_cancelled(&request.job_id);
    }

    let context = GenerationWorkerContext {
        bundle_path: bundle_path.to_path_buf(),
        job_id: request.job_id.clone(),
        chunk_index: chunk.chunk_index,
        artifact_id: request
            .artifact_id
            .clone()
            .ok_or_else(|| generation_error(&request.job_id, "artifact id is required"))?,
        kind: request.kind,
        cancel_token: CancelToken::new(),
    };
    let generated = generate(generator, &context)?;
    if generated.bytes.is_empty() {
        return Err(generation_error(
            &request.job_id,
            "generated artifact is empty",
        ));
    }
    if generated.mime != expected_mime {
        return Err(generation_error(&request.job_id, "generated MIME mismatch"));
    }
    if generation_cancel_requested(&store, &request.job_id)? || context.is_cancelled() {
        return generation_cancelled(&request.job_id);
    }

    let record = write_generated_blob(bundle_path, &request, &generated)?;
    upsert_generation_dependencies(&mut store, &request)?;
    let completed = complete_generation_chunk(
        &mut store,
        &request.job_id,
        chunk.chunk_index,
        Some(&record.blob_relative_path),
        Some(record.blob_fingerprint.as_str()),
        record.byte_count,
    )?;
    let job = existing_job(&store, &request.job_id)?
        .ok_or_else(|| generation_error(&request.job_id, "job disappeared after completion"))?;
    Ok(ArtifactGenerationOutcome {
        artifact: record_from_blob(&record, request.kind, generated.mime),
        job,
        completed_chunks: vec![completed],
    })
}

fn write_generated_blob(
    bundle_path: &Path,
    request: &ArtifactGenerationRequest,
    generated: &GeneratedArtifact,
) -> Result<BlobRecord, ArtifactStoreError> {
    let mut blob_store = BlobStore::open(bundle_path)?;
    blob_store.write_blob_atomic(
        BlobWriteIntent {
            artifact_id: request
                .artifact_id
                .clone()
                .ok_or_else(|| generation_error(&request.job_id, "artifact id is required"))?,
            artifact_kind: request.kind.as_str().to_owned(),
            stable_key: request.stable_key.clone(),
            schema_fingerprint: "artifact-store-schema:v1".to_owned(),
            generator_fingerprint: "artifact-generation:v1".to_owned(),
            runtime_capability_fingerprint: request.runtime_capability_fingerprint.clone(),
            source_fingerprint: request.source_fingerprint.clone(),
            graph_fingerprint: request.graph_fingerprint.clone(),
            output_profile_fingerprint: request.output_profile_fingerprint.clone(),
            generation_parameters_json: request.generation_parameters_json.clone(),
            expected_fingerprint: None,
        },
        &generated.bytes,
    )
}

fn upsert_generation_dependencies(
    store: &mut ArtifactStore,
    request: &ArtifactGenerationRequest,
) -> Result<(), ArtifactStoreError> {
    let artifact_id = request
        .artifact_id
        .as_deref()
        .ok_or_else(|| generation_error(&request.job_id, "artifact id is required"))?;
    let mut dependencies = vec![
        DependencyUpsert::new(ArtifactDependency::generation_parameters(
            request.generation_parameters_json.clone(),
        )),
        DependencyUpsert::new(ArtifactDependency::schema_version(1)),
        DependencyUpsert::new(ArtifactDependency::generator_version(
            "artifact-generation:v1",
        )),
    ];
    if let Some(source) = &request.source_fingerprint {
        dependencies.push(DependencyUpsert::new(
            ArtifactDependency::source_fingerprint(DependencyFingerprint::new(
                "source",
                source.clone(),
            )),
        ));
    }
    if let Some(runtime) = &request.runtime_capability_fingerprint {
        dependencies.push(DependencyUpsert::new(
            ArtifactDependency::runtime_capability_fingerprint(DependencyFingerprint::new(
                "runtime",
                runtime.clone(),
            )),
        ));
    }
    if let Some(output) = &request.output_profile_fingerprint {
        dependencies.push(DependencyUpsert::new(
            ArtifactDependency::output_profile_fingerprint(DependencyFingerprint::new(
                "output",
                output.clone(),
            )),
        ));
    }
    if let Some(graph) = &request.graph_fingerprint {
        dependencies.push(DependencyUpsert::new(
            ArtifactDependency::graph_fingerprint(DependencyFingerprint::new(
                "graph",
                graph.clone(),
            )),
        ));
    }
    if let Some(first) = request.chunks.first() {
        if let (Some(start), Some(duration)) = (first.target_start_us, first.target_duration_us) {
            dependencies.push(DependencyUpsert::new(ArtifactDependency::target_range(
                start, duration,
            )));
        }
    }
    upsert_artifact_dependencies(store, artifact_id, dependencies)
}

fn outcome_from_ready_artifact(
    store: &ArtifactStore,
    job: ArtifactGenerationJob,
    mime: GeneratedArtifactMime,
) -> Result<ArtifactGenerationOutcome, ArtifactStoreError> {
    let artifact_id = job
        .artifact_id
        .clone()
        .ok_or_else(|| generation_error(&job.job_id, "artifact id is required"))?;
    let record = ready_artifact_record(store, &artifact_id, job.kind, mime)?;
    let completed_chunks = job
        .chunks
        .iter()
        .filter(|chunk| chunk.status == crate::jobs::GenerationChunkStatus::Completed)
        .cloned()
        .collect();
    Ok(ArtifactGenerationOutcome {
        artifact: record,
        job,
        completed_chunks,
    })
}

fn ready_artifact_record(
    store: &ArtifactStore,
    artifact_id: &str,
    kind: ArtifactKind,
    mime: GeneratedArtifactMime,
) -> Result<GeneratedArtifactRecord, ArtifactStoreError> {
    store
        .connection()
        .query_row(
            "SELECT blob_relative_path, blob_fingerprint, byte_count
             FROM artifact
             WHERE artifact_id = ?1 AND status = 'ready'",
            [artifact_id],
            |row| {
                Ok(GeneratedArtifactRecord {
                    artifact_id: artifact_id.to_owned(),
                    kind,
                    mime,
                    blob_relative_path: row.get(0)?,
                    blob_fingerprint: row.get(1)?,
                    byte_count: row.get::<_, i64>(2)? as u64,
                })
            },
        )
        .map_err(|source| ArtifactStoreError::Sqlite {
            path: store.db_path.clone(),
            source,
        })
}

fn existing_job(
    store: &ArtifactStore,
    job_id: &str,
) -> Result<Option<ArtifactGenerationJob>, ArtifactStoreError> {
    Ok(list_generation_jobs(store)?
        .into_iter()
        .find(|job| job.job_id == job_id))
}

fn record_from_blob(
    record: &BlobRecord,
    kind: ArtifactKind,
    mime: GeneratedArtifactMime,
) -> GeneratedArtifactRecord {
    GeneratedArtifactRecord {
        artifact_id: record.artifact_id.clone(),
        kind,
        mime,
        blob_relative_path: record.blob_relative_path.clone(),
        blob_fingerprint: record.blob_fingerprint.to_string(),
        byte_count: record.byte_count,
    }
}

fn stable_key(
    kind: ArtifactKind,
    artifact_id: &str,
    material_id: &str,
    output_profile_fingerprint: &str,
) -> String {
    format!(
        "artifact:{artifact_id}:material:{material_id}:{}:{output_profile_fingerprint}",
        kind.as_str()
    )
}

fn generation_cancelled<T>(job_id: &str) -> Result<T, ArtifactStoreError> {
    Err(generation_error(job_id, "generation cancelled"))
}

fn generation_error(job_id: &str, reason: impl Into<String>) -> ArtifactStoreError {
    ArtifactStoreError::InvalidDerivedPath {
        path: job_id.to_owned(),
        reason: reason.into(),
    }
}
