use std::collections::BTreeMap;

use draft_model::{
    Draft, DraftSchemaVersion, Filter, Keyframe, MainTrackMagnet, Material, MaterialKind,
    MaterialMetadata, Microseconds, RationalFrameRate, Segment, SourceTimerange, TargetTimerange,
    Track, TrackKind, Transition,
};
use serde_json::json;

#[test]
fn draft_schema_creates_valid_empty_draft() {
    let draft = Draft::new("draft-001", "Untitled");

    assert_eq!(draft.schema_version, DraftSchemaVersion::CURRENT);
    assert_eq!(draft.draft_id.as_str(), "draft-001");
    assert_eq!(draft.metadata.name, "Untitled");
    assert!(draft.materials.is_empty());
    assert!(draft.tracks.is_empty());

    let serialized = serde_json::to_value(&draft).expect("draft should serialize");
    assert_eq!(serialized["schemaVersion"], json!(1));
    assert_eq!(serialized["draftId"], json!("draft-001"));
    assert_eq!(serialized["materials"], json!([]));
    assert_eq!(serialized["tracks"], json!([]));
}

#[test]
fn draft_schema_serializes_material_track_and_segment_records() {
    let material = Material {
        material_id: "material-video-001".into(),
        kind: MaterialKind::Video,
        uri: "media/video.mp4".to_owned(),
        display_name: "video.mp4".to_owned(),
        metadata: MaterialMetadata {
            duration: Some(Microseconds::new(1_500_000)),
            width: Some(1920),
            height: Some(1080),
            frame_rate: Some(RationalFrameRate::new(30_000, 1_001)),
            has_video: true,
            has_audio: true,
            audio_sample_rate: Some(48_000),
            audio_channels: Some(2),
            probe_error: None,
        },
        status: draft_model::MaterialStatus::Available,
    };

    let mut filter_parameters = BTreeMap::new();
    filter_parameters.insert("intensity".to_owned(), "0.75".to_owned());

    let segment = Segment {
        segment_id: "segment-001".into(),
        material_id: material.material_id.clone(),
        source_timerange: SourceTimerange::new(250_000, 1_000_000),
        target_timerange: TargetTimerange::new(0, 1_000_000),
        main_track_magnet: MainTrackMagnet::enabled(),
        keyframes: vec![Keyframe {
            at: Microseconds::new(500_000),
            property: "opacity".to_owned(),
            value: "0.5".to_owned(),
        }],
        filters: vec![Filter {
            name: "brightness".to_owned(),
            parameters: filter_parameters,
        }],
        transition: Some(Transition {
            name: "fade".to_owned(),
            duration: Microseconds::new(100_000),
        }),
    };

    let mut track = Track::new("track-video-001", TrackKind::Video, "Video 1");
    track.segments.push(segment);

    let mut draft = Draft::new("draft-001", "Timeline draft");
    draft.materials.push(material);
    draft.tracks.push(track);

    let serialized = serde_json::to_value(&draft).expect("draft should serialize");
    assert_eq!(
        serialized["materials"][0]["metadata"]["duration"],
        json!(1_500_000)
    );
    assert_eq!(
        serialized["materials"][0]["metadata"]["frameRate"],
        json!({ "numerator": 30000, "denominator": 1001 })
    );
    assert_eq!(
        serialized["tracks"][0]["segments"][0]["sourceTimerange"],
        json!({ "start": 250000, "duration": 1000000 })
    );
    assert_eq!(
        serialized["tracks"][0]["segments"][0]["targetTimerange"],
        json!({ "start": 0, "duration": 1000000 })
    );

    let round_tripped: Draft =
        serde_json::from_value(serialized).expect("serialized draft should deserialize");
    assert_eq!(round_tripped, draft);
}

#[test]
fn draft_schema_rejects_unknown_fields() {
    let result = serde_json::from_value::<Draft>(json!({
        "schemaVersion": 1,
        "draftId": "draft-001",
        "metadata": { "name": "Unknown field draft" },
        "materials": [],
        "tracks": [],
        "previewCaches": []
    }));

    assert!(result.is_err(), "unknown draft fields must fail");

    let result = serde_json::from_value::<Material>(json!({
        "materialId": "material-001",
        "kind": "video",
        "uri": "media/video.mp4",
        "displayName": "video.mp4",
        "metadata": {
            "duration": 1000000,
            "hasVideo": true,
            "hasAudio": false
        },
        "status": "available",
        "thumbnailPath": "cache/thumb.jpg"
    }));

    assert!(result.is_err(), "unknown material fields must fail");
}

#[test]
fn draft_schema_serializes_integer_microseconds_and_rational_frame_rate() {
    let metadata = MaterialMetadata {
        duration: Some(Microseconds::new(3_333_333)),
        width: Some(1280),
        height: Some(720),
        frame_rate: Some(RationalFrameRate::new(24, 1)),
        has_video: true,
        has_audio: false,
        audio_sample_rate: None,
        audio_channels: None,
        probe_error: None,
    };

    let serialized = serde_json::to_value(metadata).expect("metadata should serialize");
    assert_eq!(serialized["duration"], json!(3_333_333));
    assert_eq!(
        serialized["frameRate"],
        json!({ "numerator": 24, "denominator": 1 })
    );
}

#[test]
fn draft_schema_excludes_derived_artifact_fields_from_draft() {
    let serialized = serde_json::to_value(Draft::new("draft-001", "Clean draft"))
        .expect("draft should serialize");
    let object = serialized
        .as_object()
        .expect("draft JSON should be an object");

    for forbidden_key in [
        "thumbnails",
        "waveforms",
        "previewCaches",
        "renderGraph",
        "ffmpegScripts",
        "exports",
        "rawProbeJson",
    ] {
        assert!(
            !object.contains_key(forbidden_key),
            "draft JSON must exclude derived artifact key {forbidden_key}"
        );
    }
}
