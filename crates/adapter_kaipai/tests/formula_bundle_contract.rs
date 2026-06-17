use adapter_kaipai::{
    AdapterKaipaiError, FormulaBundleKind, FormulaBundleSchemaVersion, KaipaiFormulaBundle,
    ResourceKind, SafeAreaStatus,
};
use serde_json::{Value, json};

fn complete_formula_bundle_json() -> Value {
    json!({
        "schemaVersion": 1,
        "kind": "kaipaiSmartEditFormulaBundle",
        "provenance": {
            "templateId": "SEC0054",
            "recipeId": "1721819614877617",
            "formulaTaskId": "formula-task-1",
            "formulaRequestId": "formula-request-1",
            "capturedAt": "2026-06-17T00:00:00.000Z"
        },
        "sourceMedia": {
            "uri": "file:///fixtures/source.mp4",
            "width": 1080,
            "height": 1920,
            "durationMs": 3000
        },
        "recognizerResult": {
            "word_list": [
                {
                    "text": "sample",
                    "startMs": 0,
                    "endMs": 800
                }
            ]
        },
        "safeArea": {
            "value": "100,200,300,400",
            "status": "detected",
            "source": "app_face_detector_frame_0ms"
        },
        "directMaterials": [
            {
                "materialId": "direct-video-1",
                "uri": "file:///fixtures/source.mp4",
                "kind": "video",
                "displayName": "source.mp4"
            }
        ],
        "formula": {
            "tracks": [],
            "segments": []
        },
        "resources": [
            {
                "resourceId": "font-1",
                "kind": "font",
                "uri": "https://example.invalid/font.ttf",
                "sha256": "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
                "displayName": "Example Font"
            }
        ]
    })
}

#[test]
fn formula_bundle_contract_accepts_complete_sanitized_offline_evidence() {
    let bundle = KaipaiFormulaBundle::from_json_value(complete_formula_bundle_json()).unwrap();

    assert_eq!(bundle.schema_version, FormulaBundleSchemaVersion::current());
    assert_eq!(bundle.kind, FormulaBundleKind::KaipaiSmartEditFormulaBundle);
    assert_eq!(bundle.provenance.template_id, "SEC0054");
    assert_eq!(bundle.provenance.recipe_id, "1721819614877617");
    assert_eq!(bundle.source_media.duration_ms, 3000);
    assert_eq!(bundle.recognizer_result.word_list.len(), 1);
    assert_eq!(bundle.safe_area.status, SafeAreaStatus::Detected);
    assert_eq!(bundle.direct_materials.len(), 1);
    assert_eq!(bundle.resources[0].kind, ResourceKind::Font);
    assert!(bundle.formula.is_object());
}

#[test]
fn formula_bundle_contract_rejects_unknown_fields() {
    let mut json = complete_formula_bundle_json();
    json.as_object_mut()
        .unwrap()
        .insert("unexpectedProviderField".to_owned(), json!(true));

    let error = serde_json::from_value::<KaipaiFormulaBundle>(json).unwrap_err();

    assert!(
        error.to_string().contains("unknown field"),
        "unexpected error: {error}"
    );
}

#[test]
fn formula_bundle_contract_rejects_unsupported_schema_versions() {
    let mut json = complete_formula_bundle_json();
    json["schemaVersion"] = json!(999);

    let error = KaipaiFormulaBundle::from_json_value(json).unwrap_err();

    assert!(matches!(
        error,
        AdapterKaipaiError::UnsupportedSchemaVersion { .. }
    ));
}

#[test]
fn formula_bundle_contract_rejects_invalid_safe_area_status() {
    let mut json = complete_formula_bundle_json();
    json["safeArea"]["status"] = json!("providerSpecific");

    let error = serde_json::from_value::<KaipaiFormulaBundle>(json).unwrap_err();

    assert!(
        error.to_string().contains("unknown variant"),
        "unexpected error: {error}"
    );
}

#[test]
fn formula_bundle_contract_requires_recognizer_word_list() {
    let mut json = complete_formula_bundle_json();
    json["recognizerResult"]
        .as_object_mut()
        .unwrap()
        .remove("word_list");

    let error = serde_json::from_value::<KaipaiFormulaBundle>(json).unwrap_err();

    assert!(
        error.to_string().contains("missing field `word_list`"),
        "unexpected error: {error}"
    );
}

#[test]
fn formula_bundle_contract_rejects_empty_required_evidence() {
    let mut json = complete_formula_bundle_json();
    json["provenance"]["templateId"] = json!("");

    let error = KaipaiFormulaBundle::from_json_value(json).unwrap_err();

    assert!(matches!(
        error,
        AdapterKaipaiError::MissingRequiredEvidence { .. }
    ));
}
