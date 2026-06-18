#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealtimePreviewPipelineSet {
    pub canvas_pipeline_label: &'static str,
    pub textured_quad_pipeline_label: &'static str,
}

impl RealtimePreviewPipelineSet {
    pub const fn phase11_subset() -> Self {
        Self {
            canvas_pipeline_label: "realtime-preview-canvas",
            textured_quad_pipeline_label: "realtime-preview-textured-quad",
        }
    }
}
