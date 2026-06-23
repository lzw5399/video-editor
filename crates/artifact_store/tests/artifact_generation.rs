use std::sync::Mutex;

use artifact_store::generation::{
    ArtifactGenerationOutcome, ArtifactGenerator, GeneratedArtifact, GeneratedArtifactMime,
    GenerationWorkerContext, ProxyGenerationRequest, ThumbnailGenerationRequest,
    WaveformGenerationRequest, generate_proxy_artifact, generate_thumbnail_artifact,
    generate_waveform_artifact,
};
use artifact_store::invalidation::{
    SourceChange, SourceChangeKind, mark_dirty_for_source_change, mark_dirty_from_command_delta,
};
use artifact_store::jobs::{
    ArtifactKind, GenerationChunkStatus, GenerationJobStatus, cancel_generation_job,
    list_generation_jobs,
};
use artifact_store::paths::derived_root_path;
use artifact_store::resource_index::ResourceId;
use artifact_store::schema::open_artifact_store;
use draft_model::{
    ChangedEntity, CommandDelta, CommandDeltaName, DirtyDomain, InvalidationScope, MaterialId,
};
use serde_json::json;

#[test]
fn artifact_generation_proxy_writes_blob_and_persists_ready_artifact() {
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

    assert_ready_outcome(
        &bundle_path,
        &outcome,
        ArtifactKind::Proxy,
        b"proxy mp4 bytes",
    );
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
fn artifact_generation_thumbnail_records_fingerprints_and_project_relative_blob_path() {
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
        !outcome
            .artifact
            .blob_relative_path
            .contains(".veproj/derived"),
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
fn artifact_generation_waveform_writes_deterministic_data_and_integer_sample_metadata() {
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
    assert_eq!(
        outcome.artifact.mime,
        GeneratedArtifactMime::ApplicationJson
    );
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
fn artifact_generation_cancellation_prevents_blob_commit_and_resume_skips_completed_chunks() {
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
    assert_eq!(
        generator.proxy_calls(),
        1,
        "completed chunk must not be regenerated"
    );
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

#[test]
fn artifact_generation_worker_context_exposes_persisted_cancel_probe() {
    let sandbox = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = sandbox.path().join("draft.veproj");
    let request = proxy_request("job-context-cancel", "artifact-context-cancel");

    let mut generator = CancelAwareGenerator;
    let error = generate_proxy_artifact(&bundle_path, &mut generator, request)
        .expect_err("cancel-aware generator should observe mid-chunk persisted cancel");
    assert!(
        error.to_string().contains("cancel"),
        "unexpected cancel probe error: {error}"
    );

    let job = persisted_job(&bundle_path, "job-context-cancel");
    assert_eq!(job.status, GenerationJobStatus::Cancelled);
    assert_eq!(job.chunks[0].status, GenerationChunkStatus::Cancelled);
    assert_eq!(
        artifact_count(&open_artifact_store(&bundle_path).expect("store should open")),
        0
    );
}

#[test]
fn artifact_generation_failure_paths_persist_terminal_status() {
    let cases = [
        FailureMode::GeneratorError,
        FailureMode::EmptyOutput,
        FailureMode::MimeMismatch,
    ];

    for mode in cases {
        let sandbox = tempfile::tempdir().expect("tempdir should be created");
        let bundle_path = sandbox.path().join("draft.veproj");
        let request = proxy_request(
            &format!("job-failure-{}", mode.label()),
            &format!("artifact-failure-{}", mode.label()),
        );
        let mut generator = FailureGenerator { mode };

        let error = generate_proxy_artifact(&bundle_path, &mut generator, request.clone())
            .expect_err("failed generation should return an error");
        assert!(
            error.to_string().contains(mode.expected_error()),
            "unexpected generation error for {mode:?}: {error}"
        );

        let job = persisted_job(&bundle_path, &request.job_id);
        assert_eq!(
            job.status,
            GenerationJobStatus::Failed,
            "failed generation should not leave job running for {mode:?}"
        );
        assert_eq!(
            job.chunks[0].status,
            GenerationChunkStatus::Failed,
            "failed generation should not leave chunk running for {mode:?}"
        );
        assert_eq!(
            artifact_count(&open_artifact_store(&bundle_path).expect("store should open")),
            0,
            "failed generation must not create a ready artifact row for {mode:?}"
        );
    }
}

#[test]
fn artifact_generation_records_material_resource_and_source_dependencies_for_invalidation() {
    let sandbox = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = sandbox.path().join("draft.veproj");
    let mut generator = FakeArtifactGenerator::new();
    generator.proxy_bytes = b"proxy bytes".to_vec();
    generator.thumbnail_bytes = b"thumb bytes".to_vec();
    generator.waveform_bytes = br#"{"samples":[1,2,3]}"#.to_vec();

    generate_proxy_artifact(
        &bundle_path,
        &mut generator,
        proxy_request("job-dep-proxy", "artifact-dep-proxy"),
    )
    .expect("proxy should generate");
    generate_thumbnail_artifact(
        &bundle_path,
        &mut generator,
        thumbnail_request("job-dep-thumb", "artifact-dep-thumb"),
    )
    .expect("thumbnail should generate");
    generate_waveform_artifact(
        &bundle_path,
        &mut generator,
        waveform_request("job-dep-wave", "artifact-dep-wave"),
    )
    .expect("waveform should generate");

    let mut store = open_artifact_store(&bundle_path).expect("store should reopen");
    let delta = CommandDelta::targeted(
        CommandDeltaName::ImportMaterial,
        vec![ChangedEntity::Material {
            material_id: MaterialId::new("material-001"),
        }],
        vec![DirtyDomain::Material],
        Vec::new(),
        InvalidationScope::targeted(
            vec![MaterialId::new("material-001")],
            vec![
                DirtyDomain::Proxy,
                DirtyDomain::Thumbnail,
                DirtyDomain::Waveform,
            ],
        ),
        "material relinked",
    );
    let dirty = mark_dirty_from_command_delta(&mut store, &delta)
        .expect("material command delta should dirty generated artifacts");
    assert_dirty_ids(
        &dirty.dirty_artifacts,
        &[
            "artifact-dep-proxy",
            "artifact-dep-thumb",
            "artifact-dep-wave",
        ],
    );

    store
        .connection()
        .execute("UPDATE artifact SET status = 'ready', dirty = 0", [])
        .expect("artifacts should reset for source change check");
    let deleted = mark_dirty_for_source_change(
        &mut store,
        SourceChange {
            kind: SourceChangeKind::Deleted,
            material_id: Some(MaterialId::new("material-001")),
            resource_id: Some("material:material-001".to_owned()),
            old_project_relative_ref: Some("media/source.mp4".to_owned()),
            new_project_relative_ref: None,
            old_source_fingerprint: Some("source-proxy-v1".to_owned()),
            new_source_fingerprint: None,
            reason: "source deleted".to_owned(),
        },
    )
    .expect("source delete should tombstone generated artifacts");
    assert_dirty_ids(
        &deleted.dirty_artifacts,
        &[
            "artifact-dep-proxy",
            "artifact-dep-thumb",
            "artifact-dep-wave",
        ],
    );
    for artifact_id in [
        "artifact-dep-proxy",
        "artifact-dep-thumb",
        "artifact-dep-wave",
    ] {
        let status: String = store
            .connection()
            .query_row(
                "SELECT status FROM artifact WHERE artifact_id = ?1",
                [artifact_id],
                |row| row.get(0),
            )
            .expect("artifact status should exist");
        assert_eq!(status, "tombstoned");
    }
}

struct FakeArtifactGenerator {
    proxy_bytes: Vec<u8>,
    thumbnail_bytes: Vec<u8>,
    waveform_bytes: Vec<u8>,
    proxy_calls: Mutex<usize>,
}

#[derive(Debug, Clone, Copy)]
enum FailureMode {
    GeneratorError,
    EmptyOutput,
    MimeMismatch,
}

impl FailureMode {
    fn label(self) -> &'static str {
        match self {
            Self::GeneratorError => "generator",
            Self::EmptyOutput => "empty",
            Self::MimeMismatch => "mime",
        }
    }

    fn expected_error(self) -> &'static str {
        match self {
            Self::GeneratorError => "generator failed",
            Self::EmptyOutput => "generated artifact is empty",
            Self::MimeMismatch => "generated MIME mismatch",
        }
    }
}

struct FailureGenerator {
    mode: FailureMode,
}

impl ArtifactGenerator for FailureGenerator {
    fn generate_proxy(
        &mut self,
        _context: &GenerationWorkerContext,
        _request: &ProxyGenerationRequest,
    ) -> Result<GeneratedArtifact, artifact_store::ArtifactStoreError> {
        match self.mode {
            FailureMode::GeneratorError => {
                Err(artifact_store::ArtifactStoreError::InvalidDerivedPath {
                    path: "artifact-failure".to_owned(),
                    reason: "generator failed".to_owned(),
                })
            }
            FailureMode::EmptyOutput => Ok(GeneratedArtifact::new(
                GeneratedArtifactMime::VideoMp4,
                "mp4",
                Vec::new(),
            )),
            FailureMode::MimeMismatch => Ok(GeneratedArtifact::new(
                GeneratedArtifactMime::ImagePng,
                "png",
                b"wrong mime".to_vec(),
            )),
        }
    }

    fn generate_thumbnail(
        &mut self,
        _context: &GenerationWorkerContext,
        _request: &ThumbnailGenerationRequest,
    ) -> Result<GeneratedArtifact, artifact_store::ArtifactStoreError> {
        unreachable!("failure test should only generate proxy")
    }

    fn generate_waveform(
        &mut self,
        _context: &GenerationWorkerContext,
        _request: &WaveformGenerationRequest,
    ) -> Result<GeneratedArtifact, artifact_store::ArtifactStoreError> {
        unreachable!("failure test should only generate proxy")
    }
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

struct CancelAwareGenerator;

impl ArtifactGenerator for CancelAwareGenerator {
    fn generate_proxy(
        &mut self,
        context: &GenerationWorkerContext,
        _request: &ProxyGenerationRequest,
    ) -> Result<GeneratedArtifact, artifact_store::ArtifactStoreError> {
        let mut store = open_artifact_store(context.bundle_path()).expect("store should open");
        cancel_generation_job(&mut store, &context.job_id).expect("cancel should persist");
        if context.cancel_requested()? {
            return Err(artifact_store::ArtifactStoreError::InvalidDerivedPath {
                path: context.job_id.clone(),
                reason: "generation cancelled by persisted probe".to_owned(),
            });
        }
        Ok(GeneratedArtifact::new(
            GeneratedArtifactMime::VideoMp4,
            "mp4",
            b"should-not-write".to_vec(),
        ))
    }

    fn generate_thumbnail(
        &mut self,
        _context: &GenerationWorkerContext,
        _request: &ThumbnailGenerationRequest,
    ) -> Result<GeneratedArtifact, artifact_store::ArtifactStoreError> {
        unreachable!("cancel-aware proxy test should not call thumbnail")
    }

    fn generate_waveform(
        &mut self,
        _context: &GenerationWorkerContext,
        _request: &WaveformGenerationRequest,
    ) -> Result<GeneratedArtifact, artifact_store::ArtifactStoreError> {
        unreachable!("cancel-aware proxy test should not call waveform")
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
        .query_row(
            "SELECT COUNT(*) FROM artifact WHERE status = 'ready'",
            [],
            |row| row.get(0),
        )
        .expect("artifact count should read")
}

fn persisted_job(
    bundle_path: &std::path::Path,
    job_id: &str,
) -> artifact_store::jobs::ArtifactGenerationJob {
    let store = open_artifact_store(bundle_path).expect("store should open");
    list_generation_jobs(&store)
        .expect("jobs should list")
        .into_iter()
        .find(|job| job.job_id == job_id)
        .expect("job should persist")
}

fn assert_dirty_ids(rows: &[artifact_store::invalidation::DirtyArtifactRow], expected: &[&str]) {
    let mut actual = rows
        .iter()
        .map(|row| row.artifact_id.as_str())
        .collect::<Vec<_>>();
    actual.sort_unstable();
    assert_eq!(actual, expected);
}
