//! Provider-neutral template import contracts.

pub mod adaptation_report;
pub mod resource_localizer;

pub use adaptation_report::{
    AdaptationCategory, AdaptationReport, AdaptationReportItem, AdaptationReportSchemaVersion,
    AdaptationReportSummary, AdaptationSeverity, AdaptationStatus, AdaptationTargetKind,
    AdaptationTargetRef, ExternalProvenanceRef,
};
pub use resource_localizer::{
    LocalizedResource, LocalizedResourceIndexKind, LocalizedResourceIndexRef,
    LocalizedResourceManifest, LocalizedResourceStatus, ResourceLocalizationError,
    ResourceLocalizationMode, ResourceLocalizationRequest, ResourceLocalizationResult,
    TemplateResourceKind, TemplateResourceRef, localize_template_resources,
};
