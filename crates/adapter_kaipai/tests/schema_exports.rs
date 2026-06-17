use std::{
    env, fs,
    path::{Path, PathBuf},
};

use adapter_kaipai::{
    CompatibilityReport, CompatibilityReportSchemaVersion, DirectMaterialRef, FormulaBundleKind,
    FormulaBundleSchemaVersion, FormulaProvenance, FormulaResourceRef, FormulaSourceMedia,
    KaipaiFormulaBundle, RecognizerResult, ResourceKind, SafeAreaEvidence, SafeAreaStatus,
};
use schemars::{Schema, schema_for};
use serde_json::json;
use ts_rs::{Config, TS};

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("adapter_kaipai should live under crates/")
        .to_path_buf()
}

#[test]
fn schema_exports_generated_formula_bundle_contracts_from_rust() {
    let root = project_root();
    let schema_path = root.join("schemas/kaipai-formula-bundle.schema.json");
    let generated_ts_path = root.join("apps/desktop-electron/src/generated/KaipaiFormulaBundle.ts");

    let schema_json = formula_bundle_schema_json();
    assert_formula_bundle_schema_requires_evidence_fields(&schema_json);
    assert_or_update_contract_file(&schema_path, &format!("{schema_json}\n"));

    let formula_bundle_ts = formula_bundle_ts_contract();
    assert!(
        formula_bundle_ts.contains("word_list"),
        "generated TypeScript should preserve provider recognizer `word_list` evidence"
    );
    assert!(
        formula_bundle_ts.contains("safeArea"),
        "generated TypeScript should expose safeArea as adapter evidence"
    );
    assert_or_update_contract_file(generated_ts_path, &formula_bundle_ts);
}

#[test]
fn schema_exports_generated_compatibility_report_contract_from_rust() {
    let root = project_root();
    let schema_path = root.join("schemas/compatibility-report.schema.json");

    let schema_json = compatibility_report_schema_json();
    assert_compatibility_report_schema_requires_diagnostic_fields(&schema_json);
    assert_or_update_contract_file(&schema_path, &format!("{schema_json}\n"));
}

fn formula_bundle_schema_json() -> String {
    let mut schema = schema_for!(KaipaiFormulaBundle);
    constrain_current_formula_bundle_schema_version(&mut schema);
    serde_json::to_string_pretty(&schema).expect("formula bundle schema should serialize")
}

fn compatibility_report_schema_json() -> String {
    let mut schema = schema_for!(CompatibilityReport);
    constrain_current_compatibility_report_schema_version(&mut schema);
    serde_json::to_string_pretty(&schema).expect("compatibility report schema should serialize")
}

fn constrain_current_formula_bundle_schema_version(schema: &mut Schema) {
    let defs = schema
        .ensure_object()
        .get_mut("$defs")
        .and_then(serde_json::Value::as_object_mut)
        .expect("generated formula bundle schema should contain $defs");
    defs.insert(
        "FormulaBundleSchemaVersion".to_owned(),
        json!({
            "type": "integer",
            "const": FormulaBundleSchemaVersion::CURRENT_VALUE
        }),
    );
}

fn constrain_current_compatibility_report_schema_version(schema: &mut Schema) {
    let defs = schema
        .ensure_object()
        .get_mut("$defs")
        .and_then(serde_json::Value::as_object_mut)
        .expect("generated compatibility report schema should contain $defs");
    defs.insert(
        "CompatibilityReportSchemaVersion".to_owned(),
        json!({
            "type": "integer",
            "const": CompatibilityReportSchemaVersion::CURRENT_VALUE
        }),
    );
}

fn assert_formula_bundle_schema_requires_evidence_fields(schema_json: &str) {
    let schema_value: serde_json::Value =
        serde_json::from_str(schema_json).expect("formula bundle schema should parse");
    let required = schema_value["required"]
        .as_array()
        .expect("formula bundle schema should list required top-level fields");
    for field in [
        "schemaVersion",
        "kind",
        "provenance",
        "sourceMedia",
        "recognizerResult",
        "safeArea",
        "directMaterials",
        "formula",
        "resources",
    ] {
        assert!(
            required.iter().any(|value| value.as_str() == Some(field)),
            "formula bundle schema should require `{field}`"
        );
    }

    let recognizer_required = schema_value["$defs"]["RecognizerResult"]["required"]
        .as_array()
        .expect("recognizer result schema should list required fields");
    assert!(
        recognizer_required
            .iter()
            .any(|value| value.as_str() == Some("word_list")),
        "recognizer result schema should require provider `word_list` evidence"
    );
}

fn assert_compatibility_report_schema_requires_diagnostic_fields(schema_json: &str) {
    let schema_value: serde_json::Value =
        serde_json::from_str(schema_json).expect("compatibility report schema should parse");
    let required = schema_value["required"]
        .as_array()
        .expect("compatibility report schema should list required top-level fields");
    for field in [
        "schemaVersion",
        "sourceKind",
        "sourceId",
        "generatedAt",
        "summary",
        "items",
    ] {
        assert!(
            required.iter().any(|value| value.as_str() == Some(field)),
            "compatibility report schema should require `{field}`"
        );
    }
}

fn formula_bundle_ts_contract() -> String {
    ts_contract(&[
        export_decl::<serde_json::Value>(),
        export_decl::<FormulaBundleSchemaVersion>(),
        export_decl::<FormulaBundleKind>(),
        export_decl::<FormulaProvenance>(),
        export_decl::<FormulaSourceMedia>(),
        export_decl::<RecognizerResult>(),
        export_decl::<SafeAreaStatus>(),
        export_decl::<SafeAreaEvidence>(),
        export_decl::<ResourceKind>(),
        export_decl::<DirectMaterialRef>(),
        export_decl::<FormulaResourceRef>(),
        export_decl::<KaipaiFormulaBundle>(),
    ])
}

fn export_decl<T>() -> String
where
    T: TS + 'static,
{
    format!("export {}\n", T::decl(&ts_config()))
}

fn ts_config() -> Config {
    Config::new().with_large_int("number")
}

fn ts_contract(declarations: &[String]) -> String {
    let mut ts = String::from(
        "// This file was generated by Rust ts-rs declarations. Do not edit this file manually.\n\n",
    );
    for declaration in declarations {
        ts.push_str(declaration);
    }
    ts
}

fn assert_or_update_contract_file(path: impl AsRef<Path>, expected: &str) {
    let path = path.as_ref();

    if env::var_os("VE_UPDATE_GENERATED_CONTRACTS").as_deref() == Some(std::ffi::OsStr::new("1")) {
        fs::create_dir_all(path.parent().expect("contract path should have parent"))
            .expect("contract directory should be created");
        fs::write(path, expected).expect("contract artifact should be written");
        return;
    }

    let actual = fs::read_to_string(path).unwrap_or_else(|error| {
        panic!(
            "committed contract artifact should be readable at {}: {error}",
            path.display()
        )
    });
    assert_eq!(
        actual,
        expected,
        "generated contract artifact is stale: {}. Run with VE_UPDATE_GENERATED_CONTRACTS=1 to refresh.",
        path.display()
    );
}
