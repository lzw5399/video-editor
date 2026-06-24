use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
};

use adapter_kaipai::{
    DirectMaterialRef, FormulaBundleKind, FormulaBundleSchemaVersion, FormulaProvenance,
    FormulaResourceKind, FormulaResourceRef, FormulaSourceMedia, KaipaiFormulaBundle,
    RecognizerResult, RecognizerWord, SafeAreaEvidence, SafeAreaStatus,
};
use schemars::{Schema, schema_for};
use serde_json::{Value, json};
use ts_rs::TS;

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

    let schema_json = formula_bundle_schema_json();
    assert_formula_bundle_schema_requires_adapter_evidence_fields(&schema_json);
    assert_formula_bundle_schema_rejects_unknown_top_level_fields(&schema_json);
    assert_formula_bundle_schema_accepts_positive_fixtures(&schema_json);
    assert_formula_bundle_schema_rejects_schema_level_negative_fixtures(&schema_json);
    assert_or_update_contract_file(&schema_path, &format!("{schema_json}\n"));
}

fn formula_bundle_schema_json() -> String {
    let mut schema = schema_for!(KaipaiFormulaBundle);
    constrain_current_formula_bundle_schema_version(&mut schema);
    serde_json::to_string_pretty(&schema).expect("formula bundle schema should serialize")
}

fn constrain_current_formula_bundle_schema_version(schema: &mut Schema) {
    let defs = schema
        .ensure_object()
        .get_mut("$defs")
        .and_then(Value::as_object_mut)
        .expect("generated formula bundle schema should contain $defs");
    defs.insert(
        "FormulaBundleSchemaVersion".to_owned(),
        json!({
            "type": "integer",
            "const": FormulaBundleSchemaVersion::CURRENT_VALUE
        }),
    );
}

fn assert_formula_bundle_schema_requires_adapter_evidence_fields(schema_json: &str) {
    let schema_value: Value =
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

    assert_required_fields(
        &schema_value,
        "FormulaProvenance",
        &["provider", "templateId", "recipeId"],
    );
    assert_required_fields(
        &schema_value,
        "FormulaSourceMedia",
        &["resourceId", "kind", "uri"],
    );
    assert_required_fields(&schema_value, "RecognizerResult", &["wordList"]);
    assert_required_fields(
        &schema_value,
        "SafeAreaEvidence",
        &["status", "source", "value"],
    );
    assert_required_fields(
        &schema_value,
        "DirectMaterialRef",
        &["materialId", "kind", "uri", "displayName"],
    );
    assert_required_fields(
        &schema_value,
        "FormulaResourceRef",
        &["resourceId", "kind", "uri", "displayName"],
    );

    let kind_values = enum_values(&schema_value, "FormulaBundleKind");
    assert_eq!(kind_values, BTreeSet::from(["formulaBundle".to_owned()]));
}

fn assert_formula_bundle_schema_rejects_unknown_top_level_fields(schema_json: &str) {
    let schema_value: Value =
        serde_json::from_str(schema_json).expect("formula bundle schema should parse");
    let schema =
        jsonschema::validator_for(&schema_value).expect("formula bundle schema should compile");
    let mut fixture = read_formula_fixture("positive/sanitized-formula-bundle.json");

    schema
        .validate(&fixture)
        .expect("baseline positive fixture should validate");
    fixture
        .as_object_mut()
        .expect("fixture should be an object")
        .insert("unexpectedProviderField".to_owned(), json!("rejected"));
    assert!(
        schema.validate(&fixture).is_err(),
        "formula bundle schema should reject unknown top-level fields"
    );
}

fn assert_formula_bundle_schema_accepts_positive_fixtures(schema_json: &str) {
    let schema_value: Value =
        serde_json::from_str(schema_json).expect("formula bundle schema should parse");
    let schema =
        jsonschema::validator_for(&schema_value).expect("formula bundle schema should compile");

    for path in [
        "positive/sanitized-formula-bundle.json",
        "positive/sanitized-formula-with-direct-materials.json",
    ] {
        let value = read_formula_fixture(path);
        schema
            .validate(&value)
            .unwrap_or_else(|error| panic!("{path} should validate against schema: {error}"));
    }
}

fn assert_formula_bundle_schema_rejects_schema_level_negative_fixtures(schema_json: &str) {
    let schema_value: Value =
        serde_json::from_str(schema_json).expect("formula bundle schema should parse");
    let schema =
        jsonschema::validator_for(&schema_value).expect("formula bundle schema should compile");

    let value = read_formula_fixture("negative/unknown-top-level-field.json");
    assert!(
        schema.validate(&value).is_err(),
        "unknown top-level field fixture should fail schema validation"
    );
}

fn assert_required_fields(schema_value: &Value, def_name: &str, expected_fields: &[&str]) {
    let required = schema_value["$defs"][def_name]["required"]
        .as_array()
        .unwrap_or_else(|| panic!("{def_name} schema should list required fields"));
    for expected in expected_fields {
        assert!(
            required
                .iter()
                .any(|value| value.as_str() == Some(expected)),
            "{def_name} schema should require `{expected}`"
        );
    }
}

fn enum_values(schema_value: &Value, def_name: &str) -> BTreeSet<String> {
    schema_value["$defs"][def_name]["enum"]
        .as_array()
        .unwrap_or_else(|| panic!("{def_name} should expose string enum values"))
        .iter()
        .map(|value| {
            value
                .as_str()
                .unwrap_or_else(|| panic!("{def_name} enum value should be a string"))
                .to_owned()
        })
        .collect()
}

fn read_formula_fixture(path: &str) -> Value {
    let fixture_path = project_root().join("fixtures/kaipai").join(path);
    serde_json::from_slice(
        &fs::read(&fixture_path)
            .unwrap_or_else(|error| panic!("fixture should be readable: {path}: {error}")),
    )
    .unwrap_or_else(|error| panic!("fixture should parse as JSON: {path}: {error}"))
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

fn _assert_ts_exports_are_supported()
where
    FormulaBundleSchemaVersion: TS,
    FormulaBundleKind: TS,
    FormulaProvenance: TS,
    FormulaSourceMedia: TS,
    RecognizerResult: TS,
    RecognizerWord: TS,
    SafeAreaStatus: TS,
    SafeAreaEvidence: TS,
    FormulaResourceKind: TS,
    DirectMaterialRef: TS,
    FormulaResourceRef: TS,
    KaipaiFormulaBundle: TS,
{
}
