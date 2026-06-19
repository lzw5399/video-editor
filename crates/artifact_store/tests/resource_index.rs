use artifact_store::resource_index::{
    ResourceKind, ResourceRef, ResourceStatus, index_draft_resources, list_resources_for_material,
    resource_ref_for_effect, resource_ref_for_font, resource_ref_for_material, upsert_resource,
};
use draft_model::{
    Draft, Filter, Material, MaterialKind, Segment, SourceTimerange, TargetTimerange,
    TextAlignment, TextFont, TextSegment, TextSegmentSource, TextStyle, TextWrapping, Track,
    TrackKind, Transition,
};

#[test]
fn resource_index_indexes_material_font_effect_filter_transition_rows() {
    let sandbox = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = sandbox.path().join("draft.veproj");
    let draft = draft_with_resource_refs();

    let index = index_draft_resources(&bundle_path, &draft).expect("resources should index");

    let material = index
        .resource("material:video-001")
        .expect("material resource should exist");
    assert_eq!(material.resource_id.as_str(), "material:video-001");
    assert_eq!(material.kind, ResourceKind::Material);
    assert_eq!(material.stable_key, "material:video-001");
    assert_eq!(material.source_ref.as_deref(), Some("media/source.mp4"));
    assert_eq!(
        material.project_relative_ref.as_deref(),
        Some("media/source.mp4")
    );
    assert_eq!(material.status, ResourceStatus::Ready);
    assert!(
        !material
            .project_relative_ref
            .as_deref()
            .unwrap_or_default()
            .contains("derived"),
        "material refs must not point at derived blob paths"
    );

    assert_eq!(
        index
            .resource("font:fonts/PingFangSC.ttf")
            .expect("font resource should exist")
            .kind,
        ResourceKind::Font
    );
    assert_eq!(
        index
            .resource("filter:lut-cinematic")
            .expect("filter resource should exist")
            .kind,
        ResourceKind::Filter
    );
    assert_eq!(
        index
            .resource("transition:crossfade")
            .expect("transition resource should exist")
            .kind,
        ResourceKind::Transition
    );
    assert_eq!(
        index
            .resource("effect:text-shadow")
            .expect("text effect resource should exist")
            .kind,
        ResourceKind::Effect
    );

    let material_ref = resource_ref_for_material("video-001");
    assert_eq!(material_ref.kind, ResourceKind::Material);
    assert_eq!(material_ref.resource_id.as_str(), "material:video-001");
    assert_eq!(material_ref.stable_key, "material:video-001");
    assert_eq!(
        resource_ref_for_font("fonts/PingFangSC.ttf"),
        ResourceRef::new(
            ResourceKind::Font,
            "font:fonts/PingFangSC.ttf",
            "fonts/PingFangSC.ttf"
        )
    );
    assert_eq!(
        resource_ref_for_effect(ResourceKind::Filter, "lut-cinematic"),
        ResourceRef::new(
            ResourceKind::Filter,
            "filter:lut-cinematic",
            "lut-cinematic"
        )
    );
}

#[test]
fn resource_index_links_proxy_thumbnail_and_waveform_refs_to_material_resources() {
    let sandbox = tempfile::tempdir().expect("tempdir should be created");
    let bundle_path = sandbox.path().join("draft.veproj");
    let draft = draft_with_resource_refs();
    let mut index = index_draft_resources(&bundle_path, &draft).expect("resources should index");

    upsert_resource(
        &mut index,
        resource_ref_for_material("video-001").derived_role(ResourceKind::Proxy, "proxy"),
        Some("derived/blobs/proxy/video-001.mp4"),
    )
    .expect("proxy resource should upsert");
    upsert_resource(
        &mut index,
        resource_ref_for_material("video-001").derived_role(ResourceKind::Thumbnail, "thumbnail"),
        Some("derived/blobs/thumb/video-001.png"),
    )
    .expect("thumbnail resource should upsert");
    upsert_resource(
        &mut index,
        resource_ref_for_material("video-001").derived_role(ResourceKind::Waveform, "waveform"),
        Some("derived/blobs/waveform/video-001.json"),
    )
    .expect("waveform resource should upsert");

    let resources = list_resources_for_material(&index, "video-001");
    let kinds = resources
        .iter()
        .map(|resource| resource.kind)
        .collect::<Vec<_>>();
    assert!(kinds.contains(&ResourceKind::Material));
    assert!(kinds.contains(&ResourceKind::Proxy));
    assert!(kinds.contains(&ResourceKind::Thumbnail));
    assert!(kinds.contains(&ResourceKind::Waveform));
    for resource in resources {
        assert!(
            resource
                .project_relative_ref
                .as_deref()
                .map(|path| !path.starts_with('/') && !path.starts_with(".."))
                .unwrap_or(true),
            "derived artifact refs stay project-relative: {resource:?}"
        );
    }
}

fn draft_with_resource_refs() -> Draft {
    let mut draft = Draft::new("draft-resource-index", "Resource index draft");
    draft.materials.push(Material::new(
        "video-001",
        MaterialKind::Video,
        "media/source.mp4",
        "source",
    ));
    draft.materials.push(Material::new(
        "text-001",
        MaterialKind::Text,
        "text://title",
        "title",
    ));

    let mut segment = Segment::new(
        "segment-001",
        "video-001",
        SourceTimerange::new(0, 1_000_000),
        TargetTimerange::new(0, 1_000_000),
    );
    segment.filters.push(Filter {
        name: "lut-cinematic".to_owned(),
        parameters: Default::default(),
    });
    segment.transition = Some(Transition {
        name: "crossfade".to_owned(),
        duration: 200_000.into(),
    });

    let mut text_segment = Segment::new(
        "segment-text-001",
        "text-001",
        SourceTimerange::new(0, 1_000_000),
        TargetTimerange::new(0, 1_000_000),
    );
    text_segment.text = Some(TextSegment {
        content: "字幕".to_owned(),
        source: TextSegmentSource::Text,
        style: TextStyle {
            font: TextFont {
                family: "PingFang SC".to_owned(),
                font_ref: Some("fonts/PingFangSC.ttf".to_owned()),
            },
            font_size: 48,
            color: "#ffffff".to_owned(),
            alignment: TextAlignment::Center,
            line_height_millis: 1_200,
            letter_spacing_millis: 0,
            stroke: None,
            shadow: None,
            background: None,
        },
        text_box: Default::default(),
        layout_region: Default::default(),
        wrapping: TextWrapping::Auto,
        bubble: None,
        effect: Some(draft_model::TextEffectRef::Unsupported {
            name: "text-shadow".to_owned(),
            external_ref: Some("effect://text-shadow".to_owned()),
        }),
    });

    let mut video_track = Track::new("track-video", TrackKind::Video, "视频");
    video_track.segments.push(segment);
    let mut text_track = Track::new("track-text", TrackKind::Text, "文字");
    text_track.segments.push(text_segment);
    draft.tracks.push(video_track);
    draft.tracks.push(text_track);
    draft
}
