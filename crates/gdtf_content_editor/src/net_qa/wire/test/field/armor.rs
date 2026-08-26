use gdtf_battle_sim::armor::{
    ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorType, BodyPart,
};

use super::super::assert_ron_round_trip;
use crate::net_qa::wire::{
    ArmorFloorNet, ArmorHardnessNet, ArmorIntegrityNet, ArmorProtectionNet, ArmorTypeNet,
    BodyPartNet, EditorFieldNet,
};

#[test]
fn every_armor_field_arm_round_trips_for_every_piece() {
    for part in BodyPart::ALL {
        let part = BodyPartNet::from_part(part);
        assert_ron_round_trip(&EditorFieldNet::ArmorFloor {
            part,
            value: ArmorFloorNet::from_floor(ArmorFloor::new(3)),
        });
        assert_ron_round_trip(&EditorFieldNet::ArmorProtection {
            part,
            value: ArmorProtectionNet::from_protection(ArmorProtection::new(4)),
        });
        assert_ron_round_trip(&EditorFieldNet::ArmorHardness {
            part,
            value: ArmorHardnessNet::from_hardness(ArmorHardness::new(5)),
        });
        assert_ron_round_trip(&EditorFieldNet::ArmorIntegrity {
            part,
            value: ArmorIntegrityNet::from_integrity(ArmorIntegrity::new(600)),
        });
        for armor_type in ArmorType::ALL {
            assert_ron_round_trip(&EditorFieldNet::ArmorType {
                part,
                value: ArmorTypeNet::from_type(armor_type),
            });
        }
    }
}
