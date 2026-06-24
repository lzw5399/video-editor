//! Offline Kaipai adapter boundary.

pub mod error;
pub mod formula_bundle;
pub mod mapper;

pub use error::AdapterKaipaiError;
pub use formula_bundle::{
    DirectMaterialRef, FormulaBundleKind, FormulaBundleSchemaVersion, FormulaProvenance,
    FormulaResourceKind, FormulaResourceRef, FormulaSourceMedia, KaipaiFormulaBundle,
    RecognizerResult, RecognizerWord, SafeAreaEvidence, SafeAreaStatus,
};
pub use mapper::{map_kaipai_bundle_to_import_plan, KaipaiImportOptions, KaipaiMappedFixture};
