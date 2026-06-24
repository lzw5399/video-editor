use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
};

use draft_import::{
    AdaptationCategory, AdaptationReport, AdaptationReportItem, AdaptationReportSchemaVersion,
    AdaptationReportSummary, AdaptationSeverity, AdaptationStatus, AdaptationTargetKind,
    AdaptationTargetRef, DraftImportPlan, ExternalProvenanceRef,
};
use schemars::schema_for;
use serde_json::json;
use ts_rs::{Config, TS};

fn project_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .expect("draft_import should live under crates/")
        .to_path_buf()
}

#[test]
fn schema_exports_generated_adaptation_report_contracts_from_rust() {
    let root = project_root();
    let schema_path = root.join("schemas/adaptation-report.schema.json");
    let generated_ts_path = root.join("apps/desktop-electron/src/generated/TemplateImport.ts");

    let schema_json = adaptation_report_schema_json();
    assert_schema_includes_required_statuses(&schema_json);
    assert_schema_rejects_unknown_report_item_fields(&schema_json);
    assert_or_update_contract_file(&schema_path, &format!("{schema_json}\n"));

    let template_import_ts = ts_contract(&[
        export_decl::<AdaptationReportSchemaVersion>(),
        export_decl::<AdaptationStatus>(),
        export_decl::<AdaptationSeverity>(),
        export_decl::<AdaptationCategory>(),
        export_decl::<AdaptationTargetKind>(),
        export_decl::<AdaptationTargetRef>(),
        export_decl::<ExternalProvenanceRef>(),
        export_decl::<AdaptationReportSummary>(),
        export_decl::<AdaptationReportItem>(),
        export_decl::<AdaptationReport>(),
    ]);
    assert!(
        template_import_ts.contains("missingResource")
            && template_import_ts.contains("needsNativeEffect")
            && template_import_ts.contains("approximated")
            && template_import_ts.contains("dropped"),
        "generated TypeScript should expose the required report statuses"
    );
    assert_or_update_contract_file(generated_ts_path, &template_import_ts);
}

#[test]
fn schema_exports_generated_draft_import_plan_contract_from_rust() {
    let root = project_root();
    let schema_path = root.join("schemas/draft-import-plan.schema.json");

    let schema_json = draft_import_plan_schema_json();
    assert_draft_import_plan_schema_rejects_unknown_top_level_fields(&schema_json);
    assert_or_update_contract_file(&schema_path, &format!("{schema_json}\n"));
}

fn adaptation_report_schema_json() -> String {
    let schema = schema_for!(AdaptationReport);
    serde_json::to_string_pretty(&schema).expect("adaptation report schema should serialize")
}

fn draft_import_plan_schema_json() -> String {
    let schema = schema_for!(DraftImportPlan);
    serde_json::to_string_pretty(&schema).expect("draft import plan schema should serialize")
}

fn assert_schema_includes_required_statuses(schema_json: &str) {
    let schema_value: serde_json::Value =
        serde_json::from_str(schema_json).expect("adaptation report schema should parse");
    let status_values = schema_value
        .pointer("/$defs/AdaptationStatus/enum")
        .and_then(serde_json::Value::as_array)
        .expect("AdaptationStatus should expose string enum values")
        .iter()
        .map(|value| {
            value
                .as_str()
                .expect("AdaptationStatus enum values should be strings")
                .to_owned()
        })
        .collect::<BTreeSet<_>>();

    assert_eq!(
        status_values,
        BTreeSet::from([
            "approximated".to_owned(),
            "dropped".to_owned(),
            "missingResource".to_owned(),
            "needsNativeEffect".to_owned(),
            "supported".to_owned(),
        ])
    );
}

fn assert_schema_rejects_unknown_report_item_fields(schema_json: &str) {
    let schema_value: serde_json::Value =
        serde_json::from_str(schema_json).expect("adaptation report schema should parse");
    let schema =
        jsonschema::validator_for(&schema_value).expect("adaptation report schema should compile");

    schema
        .validate(&report_value(None))
        .expect("baseline report fixture should validate");

    assert!(
        schema
            .validate(&report_value(Some((
                "templateId",
                json!("external-template")
            ))))
            .is_err(),
        "report item schema should reject unknown provider-only item fields"
    );
}

fn assert_draft_import_plan_schema_rejects_unknown_top_level_fields(schema_json: &str) {
    let schema_value: serde_json::Value =
        serde_json::from_str(schema_json).expect("draft import plan schema should parse");
    let schema =
        jsonschema::validator_for(&schema_value).expect("draft import plan schema should compile");

    schema
        .validate(&draft_import_plan_value(None))
        .expect("baseline draft import plan should validate");
    assert!(
        schema
            .validate(&draft_import_plan_value(Some((
                "templateId",
                json!("external-template")
            ))))
            .is_err(),
        "draft import plan schema should reject unknown provider-only top-level fields"
    );
}

fn report_value(extra_item_field: Option<(&str, serde_json::Value)>) -> serde_json::Value {
    let mut item = json!({
        "status": "supported",
        "severity": "info",
        "category": "sourceMedia",
        "target": {
            "kind": "material",
            "id": "material-main-video"
        },
        "message": "Main video maps to a draft material.",
        "provenance": [{
            "sourceKind": "offlineTemplateBundle",
            "externalId": "source-video",
            "externalPath": "timeline.segments[0]"
        }]
    });

    if let Some((field, value)) = extra_item_field {
        item.as_object_mut()
            .expect("report item should be an object")
            .insert(field.to_owned(), value);
    }

    json!({
        "schemaVersion": 1,
        "sourceKind": "offlineTemplateBundle",
        "generatedAt": "2026-06-24T00:00:00Z",
        "summary": {
            "supported": 1,
            "approximated": 0,
            "dropped": 0,
            "missingResource": 0,
            "needsNativeEffect": 0
        },
        "items": [item]
    })
}

fn draft_import_plan_value(
    extra_top_level_field: Option<(&str, serde_json::Value)>,
) -> serde_json::Value {
    let mut plan = json!({
        "schemaVersion": 1,
        "importId": "template-alpha",
        "draftId": "draft-alpha",
        "draftName": "导入模板 Alpha",
        "canvasConfig": {
            "aspectRatio": {
                "kind": "preset",
                "preset": "ratio16x9"
            },
            "width": 1920,
            "height": 1080,
            "frameRate": {
                "numerator": 30,
                "denominator": 1
            },
            "background": {
                "kind": "black"
            },
            "adaptationPolicy": "manual"
        },
        "materials": [{
            "material": {
                "materialId": "material-main-video",
                "kind": "video",
                "uri": "resources/template-import/template-alpha/videos/main/source.mp4",
                "displayName": "source.mp4",
                "metadata": {
                    "duration": 5000000,
                    "width": 1920,
                    "height": 1080,
                    "frameRate": {
                        "numerator": 30,
                        "denominator": 1
                    },
                    "hasVideo": true,
                    "hasAudio": true,
                    "audioSampleRate": 48000,
                    "audioChannels": 2
                },
                "status": "available"
            }
        }],
        "tracks": [{
            "zOrder": 0,
            "track": {
                "trackId": "track-main-video",
                "kind": "video",
                "name": "主视频",
                "muted": false,
                "locked": false,
                "visible": true,
                "segments": [{
                    "segmentId": "segment-main-video",
                    "materialId": "material-main-video",
                    "sourceTimerange": {
                        "start": 0,
                        "duration": 5000000
                    },
                    "targetTimerange": {
                        "start": 0,
                        "duration": 5000000
                    },
                    "mainTrackMagnet": {
                        "enabled": true
                    },
                    "keyframes": [],
                    "filters": [],
                    "volume": {
                        "levelMillis": 1000
                    },
                    "audio": {
                        "gainMillis": 1000,
                        "panBalanceMillis": 0,
                        "fadeInDuration": {
                            "duration": 0
                        },
                        "fadeOutDuration": {
                            "duration": 0
                        },
                        "effectSlots": []
                    },
                    "visual": {
                        "visible": true,
                        "transform": {
                            "position": {
                                "x": 0,
                                "y": 0
                            },
                            "scale": {
                                "xMillis": 1000,
                                "yMillis": 1000
                            },
                            "rotation": {
                                "degrees": 0
                            },
                            "opacity": {
                                "valueMillis": 1000
                            },
                            "crop": {
                                "leftMillis": 0,
                                "rightMillis": 0,
                                "topMillis": 0,
                                "bottomMillis": 0
                            },
                            "anchor": {
                                "xMillis": 500,
                                "yMillis": 500
                            }
                        },
                        "fitMode": "fit",
                        "backgroundFilling": {
                            "kind": "none"
                        },
                        "blendMode": {
                            "kind": "normal"
                        },
                        "mask": {
                            "kind": "none"
                        }
                    }
                }]
            }
        }]
    });

    if let Some((field, value)) = extra_top_level_field {
        plan.as_object_mut()
            .expect("draft import plan should be an object")
            .insert(field.to_owned(), value);
    }

    plan
}

fn export_decl<T>() -> String
where
    T: TS + 'static,
{
    format!("export {}\n", T::decl(&ts_config()))
}

fn ts_config() -> Config {
    Config::new().with_large_int("number")
}

fn ts_contract(declarations: &[String]) -> String {
    let mut ts = String::from(
        "// This file was generated by Rust ts-rs declarations. Do not edit this file manually.\n\n",
    );
    for declaration in declarations {
        ts.push_str(declaration);
    }
    ts
}

fn assert_or_update_contract_file(path: impl AsRef<Path>, expected: &str) {
    let path = path.as_ref();

    if env::var_os("VE_UPDATE_GENERATED_CONTRACTS").as_deref() == Some(std::ffi::OsStr::new("1")) {
        fs::create_dir_all(path.parent().expect("contract path should have parent"))
            .expect("contract directory should be created");
        fs::write(path, expected).expect("contract artifact should be written");
        return;
    }

    let actual = fs::read_to_string(path).unwrap_or_else(|error| {
        panic!(
            "committed contract artifact should be readable at {}: {error}",
            path.display()
        )
    });
    assert_eq!(
        actual,
        expected,
        "generated contract artifact is stale: {}. Run with VE_UPDATE_GENERATED_CONTRACTS=1 to refresh.",
        path.display()
    );
}
