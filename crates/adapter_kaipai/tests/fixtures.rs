use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
};

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
        .chain(negative_formula_fixtures().iter().map(|(path, _)| *path))
        .map(str::to_owned)
        .collect::<BTreeSet<_>>();

    assert_eq!(
        actual, expected,
        "every Kaipai formula fixture must be explicitly classified"
    );
}

fn positive_formula_fixtures() -> BTreeSet<&'static str> {
    BTreeSet::from([
        "positive/sanitized-formula-bundle.json",
        "positive/sanitized-formula-with-direct-materials.json",
    ])
}

fn negative_formula_fixtures() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "negative/missing-word-list.json",
            "missing recognizerResult.word_list",
        ),
        (
            "negative/invalid-safe-area-status.json",
            "invalid safeArea.status",
        ),
        (
            "negative/unsafe-safe-area-source.json",
            "unsafe safeArea.source",
        ),
        (
            "negative/unknown-top-level-field.json",
            "unknown top-level field",
        ),
    ]
}

fn formula_fixture_paths(fixture_dir: &Path) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    if fixture_dir.exists() {
        collect_formula_fixtures(fixture_dir, fixture_dir, &mut paths);
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
        paths.insert(relative);
    }
}
