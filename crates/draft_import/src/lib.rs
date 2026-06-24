//! Provider-neutral template import contracts.

pub mod adaptation_report;
pub mod import_plan;
pub mod resource_localizer;
pub mod validation;

pub use adaptation_report::{
    AdaptationCategory, AdaptationReport, AdaptationReportItem, AdaptationReportSchemaVersion,
    AdaptationReportSummary, AdaptationSeverity, AdaptationStatus, AdaptationTargetKind,
    AdaptationTargetRef, ExternalProvenanceRef,
};
pub use import_plan::{
    DraftImportApplicationInput, DraftImportApplicationResult, DraftImportPlan,
    DraftImportPlanSchemaVersion, ImportMaterialPlan, ImportTrackPlan,
};
pub use resource_localizer::{
    LocalizedResource, LocalizedResourceIndexKind, LocalizedResourceIndexRef,
    LocalizedResourceManifest, LocalizedResourceStatus, ResourceLocalizationError,
    ResourceLocalizationMode, ResourceLocalizationRequest, ResourceLocalizationResult,
    TemplateResourceKind, TemplateResourceRef, localize_template_resources,
};
pub use validation::{
    DraftImportPlanValidationError, apply_import_plan_to_draft, validate_import_plan,
};
