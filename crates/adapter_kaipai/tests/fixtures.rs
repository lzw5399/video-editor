use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
};

use serde_json::Value;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("adapter_kaipai should live under crates/")
        .to_path_buf()
}

#[test]
fn formula_bundle_fixtures_are_explicitly_classified() {
    let fixture_dir = project_root().join("fixtures/kaipai");
    let actual = formula_fixture_paths(&fixture_dir);
    let expected = positive_formula_fixtures()
        .iter()
        .copied()
        .chain(negative_formula_fixtures().iter().copied())
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();

    assert_eq!(
        actual, expected,
        "every Kaipai formula fixture must be explicitly classified"
    );
}

#[test]
fn formula_bundle_fixtures_do_not_contain_credentials_or_remote_urls() {
    let fixture_dir = project_root().join("fixtures/kaipai");
    for fixture_path in formula_fixture_paths(&fixture_dir) {
        let value = read_formula_fixture(&fixture_dir, &fixture_path);
        scan_sanitized_fixture(&value, &fixture_path, "$root");
    }
}

fn positive_formula_fixtures() -> BTreeSet<&'static str> {
    BTreeSet::from([
        "positive/sanitized-formula-bundle.json",
        "positive/sanitized-formula-with-direct-materials.json",
    ])
}

fn negative_formula_fixtures() -> BTreeSet<&'static str> {
    BTreeSet::from([
        "negative/unknown-top-level-field.json",
        "negative/unsafe-formula-evidence.json",
    ])
}

fn formula_fixture_paths(fixture_dir: &Path) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    for formula_dir in ["positive", "negative"] {
        collect_formula_fixtures(fixture_dir, &fixture_dir.join(formula_dir), &mut paths);
    }
    paths
}

fn collect_formula_fixtures(root: &Path, dir: &Path, paths: &mut BTreeSet<String>) {
    if !dir.exists() {
        return;
    }
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

        paths.insert(
            path.strip_prefix(root)
                .expect("formula fixture should live under fixture root")
                .to_string_lossy()
                .replace('\\', "/"),
        );
    }
}

fn read_formula_fixture(fixture_dir: &Path, fixture_path: &str) -> Value {
    serde_json::from_slice(
        &fs::read(fixture_dir.join(fixture_path)).expect("formula fixture should be readable"),
    )
    .expect("formula fixture should parse as JSON")
}

fn scan_sanitized_fixture(value: &Value, fixture_path: &str, json_path: &str) {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                assert!(
                    !is_credential_like_key(key),
                    "{fixture_path} contains credential-like key at {json_path}.{key}"
                );
                scan_sanitized_fixture(child, fixture_path, &format!("{json_path}.{key}"));
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                scan_sanitized_fixture(child, fixture_path, &format!("{json_path}[{index}]"));
            }
        }
        Value::String(text) => {
            assert!(
                !looks_like_remote_url(text),
                "{fixture_path} contains remote URL at {json_path}"
            );
            assert!(
                !looks_like_signed_url(text),
                "{fixture_path} contains signed URL at {json_path}"
            );
        }
        _ => {}
    }
}

fn is_credential_like_key(key: &str) -> bool {
    let normalized = key
        .chars()
        .filter(|character| *character != '_' && *character != '-')
        .flat_map(char::to_lowercase)
        .collect::<String>();

    [
        "apikey",
        "authorization",
        "authorizationheader",
        "bearertoken",
        "cookie",
        "password",
        "privatekey",
        "refreshtoken",
        "secret",
        "secretkey",
        "session",
        "sessionjson",
        "token",
        "accesstoken",
        "accountid",
    ]
    .iter()
    .any(|needle| normalized.contains(needle))
}

fn looks_like_remote_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

fn looks_like_signed_url(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    lower.contains("x-amz-signature")
        || lower.contains("x-oss-signature")
        || lower.contains("signature=")
        || lower.contains("expires=")
}
