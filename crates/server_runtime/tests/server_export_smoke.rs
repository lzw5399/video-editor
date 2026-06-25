#[test]
fn server_export_smoke_surface_is_owned_by_server_runtime() {
    assert_eq!(server_runtime::contract_version(), "0.1.0");
}
