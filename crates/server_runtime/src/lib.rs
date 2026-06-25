//! Compile-safe server runtime crate shell over `editor_runtime`.
//!
//! Plan 18-05 fills the Electron-free export runner. This crate exists now so
//! future server code compiles against the shared runtime API instead of
//! reaching into desktop Node bindings.

pub use editor_runtime::{
    EDITOR_RUNTIME_CONTRACT_VERSION, ExportService, ProjectSessionService, RuntimeSessionRegistry,
};

pub fn contract_version() -> &'static str {
    EDITOR_RUNTIME_CONTRACT_VERSION
}

#[cfg(test)]
mod tests {
    use std::fs;

    use draft_model::Draft;
    use project_store::{StdPlatformFileSystem, create_project_bundle};

    use super::*;

    #[test]
    fn opens_project_bundle_with_runtime_owned_session_handle() {
        let temp_dir = tempfile::tempdir().expect("tempdir should be created");
        let bundle_path = temp_dir.path().join("server-open.veproj");
        let draft = Draft::new("server-open-draft", "Server Open Draft");
        create_project_bundle(&StdPlatformFileSystem, &bundle_path, &draft)
            .expect("bundle should be created");

        let runtime = ServerRuntime::new().expect("server runtime should start");
        let opened = open_project(&runtime, &bundle_path).expect("server should open bundle");

        assert_eq!(opened.draft_id.as_str(), "server-open-draft");
        assert_eq!(opened.draft_name, "Server Open Draft");
        assert_eq!(opened.handle.owner_session(), runtime.session_id());
        assert!(opened.project_json_path.ends_with("project.json"));
    }

    #[test]
    fn crate_manifest_stays_on_shared_runtime_boundary() {
        let manifest = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"))
            .expect("manifest should be readable");

        assert!(manifest.contains("editor_runtime"));
        assert!(manifest.contains("media_runtime"));
        assert!(!manifest.contains("bindings_node"));
        assert!(!manifest.contains("napi"));
    }

    #[test]
    fn cli_entrypoint_routes_to_server_library_api() {
        let cli = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/main.rs"))
            .expect("server CLI should exist");

        assert!(cli.contains("server_runtime"));
        assert!(cli.contains("open_project"));
        assert!(cli.contains("start_export"));
        assert!(cli.contains("get_export_status"));
        assert!(cli.contains("serde_json"));
    }
}
