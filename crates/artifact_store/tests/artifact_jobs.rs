use artifact_store::jobs::{
    ArtifactGenerationRequest, ArtifactKind, GenerationChunkStatus, GenerationJobStatus,
    GenerationProgress, cancel_generation_job, complete_generation_chunk, create_generation_job,
    fail_generation_chunk, job_status_summary, list_generation_jobs, next_pending_chunk,
    resume_generation_job, start_generation_chunk,
};
use artifact_store::schema::open_artifact_store;
use rusqlite::OptionalExtension;
use serde_json::json;

#[test]
fn artifact_jobs_create_rows_for_all_supported_artifact_kinds() {
    let sandbox = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = sandbox.path().join("draft.veproj");
    let mut store = open_artifact_store(&bundle_path).expect("store should open");

    for (index, kind) in [
        ArtifactKind::Proxy,
        ArtifactKind::Thumbnail,
        ArtifactKind::Waveform,
        ArtifactKind::GraphSnapshot,
        ArtifactKind::PreviewFrame,
        ArtifactKind::PreviewSegment,
        ArtifactKind::FfmpegScript,
        ArtifactKind::SyncManifest,
    ]
    .into_iter()
    .enumerate()
    {
        let request = job_request(
            &format!("job-{index}"),
            &format!("artifact-{index}"),
            kind,
            vec![
                GenerationProgress::new(Some(index as u64 * 1_000_000), Some(1_000_000), Some(0)),
                GenerationProgress::new(
                    Some(index as u64 * 1_000_000 + 1_000_000),
                    Some(1_000_000),
                    Some(0),
                ),
            ],
        );

        let job = create_generation_job(&mut store, request).expect("job should be created");

        assert_eq!(job.status, GenerationJobStatus::Waiting);
        assert_eq!(job.kind, kind);
        assert_eq!(job.chunks.len(), 2);
        assert!(job.chunks.iter().all(|chunk| {
            chunk.status == GenerationChunkStatus::Waiting
                && chunk.target_duration_us == Some(1_000_000)
        }));
    }

    let reopened = open_artifact_store(&bundle_path).expect("store should reopen");
    let jobs = list_generation_jobs(&reopened).expect("jobs should list from sqlite");
    assert_eq!(jobs.len(), 8);
    assert_eq!(
        jobs.iter()
            .map(|job| job.kind)
            .collect::<Vec<ArtifactKind>>(),
        vec![
            ArtifactKind::Proxy,
            ArtifactKind::Thumbnail,
            ArtifactKind::Waveform,
            ArtifactKind::GraphSnapshot,
            ArtifactKind::PreviewFrame,
            ArtifactKind::PreviewSegment,
            ArtifactKind::FfmpegScript,
            ArtifactKind::SyncManifest,
        ]
    );
}

#[test]
fn artifact_jobs_persist_chunk_transitions_and_progress_transactionally() {
    let sandbox = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = sandbox.path().join("draft.veproj");
    let mut store = open_artifact_store(&bundle_path).expect("store should open");
    create_generation_job(
        &mut store,
        job_request(
            "job-progress",
            "artifact-progress",
            ArtifactKind::Proxy,
            vec![
                GenerationProgress::new(Some(0), Some(1_000_000), Some(0)),
                GenerationProgress::new(Some(1_000_000), Some(1_000_000), Some(0)),
            ],
        ),
    )
    .expect("job should be created");

    let first = next_pending_chunk(&store, "job-progress")
        .expect("next chunk should query")
        .expect("first chunk should exist");
    assert_eq!(first.chunk_index, 0);

    let running =
        start_generation_chunk(&mut store, "job-progress", 0).expect("chunk should start");
    assert_eq!(running.status, GenerationChunkStatus::Running);
    assert_eq!(running.progress_per_mille, Some(0));

    let completed = complete_generation_chunk(
        &mut store,
        "job-progress",
        0,
        Some("blobs/blake3/v1/aa/proxy.bin"),
        Some("blake3:v1:proxy"),
        128,
    )
    .expect("chunk should complete");
    assert_eq!(completed.status, GenerationChunkStatus::Completed);

    let failed = fail_generation_chunk(&mut store, "job-progress", 1, "decode failed")
        .expect("chunk should fail");
    assert_eq!(failed.status, GenerationChunkStatus::Failed);

    let reopened = open_artifact_store(&bundle_path).expect("store should reopen");
    let persisted = list_generation_jobs(&reopened)
        .expect("jobs should list")
        .into_iter()
        .find(|job| job.job_id == "job-progress")
        .expect("job should exist");
    assert_eq!(persisted.status, GenerationJobStatus::Failed);
    assert_eq!(persisted.progress.progress_per_mille, Some(500));
    assert_eq!(persisted.chunks[0].status, GenerationChunkStatus::Completed);
    assert_eq!(persisted.chunks[1].status, GenerationChunkStatus::Failed);
}

#[test]
fn artifact_jobs_resume_only_incomplete_and_preserve_terminal_states() {
    let sandbox = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = sandbox.path().join("draft.veproj");
    let mut store = open_artifact_store(&bundle_path).expect("store should open");

    create_generation_job(
        &mut store,
        job_request(
            "job-resume",
            "artifact-resume",
            ArtifactKind::Thumbnail,
            vec![
                GenerationProgress::new(Some(0), Some(500_000), Some(0)),
                GenerationProgress::new(Some(500_000), Some(500_000), Some(0)),
            ],
        ),
    )
    .expect("job should be created");
    start_generation_chunk(&mut store, "job-resume", 0).expect("chunk should start");
    complete_generation_chunk(
        &mut store,
        "job-resume",
        0,
        Some("blobs/blake3/v1/bb/thumb.bin"),
        Some("blake3:v1:thumb"),
        64,
    )
    .expect("chunk should complete");

    let reopened = open_artifact_store(&bundle_path).expect("store should reopen");
    let resume = resume_generation_job(&reopened, "job-resume")
        .expect("resume should query")
        .expect("incomplete job should be resumable");
    assert_eq!(resume.pending_chunks.len(), 1);
    assert_eq!(resume.pending_chunks[0].chunk_index, 1);

    let mut store = open_artifact_store(&bundle_path).expect("store should reopen mutable");
    start_generation_chunk(&mut store, "job-resume", 1).expect("second chunk should start");
    complete_generation_chunk(
        &mut store,
        "job-resume",
        1,
        Some("blobs/blake3/v1/cc/thumb2.bin"),
        Some("blake3:v1:thumb2"),
        64,
    )
    .expect("second chunk should complete");
    assert!(
        resume_generation_job(&store, "job-resume")
            .expect("resume should query")
            .is_none(),
        "completed jobs should not return a resume plan"
    );

    let late = fail_generation_chunk(&mut store, "job-resume", 0, "late worker failure")
        .expect_err("terminal completed job should reject late chunk failure");
    assert!(
        late.to_string().contains("terminal"),
        "unexpected terminal guard error: {late}"
    );

    let status: String = store
        .connection()
        .query_row(
            "SELECT status FROM generation_job WHERE job_id = ?1",
            ["job-resume"],
            |row| row.get(0),
        )
        .expect("status should read");
    assert_eq!(status, "completed");
}

#[test]
fn artifact_jobs_cancel_and_status_summary_are_durable_and_ui_safe() {
    let sandbox = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = sandbox.path().join("draft.veproj");
    let mut store = open_artifact_store(&bundle_path).expect("store should open");
    create_generation_job(
        &mut store,
        job_request(
            "job-cancel",
            "artifact-cancel",
            ArtifactKind::Waveform,
            vec![GenerationProgress::new(Some(0), Some(2_000_000), Some(0))],
        ),
    )
    .expect("job should be created");
    start_generation_chunk(&mut store, "job-cancel", 0).expect("chunk should start");

    let cancelled =
        cancel_generation_job(&mut store, "job-cancel").expect("cancel should persist request");
    assert_eq!(cancelled.status, GenerationJobStatus::CancelRequested);

    let reopened = open_artifact_store(&bundle_path).expect("store should reopen");
    let summary = job_status_summary(&reopened, "job-cancel")
        .expect("summary should query")
        .expect("summary should exist");
    assert_eq!(summary.job_id, "job-cancel");
    assert_eq!(summary.kind, ArtifactKind::Waveform);
    assert!(summary.can_cancel);
    assert!(!summary.can_resume);
    assert_eq!(summary.progress_per_mille, Some(0));

    let serialized = serde_json::to_string(&summary).expect("summary should serialize");
    for forbidden in [
        "artifact-store.sqlite",
        ".veproj/derived",
        "blake3:v1:",
        "graphNode",
        "dirtyRange",
        "filter_complex",
        "priority",
    ] {
        assert!(
            !serialized.contains(forbidden),
            "summary leaked forbidden internal string {forbidden}: {serialized}"
        );
    }
}

fn job_request(
    job_id: &str,
    artifact_id: &str,
    kind: ArtifactKind,
    chunks: Vec<GenerationProgress>,
) -> ArtifactGenerationRequest {
    ArtifactGenerationRequest {
        job_id: job_id.to_owned(),
        artifact_id: Some(artifact_id.to_owned()),
        kind,
        stable_key: format!("material:material-001:{}", kind.as_str()),
        generation_parameters_json: json!({
            "materialId": "material-001",
            "sourceRef": "media/source.mp4",
            "internalProbe": "should stay inside sqlite"
        }),
        source_fingerprint: Some("source:v1".to_owned()),
        runtime_capability_fingerprint: Some("runtime:v1".to_owned()),
        output_profile_fingerprint: Some("output:v1".to_owned()),
        graph_fingerprint: None,
        chunks,
    }
}

#[allow(dead_code)]
fn optional_status_for_job(
    store: &artifact_store::schema::ArtifactStore,
    job_id: &str,
) -> Option<String> {
    store
        .connection()
        .query_row(
            "SELECT status FROM generation_job WHERE job_id = ?1",
            [job_id],
            |row| row.get(0),
        )
        .optional()
        .expect("optional status should query")
}
