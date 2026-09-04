use gdtf_battle_sim::armor::{
    ArmorFloor, ArmorHardness, ArmorIntegrity, ArmorProtection, ArmorType, BodyPart,
};

use super::super::assert_ron_round_trip;
use crate::mcp::wire::{
    ArmorFieldNet, ArmorFloorNet, ArmorHardnessNet, ArmorIntegrityNet, ArmorProtectionNet,
    ArmorTypeNet, BodyPartNet, EditorFieldNet,
};

#[test]
fn every_armor_field_arm_round_trips_for_every_piece() {
    for part in BodyPart::ALL {
        let part = BodyPartNet::from_part(part);
        assert_ron_round_trip(&EditorFieldNet::Armor(ArmorFieldNet::Floor {
            part,
            value: ArmorFloorNet::from_floor(ArmorFloor::new(3)),
        }));
        assert_ron_round_trip(&EditorFieldNet::Armor(ArmorFieldNet::Protection {
            part,
            value: ArmorProtectionNet::from_protection(ArmorProtection::new(4)),
        }));
        assert_ron_round_trip(&EditorFieldNet::Armor(ArmorFieldNet::Hardness {
            part,
            value: ArmorHardnessNet::from_hardness(ArmorHardness::new(5)),
        }));
        assert_ron_round_trip(&EditorFieldNet::Armor(ArmorFieldNet::Integrity {
            part,
            value: ArmorIntegrityNet::from_integrity(ArmorIntegrity::new(600)),
        }));
        for armor_type in ArmorType::ALL {
            assert_ron_round_trip(&EditorFieldNet::Armor(ArmorFieldNet::Type {
                part,
                value: ArmorTypeNet::from_type(armor_type),
            }));
        }
    }
}
