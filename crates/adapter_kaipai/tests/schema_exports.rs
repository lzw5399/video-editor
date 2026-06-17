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
use serde_json::{Value, json};
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

#[test]
fn schema_exports_formula_bundle_rejects_values_rust_rejects() {
    let schema_json: Value = serde_json::from_str(&formula_bundle_schema_json())
        .expect("formula bundle schema should parse");
    let validator =
        jsonschema::validator_for(&schema_json).expect("formula bundle schema should compile");
    let base = valid_formula_bundle_payload();

    for (case_name, invalid) in [
        (
            "empty template id",
            patch(&base, |value| value["provenance"]["templateId"] = json!("")),
        ),
        (
            "empty recipe id",
            patch(&base, |value| value["provenance"]["recipeId"] = json!("")),
        ),
        (
            "empty source media uri",
            patch(&base, |value| value["sourceMedia"]["uri"] = json!("")),
        ),
        (
            "zero source width",
            patch(&base, |value| value["sourceMedia"]["width"] = json!(0)),
        ),
        (
            "zero source height",
            patch(&base, |value| value["sourceMedia"]["height"] = json!(0)),
        ),
        (
            "zero source duration",
            patch(&base, |value| value["sourceMedia"]["durationMs"] = json!(0)),
        ),
        (
            "empty safe area value",
            patch(&base, |value| value["safeArea"]["value"] = json!("")),
        ),
        (
            "empty safe area source",
            patch(&base, |value| value["safeArea"]["source"] = json!("")),
        ),
        (
            "empty direct material id",
            patch(&base, |value| {
                value["directMaterials"] = json!([
                    {
                        "materialId": "",
                        "uri": "media/source.mp4",
                        "kind": "video",
                        "displayName": "source.mp4"
                    }
                ]);
            }),
        ),
        (
            "empty resource id",
            patch(&base, |value| {
                value["resources"] = json!([
                    {
                        "resourceId": "",
                        "kind": "font",
                        "uri": "resources/fonts/redacted.ttf"
                    }
                ]);
            }),
        ),
    ] {
        assert!(
            validator.validate(&invalid).is_err(),
            "generated schema should reject {case_name}"
        );
    }
}

fn formula_bundle_schema_json() -> String {
    let mut schema = schema_for!(KaipaiFormulaBundle);
    constrain_current_formula_bundle_schema_version(&mut schema);
    constrain_formula_bundle_value_contract(&mut schema);
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

fn constrain_formula_bundle_value_contract(schema: &mut Schema) {
    let defs = schema
        .ensure_object()
        .get_mut("$defs")
        .and_then(serde_json::Value::as_object_mut)
        .expect("generated formula bundle schema should contain $defs");

    for (def_name, property) in [
        ("FormulaProvenance", "templateId"),
        ("FormulaProvenance", "recipeId"),
        ("FormulaSourceMedia", "uri"),
        ("SafeAreaEvidence", "value"),
        ("SafeAreaEvidence", "source"),
        ("DirectMaterialRef", "materialId"),
        ("DirectMaterialRef", "uri"),
        ("DirectMaterialRef", "displayName"),
        ("FormulaResourceRef", "resourceId"),
        ("FormulaResourceRef", "uri"),
    ] {
        property_schema_mut(defs, def_name, property)
            .as_object_mut()
            .expect("string property schema should be an object")
            .insert("minLength".to_owned(), json!(1));
    }

    for property in ["width", "height", "durationMs"] {
        property_schema_mut(defs, "FormulaSourceMedia", property)
            .as_object_mut()
            .expect("numeric property schema should be an object")
            .insert("minimum".to_owned(), json!(1));
    }
}

fn property_schema_mut<'a>(
    defs: &'a mut serde_json::Map<String, Value>,
    def_name: &str,
    property: &str,
) -> &'a mut Value {
    defs.get_mut(def_name)
        .and_then(Value::as_object_mut)
        .and_then(|definition| definition.get_mut("properties"))
        .and_then(Value::as_object_mut)
        .and_then(|properties| properties.get_mut(property))
        .unwrap_or_else(|| panic!("generated schema should expose {def_name}.{property}"))
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
    assert_eq!(
        schema_value["$defs"]["FormulaSourceMedia"]["properties"]["width"]["minimum"],
        json!(1),
        "formula bundle schema should reject zero source media width"
    );
    assert_eq!(
        schema_value["$defs"]["FormulaSourceMedia"]["properties"]["height"]["minimum"],
        json!(1),
        "formula bundle schema should reject zero source media height"
    );
    assert_eq!(
        schema_value["$defs"]["FormulaSourceMedia"]["properties"]["durationMs"]["minimum"],
        json!(1),
        "formula bundle schema should reject zero source media duration"
    );
    assert_eq!(
        schema_value["$defs"]["FormulaProvenance"]["properties"]["templateId"]["minLength"],
        json!(1),
        "formula bundle schema should reject empty template id"
    );
    assert_eq!(
        schema_value["$defs"]["SafeAreaEvidence"]["properties"]["value"]["minLength"],
        json!(1),
        "formula bundle schema should reject empty safe area evidence"
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

fn valid_formula_bundle_payload() -> Value {
    json!({
        "schemaVersion": FormulaBundleSchemaVersion::CURRENT_VALUE,
        "kind": "kaipaiSmartEditFormulaBundle",
        "provenance": {
            "templateId": "tpl-redacted-schema",
            "recipeId": "recipe-redacted-schema"
        },
        "sourceMedia": {
            "uri": "media/source.mp4",
            "width": 1,
            "height": 1,
            "durationMs": 1
        },
        "recognizerResult": {
            "word_list": []
        },
        "safeArea": {
            "value": "0,0,1,1",
            "status": "detected",
            "source": "redactedLocalRecognizer"
        },
        "directMaterials": [],
        "formula": {},
        "resources": []
    })
}

fn patch(base: &Value, update: impl FnOnce(&mut Value)) -> Value {
    let mut value = base.clone();
    update(&mut value);
    value
}
