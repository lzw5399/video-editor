use std::{fs, path::Path};

const GAP_INVENTORY_PATH: &str = "docs/compatibility/draft-v2-template-semantics-gaps.md";

#[test]
fn draft_v2_gap_inventory_documents_required_template_semantics() {
    let root = project_root();
    let document = read_gap_inventory(&root);

    for heading in REQUIRED_HEADINGS {
        assert!(
            document.contains(heading),
            "gap inventory must contain required heading `{heading}`"
        );
    }

    for phrase in REQUIRED_JIANYING_STYLE_TERMS {
        assert!(
            document.contains(phrase),
            "gap inventory must use Jianying-style term `{phrase}`"
        );
    }

    for phrase in REQUIRED_VERIFICATION_PHRASES {
        assert!(
            document.contains(phrase),
            "gap inventory must record verification phrase `{phrase}`"
        );
    }
}

#[test]
fn draft_v2_gap_inventory_blocks_mapper_preview_export_claims() {
    let root = project_root();
    let adapter_source = root.join("crates/adapter_kaipai/src");
    let mut violations = Vec::new();
    collect_claim_violations(&adapter_source, &mut violations);

    assert!(
        violations.is_empty(),
        "adapter source must not claim mapper preview/export support or native parity before Draft v2 gaps are resolved:\n{}",
        violations.join("\n")
    );
}

const REQUIRED_HEADINGS: &[&str] = &[
    "## Draft.canvas",
    "## Font material/resource references",
    "## Resource manifest",
    "## Canvas adjustment/transform",
    "## Sticker and text sticker payloads",
    "## External provenance",
    "## Compatibility report artifacts",
    "## Typed transform/keyframe semantics",
];

const REQUIRED_JIANYING_STYLE_TERMS: &[&str] = &[
    "draft",
    "material",
    "track",
    "segment",
    "keyframe",
    "filter",
    "transition",
    "sticker",
    "canvas adjustment",
];

const REQUIRED_VERIFICATION_PHRASES: &[&str] = &[
    "Current source file gap",
    "Required canonical concept",
    "Import/report behavior until resolved",
    "Verification gate",
    "position",
    "scale",
    "rotation",
    "opacity",
    "volume",
];

fn read_gap_inventory(root: &Path) -> String {
    let path = root.join(GAP_INVENTORY_PATH);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()))
}

fn collect_claim_violations(dir: &Path, violations: &mut Vec<String>) {
    let entries = fs::read_dir(dir)
        .unwrap_or_else(|error| panic!("failed to read adapter source {}: {error}", dir.display()));

    for entry in entries {
        let entry = entry.expect("adapter source directory entry should be readable");
        let path = entry.path();
        if path.is_dir() {
            collect_claim_violations(&path, violations);
            continue;
        }
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }

        let source = fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", path.display()));
        for (line_index, line) in source.lines().enumerate() {
            if claims_preview_export_support(line) || claims_native_parity(line) {
                violations.push(format!("{}:{}: {}", path.display(), line_index + 1, line.trim()));
            }
        }
    }
}

fn claims_preview_export_support(line: &str) -> bool {
    let normalized = line.to_ascii_lowercase();
    normalized.contains("mapper")
        && normalized.contains("support")
        && (normalized.contains("preview") || normalized.contains("export"))
}

fn claims_native_parity(line: &str) -> bool {
    let normalized = line.to_ascii_lowercase();
    normalized.contains("native parity")
        || normalized.contains("pixel-level parity")
        || normalized.contains("kaipai parity")
}

fn project_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("adapter_kaipai crate should live under crates/")
        .to_path_buf()
}
