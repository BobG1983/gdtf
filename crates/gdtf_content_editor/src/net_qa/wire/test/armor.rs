use gdtf_battle_sim::armor::{
    ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorType, BodyPart,
};

use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::net_qa::wire::{
    ArmorFloorNet, ArmorHardnessNet, ArmorIntegrityNet, ArmorProtectionNet, ArmorTypeNet,
    BodyPartNet,
};

#[test]
fn every_body_part_arm_round_trips_and_reads_back_as_the_part_it_mirrored() {
    for part in BodyPart::ALL {
        let mirrored = BodyPartNet::from_part(part);
        assert_ron_round_trip(&mirrored);
        assert_eq!(
            mirrored.to_part(),
            part,
            "a client's body part must come back as the sim's own, or a stat write would land \
             on a different piece than the one it named",
        );
    }
}

#[test]
fn every_armor_type_arm_round_trips_and_reads_back_as_the_type_it_mirrored() {
    for armor_type in ArmorType::ALL {
        let mirrored = ArmorTypeNet::from_type(armor_type);
        assert_ron_round_trip(&mirrored);
        assert_eq!(
            mirrored.to_type(),
            armor_type,
            "a client's armor type must come back as the sim's own",
        );
    }
}

#[test]
fn every_stat_round_trips_and_reads_back_as_the_value_it_mirrored() {
    let floor = ArmorFloorNet::from_floor(ArmorFloor::new(7));
    assert_ron_round_trip(&floor);
    assert_eq!(*floor.to_floor(), 7, "a floor survives the wire unchanged");

    let protection = ArmorProtectionNet::from_protection(ArmorProtection::new(11));
    assert_ron_round_trip(&protection);
    assert_eq!(
        *protection.to_protection(),
        11,
        "a protection survives the wire unchanged"
    );

    let hardness = ArmorHardnessNet::from_hardness(ArmorHardness::new(13));
    assert_ron_round_trip(&hardness);
    assert_eq!(
        *hardness.to_hardness(),
        13,
        "a hardness survives the wire unchanged"
    );

    let integrity = ArmorIntegrityNet::from_integrity(ArmorIntegrity::new(900));
    assert_ron_round_trip(&integrity);
    assert_eq!(
        *integrity.to_integrity(),
        900,
        "an integrity survives the wire unchanged"
    );
}

#[test]
fn the_armor_mirrors_trace_usable_shapes() {
    assert_schema_is_usable::<BodyPartNet>("BodyPartNet");
    assert_schema_is_usable::<ArmorTypeNet>("ArmorTypeNet");
    assert_schema_is_usable::<ArmorFloorNet>("ArmorFloorNet");
    assert_schema_is_usable::<ArmorProtectionNet>("ArmorProtectionNet");
    assert_schema_is_usable::<ArmorHardnessNet>("ArmorHardnessNet");
    assert_schema_is_usable::<ArmorIntegrityNet>("ArmorIntegrityNet");
}
