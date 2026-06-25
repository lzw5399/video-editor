use std::path::PathBuf;

use draft_model::ExportPreset;
use serde::{Deserialize, Serialize};
use task_runtime::{
    JobDomain, JobEnvelope, JobId, JobPriority, ResourceClass, TaskCancellationToken,
};

use crate::{ProjectSessionHandle, RuntimeError};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartProjectSessionExportRequest {
    pub project_session: ProjectSessionHandle,
    pub output_path: PathBuf,
    pub preset: ExportPreset,
    pub requested_at_us: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSessionExportJob {
    pub job_id: JobId,
    pub project_session: ProjectSessionHandle,
    pub output_path: PathBuf,
    pub preset: ExportPreset,
    pub scheduler_envelope: JobEnvelope,
}

#[derive(Debug, Default)]
pub struct ExportService {
    next_id: u64,
}

impl ExportService {
    pub fn start_project_session_export(
        &mut self,
        request: StartProjectSessionExportRequest,
    ) -> Result<ProjectSessionExportJob, RuntimeError> {
        let id_number = self.next_id.saturating_add(1);
        self.next_id = id_number;
        let job_id = JobId::new(format!("export-{id_number}"));
        let scheduler_envelope = JobEnvelope::new(
            job_id.clone(),
            JobDomain::Export,
            JobPriority::UserVisible,
            ResourceClass::FfmpegProcess,
            TaskCancellationToken::new(id_number),
            request.requested_at_us,
        );
        Ok(ProjectSessionExportJob {
            job_id,
            project_session: request.project_session,
            output_path: request.output_path,
            preset: request.preset,
            scheduler_envelope,
        })
    }
}
