use std::path::Path;

use draft_model::{
    BUNDLED_TEXT_FONT_COVERAGE_SAMPLE, BUNDLED_TEXT_FONT_FAMILY, BUNDLED_TEXT_FONT_REF, TextFont,
    bundled_font_registry, validate_bundled_font_registry,
};

#[test]
fn font_registry_tracks_bundled_cjk_font_asset_license_and_glyphs() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let entries = bundled_font_registry();
    let font = entries
        .iter()
        .find(|entry| entry.font_ref == BUNDLED_TEXT_FONT_REF)
        .expect("bundled default font should be registered");

    assert_eq!(font.family, BUNDLED_TEXT_FONT_FAMILY);
    assert_eq!(font.style, "Regular");
    assert_eq!(font.weight, 400);
    assert_eq!(
        font.relative_path,
        "assets/fonts/noto-sans-cjk-sc/NotoSansCJKsc-Regular.otf"
    );
    assert_eq!(font.license_spdx, "OFL-1.1");
    assert_eq!(font.license_path, "assets/fonts/noto-sans-cjk-sc/OFL.txt");
    assert!(
        root.join(font.relative_path).is_file(),
        "bundled font file should exist"
    );
    assert!(
        root.join(font.license_path).is_file(),
        "bundled font license should exist"
    );

    let validations = validate_bundled_font_registry(&root).expect("bundled fonts should validate");
    assert!(validations.iter().any(|validation| {
        validation.font_ref == BUNDLED_TEXT_FONT_REF
            && validation.covered_sample == BUNDLED_TEXT_FONT_COVERAGE_SAMPLE
    }));
}

#[test]
fn text_font_default_uses_bundled_font_ref() {
    let font = TextFont::default();

    assert_eq!(font.family, BUNDLED_TEXT_FONT_FAMILY);
    assert_eq!(font.font_ref.as_deref(), Some(BUNDLED_TEXT_FONT_REF));
}
