use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
};

use adapter_kaipai::KaipaiFormulaBundle;
use serde_json::{Value, json};

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("adapter_kaipai should live under crates/")
        .to_path_buf()
}

#[test]
fn formula_bundle_fixtures_are_explicitly_classified() {
    let root = project_root();
    let fixture_dir = root.join("fixtures/kaipai");

    let actual = formula_fixture_paths(&fixture_dir);
    let expected = positive_formula_fixtures()
        .iter()
        .copied()
        .chain(negative_formula_fixtures().iter().map(|(path, _, _)| *path))
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();

    assert_eq!(
        actual, expected,
        "every Kaipai formula fixture must be explicitly classified"
    );
}

#[test]
fn formula_bundle_fixtures_positive_validate_through_model_and_schema() {
    let root = project_root();
    let fixture_dir = root.join("fixtures/kaipai");
    let schema = formula_bundle_schema_validator();

    for fixture_path in positive_formula_fixtures() {
        let value = read_formula_fixture(&fixture_dir, fixture_path);
        schema
            .validate(&value)
            .expect("positive formula fixture should validate against generated JSON Schema");
        let bundle = KaipaiFormulaBundle::from_json_value(value)
            .expect("positive formula fixture should validate through Rust contract");

        assert_required_formula_evidence(&bundle, fixture_path);
        assert_round_trips(&bundle, fixture_path);
    }
}

#[test]
fn formula_bundle_fixtures_negative_fail_exact_expected_errors() {
    let root = project_root();
    let fixture_dir = root.join("fixtures/kaipai");
    let schema = formula_bundle_schema_validator();

    for (fixture_path, expected_error, should_fail_schema) in negative_formula_fixtures() {
        let value = read_formula_fixture(&fixture_dir, fixture_path);
        let error = KaipaiFormulaBundle::from_json_value(value.clone())
            .expect_err("negative formula fixture should fail validation");

        assert_eq!(error.to_string(), expected_error, "{fixture_path}");
        if should_fail_schema {
            assert!(
                schema.validate(&value).is_err(),
                "negative formula fixture should fail generated schema validation: {fixture_path}"
            );
        }
    }
}

#[test]
fn formula_bundle_fixtures_reject_in_memory_unsafe_payloads() {
    let base = read_formula_fixture(
        &project_root().join("fixtures/kaipai"),
        "positive/sanitized-formula-bundle.json",
    );

    for (case_name, payload, expected_error) in [
        (
            "remote formula URL",
            patch(&base, |value| {
                value["formula"]["sourceUrl"] = json!("https://example.invalid/source.mp4");
            }),
            "unsafe Kaipai formula evidence at `formula.sourceUrl`: remote URLs are not allowed in formula evidence",
        ),
        (
            "signed formula URL",
            patch(&base, |value| {
                value["formula"]["objectRef"] =
                    json!("redacted-local-ref?X-Amz-Signature=redacted");
            }),
            "unsafe Kaipai formula evidence at `formula.objectRef`: signed URLs are not allowed in formula evidence",
        ),
        (
            "authorization key",
            patch(&base, |value| {
                value["formula"]["Authorization"] = json!("redacted");
            }),
            "unsafe Kaipai formula evidence at `formula.Authorization`: credential-like fields are not allowed in formula evidence",
        ),
        (
            "cookie key",
            patch(&base, |value| {
                value["formula"]["Cookie"] = json!("redacted");
            }),
            "unsafe Kaipai formula evidence at `formula.Cookie`: credential-like fields are not allowed in formula evidence",
        ),
        (
            "session json key",
            patch(&base, |value| {
                value["formula"]["sessionJson"] = json!({"value": "redacted"});
            }),
            "unsafe Kaipai formula evidence at `formula.sessionJson`: credential-like fields are not allowed in formula evidence",
        ),
        (
            "access token key",
            patch(&base, |value| {
                value["formula"]["access_token"] = json!("redacted");
            }),
            "unsafe Kaipai formula evidence at `formula.access_token`: credential-like fields are not allowed in formula evidence",
        ),
        (
            "api key",
            patch(&base, |value| {
                value["formula"]["apiKey"] = json!("redacted");
            }),
            "unsafe Kaipai formula evidence at `formula.apiKey`: credential-like fields are not allowed in formula evidence",
        ),
        (
            "password key",
            patch(&base, |value| {
                value["formula"]["password"] = json!("redacted");
            }),
            "unsafe Kaipai formula evidence at `formula.password`: credential-like fields are not allowed in formula evidence",
        ),
        (
            "secret key",
            patch(&base, |value| {
                value["formula"]["secretKey"] = json!("redacted");
            }),
            "unsafe Kaipai formula evidence at `formula.secretKey`: credential-like fields are not allowed in formula evidence",
        ),
        (
            "private key",
            patch(&base, |value| {
                value["formula"]["privateKey"] = json!("redacted");
            }),
            "unsafe Kaipai formula evidence at `formula.privateKey`: credential-like fields are not allowed in formula evidence",
        ),
        (
            "bearer token",
            patch(&base, |value| {
                value["formula"]["bearerToken"] = json!("redacted");
            }),
            "unsafe Kaipai formula evidence at `formula.bearerToken`: credential-like fields are not allowed in formula evidence",
        ),
        (
            "authorization header",
            patch(&base, |value| {
                value["formula"]["authorizationHeader"] = json!("redacted");
            }),
            "unsafe Kaipai formula evidence at `formula.authorizationHeader`: credential-like fields are not allowed in formula evidence",
        ),
        (
            "refresh token key",
            patch(&base, |value| {
                value["formula"]["refresh_token"] = json!("redacted");
            }),
            "unsafe Kaipai formula evidence at `formula.refresh_token`: credential-like fields are not allowed in formula evidence",
        ),
        (
            "account id key",
            patch(&base, |value| {
                value["formula"]["accountId"] = json!("redacted");
            }),
            "unsafe Kaipai formula evidence at `formula.accountId`: credential-like fields are not allowed in formula evidence",
        ),
        (
            "remote source media URI",
            patch(&base, |value| {
                value["sourceMedia"]["uri"] = json!("https://prod.kaipai.example/source.mp4");
            }),
            "unsafe Kaipai formula evidence at `sourceMedia.uri`: remote resource references are not allowed in sanitized formula bundles",
        ),
        (
            "remote direct material URI",
            patch(&base, |value| {
                value["directMaterials"][0]["uri"] =
                    json!("https://prod.kaipai.example/material.mp4");
            }),
            "unsafe Kaipai formula evidence at `directMaterials[0].uri`: remote resource references are not allowed in sanitized formula bundles",
        ),
        (
            "remote resource URI",
            patch(&base, |value| {
                value["resources"][0]["uri"] = json!("https://prod.kaipai.example/font.ttf");
            }),
            "unsafe Kaipai formula evidence at `resources[0].uri`: remote resource references are not allowed in sanitized formula bundles",
        ),
    ] {
        let error = match KaipaiFormulaBundle::from_json_value(payload) {
            Ok(_) => panic!("unsafe payload should fail validation: {case_name}"),
            Err(error) => error,
        };
        assert_eq!(error.to_string(), expected_error, "{case_name}");
    }
}

fn positive_formula_fixtures() -> BTreeSet<&'static str> {
    BTreeSet::from([
        "positive/resource-bundle-with-local-assets.json",
        "positive/sanitized-formula-bundle.json",
        "positive/sanitized-formula-with-direct-materials.json",
    ])
}

fn negative_formula_fixtures() -> Vec<(&'static str, &'static str, bool)> {
    vec![
        (
            "negative/missing-word-list.json",
            "invalid Kaipai formula bundle JSON: missing field `word_list`",
            true,
        ),
        (
            "negative/invalid-safe-area-status.json",
            "invalid Kaipai formula bundle JSON: unknown variant `providerSpecific`, expected one of `detected`, `provided`, `unavailable`",
            true,
        ),
        (
            "negative/unsafe-safe-area-source.json",
            "unsafe Kaipai formula evidence at `safeArea.source`: safe area source must be redacted local fixture evidence",
            false,
        ),
        (
            "negative/unknown-top-level-field.json",
            "invalid Kaipai formula bundle JSON: unknown field `unexpectedProviderField`, expected one of `schemaVersion`, `kind`, `provenance`, `sourceMedia`, `recognizerResult`, `safeArea`, `directMaterials`, `formula`, `resources`",
            true,
        ),
    ]
}

fn formula_fixture_paths(fixture_dir: &Path) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    if fixture_dir.exists() {
        for formula_dir in ["positive", "negative"] {
            let path = fixture_dir.join(formula_dir);
            if path.exists() {
                collect_formula_fixtures(fixture_dir, &path, &mut paths);
            }
        }
    }
    paths
}

fn collect_formula_fixtures(root: &Path, dir: &Path, paths: &mut BTreeSet<String>) {
    for entry in fs::read_dir(dir).expect("fixture directory should be readable") {
        let entry = entry.expect("fixture directory entry should be readable");
        let path = entry.path();

        if path.is_dir() {
            collect_formula_fixtures(root, &path, paths);
            continue;
        }

        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }

        let relative = path
            .strip_prefix(root)
            .expect("formula fixture should live under fixture root")
            .to_string_lossy()
            .replace('\\', "/");
        if resource_localizer_negative_fixture_paths().contains(relative.as_str()) {
            continue;
        }
        paths.insert(relative);
    }
}

fn resource_localizer_negative_fixture_paths() -> BTreeSet<&'static str> {
    BTreeSet::from([
        "negative/missing-resource.json",
        "negative/path-traversal-resource.json",
        "negative/sha256-mismatch.json",
    ])
}

fn read_formula_fixture(fixture_dir: &Path, fixture_path: &str) -> Value {
    serde_json::from_slice(
        &fs::read(fixture_dir.join(fixture_path)).expect("formula fixture should be readable"),
    )
    .expect("formula fixture should parse as JSON")
}

fn formula_bundle_schema_validator() -> jsonschema::Validator {
    let schema_path = project_root().join("schemas/kaipai-formula-bundle.schema.json");
    let schema_json: Value = serde_json::from_slice(
        &fs::read(&schema_path).expect("generated formula bundle schema should be readable"),
    )
    .expect("generated formula bundle schema should parse");
    jsonschema::validator_for(&schema_json).expect("generated formula bundle schema should compile")
}

fn assert_required_formula_evidence(bundle: &KaipaiFormulaBundle, fixture_path: &str) {
    assert!(
        bundle.formula.is_object(),
        "positive fixture should preserve raw formula evidence: {fixture_path}"
    );
    assert!(
        !bundle.provenance.template_id.is_empty(),
        "positive fixture should include template provenance: {fixture_path}"
    );
    assert!(
        !bundle.provenance.recipe_id.is_empty(),
        "positive fixture should include recipe provenance: {fixture_path}"
    );
    assert!(
        bundle.provenance.formula_task_id.is_some(),
        "positive fixture should include formula task provenance: {fixture_path}"
    );
    assert!(
        bundle.provenance.formula_request_id.is_some(),
        "positive fixture should include formula request provenance: {fixture_path}"
    );
    assert!(
        !bundle.source_media.uri.is_empty(),
        "positive fixture should include source media evidence: {fixture_path}"
    );
    assert!(
        !bundle.recognizer_result.word_list.is_empty(),
        "positive fixture should include recognizer word_list evidence: {fixture_path}"
    );
    assert!(
        !bundle.safe_area.value.is_empty(),
        "positive fixture should include safeArea evidence: {fixture_path}"
    );
    assert!(
        !bundle.direct_materials.is_empty(),
        "positive fixture should include direct material references: {fixture_path}"
    );
    assert!(
        !bundle.resources.is_empty(),
        "positive fixture should include resource references: {fixture_path}"
    );
}

fn assert_round_trips(bundle: &KaipaiFormulaBundle, fixture_path: &str) {
    let serialized = serde_json::to_value(bundle).expect("formula fixture should serialize");
    let round_tripped = KaipaiFormulaBundle::from_json_value(serialized)
        .expect("serialized formula fixture should deserialize");
    assert_eq!(
        round_tripped, *bundle,
        "formula fixture should preserve adapter evidence equality: {fixture_path}"
    );
}

fn patch(base: &Value, update: impl FnOnce(&mut Value)) -> Value {
    let mut value = base.clone();
    update(&mut value);
    value
}
