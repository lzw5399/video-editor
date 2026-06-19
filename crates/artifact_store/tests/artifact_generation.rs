use std::sync::Mutex;

use artifact_store::generation::{
    ArtifactGenerationOutcome, ArtifactGenerator, GeneratedArtifact, GeneratedArtifactMime,
    GenerationWorkerContext, ProxyGenerationRequest, ThumbnailGenerationRequest,
    WaveformGenerationRequest, generate_proxy_artifact, generate_thumbnail_artifact,
    generate_waveform_artifact,
};
use artifact_store::jobs::{ArtifactKind, GenerationJobStatus, cancel_generation_job};
use artifact_store::paths::derived_root_path;
use artifact_store::resource_index::ResourceId;
use artifact_store::schema::open_artifact_store;
use draft_model::MaterialId;
use serde_json::json;

#[test]
fn generation_proxy_writes_blob_and_persists_ready_artifact() {
    let sandbox = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = sandbox.path().join("draft.veproj");
    let mut generator = FakeArtifactGenerator::new();
    generator.proxy_bytes = b"proxy mp4 bytes".to_vec();

    let outcome = generate_proxy_artifact(
        &bundle_path,
        &mut generator,
        proxy_request("job-proxy", "artifact-proxy"),
    )
    .expect("proxy generation should succeed");

    assert_ready_outcome(&bundle_path, &outcome, ArtifactKind::Proxy, b"proxy mp4 bytes");
    assert_eq!(outcome.job.status, GenerationJobStatus::Completed);
    assert_eq!(outcome.completed_chunks.len(), 1);
    assert_eq!(generator.proxy_calls(), 1);

    let reopened = open_artifact_store(&bundle_path).expect("store should reopen");
    let (status, dirty, path): (String, i64, String) = reopened
        .connection()
        .query_row(
            "SELECT status, dirty, blob_relative_path FROM artifact WHERE artifact_id = ?1",
            ["artifact-proxy"],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("artifact row should exist");
    assert_eq!(status, "ready");
    assert_eq!(dirty, 0);
    assert_eq!(path, outcome.artifact.blob_relative_path);
}

#[test]
fn generation_thumbnail_records_fingerprints_and_project_relative_blob_path() {
    let sandbox = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = sandbox.path().join("draft.veproj");
    let mut generator = FakeArtifactGenerator::new();
    generator.thumbnail_bytes = b"thumbnail png bytes".to_vec();

    let outcome = generate_thumbnail_artifact(
        &bundle_path,
        &mut generator,
        thumbnail_request("job-thumb", "artifact-thumb"),
    )
    .expect("thumbnail generation should succeed");

    assert_ready_outcome(
        &bundle_path,
        &outcome,
        ArtifactKind::Thumbnail,
        b"thumbnail png bytes",
    );
    assert!(
        !outcome.artifact.blob_relative_path.starts_with('/'),
        "blob path must be derived-root-relative"
    );
    assert!(
        !outcome.artifact.blob_relative_path.contains(".veproj/derived"),
        "blob path must not leak cache root"
    );
    assert_eq!(outcome.artifact.mime, GeneratedArtifactMime::ImagePng);

    let reopened = open_artifact_store(&bundle_path).expect("store should reopen");
    let (source_fp, runtime_fp, params): (String, String, String) = reopened
        .connection()
        .query_row(
            "SELECT source_fingerprint, runtime_capability_fingerprint, generation_parameters_json
             FROM artifact WHERE artifact_id = ?1",
            ["artifact-thumb"],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("artifact row should exist");
    assert_eq!(source_fp, "source-thumb-v1");
    assert_eq!(runtime_fp, "runtime-thumb-v1");
    assert!(params.contains("\"width\":320"));
}

#[test]
fn generation_waveform_writes_deterministic_data_and_integer_sample_metadata() {
    let sandbox = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = sandbox.path().join("draft.veproj");
    let mut generator = FakeArtifactGenerator::new();
    generator.waveform_bytes = br#"{"samples":[0,25,50],"durationUs":2000000}"#.to_vec();

    let outcome = generate_waveform_artifact(
        &bundle_path,
        &mut generator,
        waveform_request("job-waveform", "artifact-waveform"),
    )
    .expect("waveform generation should succeed");

    assert_ready_outcome(
        &bundle_path,
        &outcome,
        ArtifactKind::Waveform,
        br#"{"samples":[0,25,50],"durationUs":2000000}"#,
    );
    assert_eq!(outcome.artifact.mime, GeneratedArtifactMime::ApplicationJson);
    assert_eq!(outcome.job.progress.progress_per_mille, Some(1000));

    let reopened = open_artifact_store(&bundle_path).expect("store should reopen");
    let params: String = reopened
        .connection()
        .query_row(
            "SELECT generation_parameters_json FROM artifact WHERE artifact_id = ?1",
            ["artifact-waveform"],
            |row| row.get(0),
        )
        .expect("artifact params should exist");
    assert!(params.contains("\"durationUs\":2000000"));
    assert!(params.contains("\"samplesPerSecond\":100"));
}

#[test]
fn generation_cancellation_prevents_blob_commit_and_resume_skips_completed_chunks() {
    let sandbox = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = sandbox.path().join("draft.veproj");
    let mut generator = FakeArtifactGenerator::new();
    generator.proxy_bytes = b"first proxy bytes".to_vec();

    let first = generate_proxy_artifact(
        &bundle_path,
        &mut generator,
        proxy_request("job-resume-proxy", "artifact-resume-proxy"),
    )
    .expect("first proxy generation should succeed");
    assert_eq!(first.job.status, GenerationJobStatus::Completed);

    let second = generate_proxy_artifact(
        &bundle_path,
        &mut generator,
        proxy_request("job-resume-proxy", "artifact-resume-proxy"),
    )
    .expect("completed proxy generation should resume without rewrite");
    assert_eq!(second.job.status, GenerationJobStatus::Completed);
    assert_eq!(generator.proxy_calls(), 1, "completed chunk must not be regenerated");
    assert_eq!(
        first.artifact.blob_relative_path,
        second.artifact.blob_relative_path
    );

    let mut store = open_artifact_store(&bundle_path).expect("store should reopen");
    let mut generator = FakeArtifactGenerator::new();
    let request = proxy_request("job-cancel-proxy", "artifact-cancel-proxy");
    let created = artifact_store::jobs::create_generation_job(
        &mut store,
        request.clone().into_generation_request(),
    )
    .expect("job should be created");
    assert_eq!(created.status, GenerationJobStatus::Waiting);
    cancel_generation_job(&mut store, "job-cancel-proxy").expect("cancel should persist");

    let error = generate_proxy_artifact(&bundle_path, &mut generator, request)
        .expect_err("cancelled job should not commit blob");
    assert!(
        error.to_string().contains("cancel"),
        "unexpected cancellation error: {error}"
    );
    assert_eq!(generator.proxy_calls(), 0);
    assert_eq!(
        artifact_count(&open_artifact_store(&bundle_path).expect("store should open")),
        1,
        "cancelled generation must not create a ready artifact row"
    );
}

struct FakeArtifactGenerator {
    proxy_bytes: Vec<u8>,
    thumbnail_bytes: Vec<u8>,
    waveform_bytes: Vec<u8>,
    proxy_calls: Mutex<usize>,
}

impl FakeArtifactGenerator {
    fn new() -> Self {
        Self {
            proxy_bytes: Vec::new(),
            thumbnail_bytes: Vec::new(),
            waveform_bytes: Vec::new(),
            proxy_calls: Mutex::new(0),
        }
    }

    fn proxy_calls(&self) -> usize {
        *self.proxy_calls.lock().expect("proxy call lock")
    }
}

impl ArtifactGenerator for FakeArtifactGenerator {
    fn generate_proxy(
        &mut self,
        context: &GenerationWorkerContext,
        _request: &ProxyGenerationRequest,
    ) -> Result<GeneratedArtifact, artifact_store::ArtifactStoreError> {
        *self.proxy_calls.lock().expect("proxy call lock") += 1;
        assert_eq!(context.chunk_index, 0);
        Ok(GeneratedArtifact::new(
            GeneratedArtifactMime::VideoMp4,
            "mp4",
            self.proxy_bytes.clone(),
        ))
    }

    fn generate_thumbnail(
        &mut self,
        _context: &GenerationWorkerContext,
        _request: &ThumbnailGenerationRequest,
    ) -> Result<GeneratedArtifact, artifact_store::ArtifactStoreError> {
        Ok(GeneratedArtifact::new(
            GeneratedArtifactMime::ImagePng,
            "png",
            self.thumbnail_bytes.clone(),
        ))
    }

    fn generate_waveform(
        &mut self,
        _context: &GenerationWorkerContext,
        _request: &WaveformGenerationRequest,
    ) -> Result<GeneratedArtifact, artifact_store::ArtifactStoreError> {
        Ok(GeneratedArtifact::new(
            GeneratedArtifactMime::ApplicationJson,
            "json",
            self.waveform_bytes.clone(),
        ))
    }
}

fn proxy_request(job_id: &str, artifact_id: &str) -> ProxyGenerationRequest {
    ProxyGenerationRequest {
        job_id: job_id.to_owned(),
        artifact_id: artifact_id.to_owned(),
        resource_id: ResourceId::new("material:material-001"),
        material_id: MaterialId::new("material-001"),
        source_ref: "media/source.mp4".to_owned(),
        source_fingerprint: "source-proxy-v1".to_owned(),
        runtime_capability_fingerprint: "runtime-proxy-v1".to_owned(),
        output_profile_fingerprint: "output-proxy-v1".to_owned(),
        generation_parameters_json: json!({
            "width": 960,
            "height": 540,
            "targetBitrateKbps": 1200,
            "sourceStartUs": 0,
            "durationUs": 2_000_000
        }),
        target_start_us: Some(0),
        target_duration_us: Some(2_000_000),
        expected_mime: GeneratedArtifactMime::VideoMp4,
        extension: "mp4".to_owned(),
    }
}

fn thumbnail_request(job_id: &str, artifact_id: &str) -> ThumbnailGenerationRequest {
    ThumbnailGenerationRequest {
        job_id: job_id.to_owned(),
        artifact_id: artifact_id.to_owned(),
        resource_id: ResourceId::new("material:material-001"),
        material_id: MaterialId::new("material-001"),
        source_ref: "media/source.mp4".to_owned(),
        source_fingerprint: "source-thumb-v1".to_owned(),
        runtime_capability_fingerprint: "runtime-thumb-v1".to_owned(),
        output_profile_fingerprint: "output-thumb-v1".to_owned(),
        generation_parameters_json: json!({
            "width": 320,
            "height": 180,
            "targetTimeUs": 500_000
        }),
        target_time_us: 500_000,
        expected_mime: GeneratedArtifactMime::ImagePng,
        extension: "png".to_owned(),
    }
}

fn waveform_request(job_id: &str, artifact_id: &str) -> WaveformGenerationRequest {
    WaveformGenerationRequest {
        job_id: job_id.to_owned(),
        artifact_id: artifact_id.to_owned(),
        resource_id: ResourceId::new("material:material-001"),
        material_id: MaterialId::new("material-001"),
        source_ref: "media/source.mp4".to_owned(),
        source_fingerprint: "source-wave-v1".to_owned(),
        runtime_capability_fingerprint: "runtime-wave-v1".to_owned(),
        output_profile_fingerprint: "output-wave-v1".to_owned(),
        generation_parameters_json: json!({
            "durationUs": 2_000_000,
            "samplesPerSecond": 100,
            "channels": 2
        }),
        source_start_us: 0,
        duration_us: 2_000_000,
        samples_per_second: 100,
        expected_mime: GeneratedArtifactMime::ApplicationJson,
        extension: "json".to_owned(),
    }
}

fn assert_ready_outcome(
    bundle_path: &std::path::Path,
    outcome: &ArtifactGenerationOutcome,
    kind: ArtifactKind,
    bytes: &[u8],
) {
    assert_eq!(outcome.artifact.kind, kind);
    assert!(outcome.artifact.byte_count > 0);
    assert!(outcome.artifact.blob_relative_path.starts_with("blobs/"));
    let blob_path = derived_root_path(bundle_path).join(&outcome.artifact.blob_relative_path);
    assert!(blob_path.is_file(), "blob should exist: {blob_path:?}");
    assert_eq!(std::fs::read(blob_path).expect("blob should read"), bytes);
}

fn artifact_count(store: &artifact_store::schema::ArtifactStore) -> i64 {
    store
        .connection()
        .query_row("SELECT COUNT(*) FROM artifact WHERE status = 'ready'", [], |row| {
            row.get(0)
        })
        .expect("artifact count should read")
}
