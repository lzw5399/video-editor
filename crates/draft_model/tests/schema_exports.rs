use std::path::{Path, PathBuf};

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("draft_model should live under crates/")
        .to_path_buf()
}

#[test]
fn schema_generated_contract_artifacts_are_committed() {
    let root = project_root();
    let required = [
        root.join("schemas/command.schema.json"),
        root.join("apps/desktop-electron/src/generated/CommandEnvelope.ts"),
        root.join("apps/desktop-electron/src/generated/CommandResultEnvelope.ts"),
    ];

    for path in required {
        assert!(
            path.is_file(),
            "generated contract artifact is missing: {}",
            path.display()
        );
    }
}
