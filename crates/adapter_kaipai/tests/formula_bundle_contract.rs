use std::{env, fs, path::PathBuf};

use adapter_kaipai::KaipaiFormulaBundle;
use serde_json::{Value, json};

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("adapter_kaipai should live under crates/")
        .to_path_buf()
}

fn fixture_path(path: &str) -> PathBuf {
    project_root().join("fixtures/kaipai").join(path)
}

fn read_fixture(path: &str) -> String {
    fs::read_to_string(fixture_path(path)).expect("formula fixture should be readable")
}

fn read_fixture_value(path: &str) -> Value {
    serde_json::from_str(&read_fixture(path)).expect("formula fixture should parse as JSON")
}

#[test]
fn formula_bundle_positive_sanitized_fixtures_parse_and_validate() {
    for path in [
        "positive/sanitized-formula-bundle.json",
        "positive/sanitized-formula-with-direct-materials.json",
    ] {
        let bundle = KaipaiFormulaBundle::from_json_str(&read_fixture(path))
            .unwrap_or_else(|error| panic!("{path} should parse: {error}"));

        assert_eq!(bundle.schema_version.get(), 1, "{path}");
        assert_eq!(bundle.kind.as_str(), "formulaBundle", "{path}");
        assert_eq!(bundle.provenance.provider, "kaipai", "{path}");
        assert!(!bundle.provenance.template_id.is_empty(), "{path}");
        assert!(!bundle.provenance.recipe_id.is_empty(), "{path}");
        assert!(!bundle.source_media.uri.is_empty(), "{path}");
        assert!(!bundle.recognizer_result.word_list.is_empty(), "{path}");
        assert!(!bundle.safe_area.value.is_empty(), "{path}");
        assert!(bundle.formula.is_object(), "{path}");
        assert!(!bundle.resources.is_empty(), "{path}");

        bundle
            .validate()
            .unwrap_or_else(|error| panic!("{path} should validate: {error}"));
    }
}

#[test]
fn formula_bundle_rejects_unknown_top_level_fields() {
    let error = KaipaiFormulaBundle::from_json_str(&read_fixture(
        "negative/unknown-top-level-field.json",
    ))
    .expect_err("unknown top-level provider fields must be rejected");

    assert!(
        error.to_string().contains("unknown field `unexpectedProviderField`"),
        "unexpected error: {error}"
    );
}

#[test]
fn formula_bundle_rejects_unsafe_provider_evidence() {
    let error = KaipaiFormulaBundle::from_json_str(&read_fixture(
        "negative/unsafe-formula-evidence.json",
    ))
    .expect_err("unsafe formula evidence must be rejected");

    assert!(
        error
            .to_string()
            .contains("unsafe Kaipai formula evidence at `formula.sourceUrl`"),
        "unexpected error: {error}"
    );
}

#[test]
fn formula_bundle_rejects_in_memory_credentials_and_remote_refs() {
    let base = read_fixture_value("positive/sanitized-formula-bundle.json");

    for (case_name, payload, expected_path) in [
        (
            "credential-like formula key",
            patch(&base, |value| {
                value["formula"]["access_token"] = json!("redacted");
            }),
            "formula.access_token",
        ),
        (
            "signed formula URL",
            patch(&base, |value| {
                value["formula"]["objectRef"] =
                    json!("resources/source/redacted-main.mp4?X-Amz-Signature=redacted");
            }),
            "formula.objectRef",
        ),
        (
            "remote source media URI",
            patch(&base, |value| {
                value["sourceMedia"]["uri"] = json!("https://provider.invalid/source.mp4");
            }),
            "sourceMedia.uri",
        ),
        (
            "remote direct material URI",
            patch(&base, |value| {
                value["directMaterials"][0]["uri"] =
                    json!("https://provider.invalid/material.mp4");
            }),
            "directMaterials[0].uri",
        ),
        (
            "remote resource URI",
            patch(&base, |value| {
                value["resources"][0]["uri"] = json!("https://provider.invalid/font.ttf");
            }),
            "resources[0].uri",
        ),
    ] {
        let error = KaipaiFormulaBundle::from_json_value(payload)
            .expect_err("unsafe payload should fail validation");
        assert!(
            error.to_string().contains(expected_path),
            "{case_name}: unexpected error: {error}"
        );
    }
}

fn patch(base: &Value, update: impl FnOnce(&mut Value)) -> Value {
    let mut value = base.clone();
    update(&mut value);
    value
}
