//! Provider-neutral template import contracts.

pub mod adaptation_report;

pub use adaptation_report::{
    AdaptationCategory, AdaptationReport, AdaptationReportItem, AdaptationReportSchemaVersion,
    AdaptationReportSummary, AdaptationSeverity, AdaptationStatus, AdaptationTargetKind,
    AdaptationTargetRef, ExternalProvenanceRef,
};
