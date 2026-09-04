use gdtf_battle_sim::weapon::{
    AoeRange, BlastRadius, ConeHalfAngle, FireModeSpec, HitType, ModeConeMult, ModeKind, ModeShots,
    ModeTuPercent,
};

use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::mcp::wire::fire_mode::{
    AoeRangeNet, BlastRadiusNet, ConeHalfAngleNet, FireModeSpecNet, HitTypeNet, ModeConeMultNet,
    ModeKindNet, ModeShotsNet, ModeTuPercentNet,
};

/// Every hit geometry the form's own picker offers.
fn every_hit_type() -> [HitType; 4] {
    [
        HitType::Single,
        HitType::Blast {
            radius: BlastRadius::new(2),
        },
        HitType::Cone {
            range: AoeRange::new(3),
            angle: ConeHalfAngle::new(30.0),
        },
        HitType::Line {
            range: AoeRange::new(3),
        },
    ]
}

fn a_spec(kind: ModeKind, hit_type: HitType) -> FireModeSpec {
    FireModeSpec::with_hit_type(
        kind,
        ModeConeMult::new(1.5),
        ModeTuPercent::new(0.25),
        ModeShots::new(3),
        hit_type,
    )
}

#[test]
fn every_mode_kind_round_trips_and_reads_back_as_the_kind_it_mirrored() {
    for kind in [ModeKind::Single, ModeKind::Burst, ModeKind::Full] {
        let mirrored = ModeKindNet::from_kind(kind);
        assert_ron_round_trip(&mirrored);
        assert_eq!(
            mirrored.to_kind(),
            kind,
            "a client's mode kind must come back as the sim's own",
        );
    }
}

#[test]
fn every_hit_type_arm_round_trips_and_reads_back_with_its_payload() {
    for hit_type in every_hit_type() {
        let mirrored = HitTypeNet::from_hit_type(hit_type);
        assert_ron_round_trip(&mirrored);
        assert_eq!(
            mirrored.to_hit_type(),
            hit_type,
            "a client's hit geometry must come back as the sim's own, radius and angle and all",
        );
    }
}

#[test]
fn a_spec_round_trips_with_all_five_fields_the_form_edits() {
    for kind in [ModeKind::Single, ModeKind::Burst, ModeKind::Full] {
        for hit_type in every_hit_type() {
            let spec = a_spec(kind, hit_type);
            let mirrored = FireModeSpecNet::from_spec(spec);
            assert_ron_round_trip(&mirrored);
            assert_eq!(
                mirrored.to_spec(),
                spec,
                "the wire carries every field the form's fire-mode row edits, hit_type included",
            );
        }
    }
}

#[test]
fn every_fire_mode_payload_round_trips_on_its_own() {
    assert_ron_round_trip(&ModeConeMultNet::new(1.5));
    assert_ron_round_trip(&ModeTuPercentNet::new(0.25));
    assert_ron_round_trip(&ModeShotsNet::new(3));
    assert_ron_round_trip(&BlastRadiusNet::new(2));
    assert_ron_round_trip(&AoeRangeNet::new(3));
    assert_ron_round_trip(&ConeHalfAngleNet::new(30.0));
}

#[test]
fn the_fire_mode_mirrors_trace_usable_shapes() {
    assert_schema_is_usable::<ModeKindNet>("ModeKindNet");
    assert_schema_is_usable::<HitTypeNet>("HitTypeNet");
    assert_schema_is_usable::<FireModeSpecNet>("FireModeSpecNet");
    assert_schema_is_usable::<ModeConeMultNet>("ModeConeMultNet");
    assert_schema_is_usable::<ModeTuPercentNet>("ModeTuPercentNet");
    assert_schema_is_usable::<ModeShotsNet>("ModeShotsNet");
    assert_schema_is_usable::<BlastRadiusNet>("BlastRadiusNet");
    assert_schema_is_usable::<AoeRangeNet>("AoeRangeNet");
    assert_schema_is_usable::<ConeHalfAngleNet>("ConeHalfAngleNet");
}
