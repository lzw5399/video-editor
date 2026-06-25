use draft_model::{
    BlendModeKind, CapabilitySupport, CapabilitySurface, EffectCapabilityRegistry,
    ExternalEffectReference, Filter, FilterKind, MaskKind, Microseconds, RetimeMode, Segment,
    SegmentBlendMode, SegmentMask, SegmentRetiming, SourceTimerange, SpeedCurvePoint, SpeedRatio,
    TargetTimerange, Transition, TransitionKind,
};

#[test]
fn phase19_production_effects_contracts_capability_registry_requires_typed_support_states() {
    let registry = EffectCapabilityRegistry::phase19_first_party();

    for capability_id in [
        "retime.constantSpeed",
        "retime.speedCurve",
        "transition.dissolve",
        "effect.gaussianBlur",
        "effect.basicColorAdjustment",
        "effect.opacityAdjustment",
        "mask.rectangle",
        "mask.ellipse",
        "blend.normal",
        "blend.multiply",
        "blend.screen",
    ] {
        let entry = registry
            .entry(capability_id)
            .unwrap_or_else(|| panic!("missing capability entry {capability_id}"));
        assert!(
            matches!(
                entry.support_for(CapabilitySurface::Preview),
                CapabilitySupport::Supported { .. } | CapabilitySupport::Degraded { .. }
            ),
            "{capability_id} should expose explicit preview support"
        );
        assert!(
            matches!(
                entry.support_for(CapabilitySurface::Export),
                CapabilitySupport::Supported { .. } | CapabilitySupport::Degraded { .. }
            ),
            "{capability_id} should expose explicit export support"
        );
    }

    let external = registry
        .entry("external:jianying:private-effect-id")
        .expect("external provider reference should be represented explicitly");
    assert!(matches!(
        external.support_for(CapabilitySurface::Preview),
        CapabilitySupport::ExternalReference { .. }
    ));
    assert!(matches!(
        external.support_for(CapabilitySurface::Export),
        CapabilitySupport::ExternalReference { .. }
    ));
}

#[test]
fn phase19_production_effects_contracts_first_party_filter_transition_and_retime_are_not_stringly()
{
    let filter = Filter::basic_color_adjustment(50, 1_100, 900);
    assert!(matches!(
        filter.kind,
        FilterKind::BasicColorAdjustment {
            brightness_millis: 50,
            contrast_millis: 1_100,
            saturation_millis: 900
        }
    ));
    assert_eq!(filter.capability_id(), "effect.basicColorAdjustment");

    let transition = Transition::dissolve(Microseconds::new(300_000));
    assert!(matches!(
        transition.reference,
        draft_model::TransitionReference::FirstParty {
            transition: TransitionKind::Dissolve
        }
    ));
    assert_eq!(transition.capability_id(), "transition.dissolve");

    let retiming = SegmentRetiming {
        mode: RetimeMode::SpeedCurve {
            points: vec![
                SpeedCurvePoint {
                    target_time: Microseconds::ZERO,
                    speed: SpeedRatio::new(1, 1),
                },
                SpeedCurvePoint {
                    target_time: Microseconds::new(500_000),
                    speed: SpeedRatio::new(3, 2),
                },
            ],
        },
        audio_policy: Default::default(),
    };
    let mut segment = Segment::new(
        "segment-a",
        "material-a",
        SourceTimerange::new(0, 1_000_000),
        TargetTimerange::new(0, 1_000_000),
    );
    segment.retiming = retiming;

    let json = serde_json::to_value(&segment).expect("segment serializes");
    assert!(
        json.pointer("/retiming/mode/points/0/targetTime").is_some(),
        "speed curve points must persist integer microsecond target times"
    );
    assert!(
        json.pointer("/retiming/mode/points/0/speed/numerator")
            .is_some(),
        "speed ratios must persist rational numerator/denominator fields"
    );
    assert!(
        json.pointer("/retiming/mode/points/0/speed/denominator")
            .is_some(),
        "speed ratios must persist rational numerator/denominator fields"
    );
}

#[test]
fn phase19_production_effects_contracts_external_provider_ids_remain_compatibility_references_only()
{
    let external = ExternalEffectReference::new("kaipai", "native-effect-42")
        .with_display_name("Kaipai native effect");
    let filter = Filter {
        kind: FilterKind::ExternalReference {
            reference: external.clone(),
        },
        enabled: true,
    };
    let transition = Transition::external_reference(
        "jianying",
        "native-transition-7",
        Microseconds::new(120_000),
    );
    let mask = SegmentMask::ExternalReference {
        reference: external.clone(),
    };
    let blend_mode = SegmentBlendMode::ExternalReference {
        reference: external.clone(),
    };

    assert_eq!(filter.external(), Some(&external));
    assert!(transition.external().is_some());
    assert_eq!(mask.mask_kind(), None);
    assert_eq!(blend_mode.kind(), None);
    assert_ne!(
        MaskKind::Rectangle.capability_id(),
        "kaipai:native-effect-42"
    );
    assert_ne!(
        BlendModeKind::Screen.capability_id(),
        "kaipai:native-effect-42"
    );
}
