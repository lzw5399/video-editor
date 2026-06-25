#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RealtimePreviewPipelineSet {
    pub canvas_pipeline_label: &'static str,
    pub textured_quad_pipeline_label: &'static str,
    pub production_effect_uniform_label: &'static str,
    pub production_mask_blend_uniform_label: &'static str,
    pub production_blend_pipeline_label: &'static str,
}

impl RealtimePreviewPipelineSet {
    pub const fn phase11_subset() -> Self {
        Self {
            canvas_pipeline_label: "realtime-preview-canvas",
            textured_quad_pipeline_label: "realtime-preview-textured-quad",
            production_effect_uniform_label: "phase19-production-effect-uniforms",
            production_mask_blend_uniform_label: "phase19-mask-blend-uniforms",
            production_blend_pipeline_label: "phase19-mask-blend-wgpu-pipelines",
        }
    }
}
