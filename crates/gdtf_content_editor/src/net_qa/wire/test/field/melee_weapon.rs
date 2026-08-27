use gdtf_battle_sim::weapon::DamageType;

use super::super::assert_ron_round_trip;
use crate::net_qa::wire::{
    DamageTypeNet, EditorDraftNameNet, EditorFieldNet, FatalBiasNet, HandednessNet,
    MeleeWeaponFieldNet, ReachNet, ShoveNet, WeaponDamageNet, WeaponPunchNet, WeaponShredNet,
};

#[test]
fn every_melee_weapon_field_arm_round_trips() {
    assert_ron_round_trip(&EditorFieldNet::MeleeWeapon(MeleeWeaponFieldNet::Name(
        EditorDraftNameNet::new("chain cleaver"),
    )));
    assert_ron_round_trip(&EditorFieldNet::MeleeWeapon(MeleeWeaponFieldNet::Damage(
        WeaponDamageNet::new(7),
    )));
    assert_ron_round_trip(&EditorFieldNet::MeleeWeapon(MeleeWeaponFieldNet::Punch(
        WeaponPunchNet::new(2),
    )));
    assert_ron_round_trip(&EditorFieldNet::MeleeWeapon(MeleeWeaponFieldNet::Shred(
        WeaponShredNet::new(1),
    )));
    assert_ron_round_trip(&EditorFieldNet::MeleeWeapon(
        MeleeWeaponFieldNet::DamageType(DamageTypeNet::from_damage_type(DamageType::Rend)),
    ));
    assert_ron_round_trip(&EditorFieldNet::MeleeWeapon(
        MeleeWeaponFieldNet::FatalBias(FatalBiasNet::new(0.25)),
    ));
    assert_ron_round_trip(&EditorFieldNet::MeleeWeapon(
        MeleeWeaponFieldNet::Handedness(HandednessNet::TwoHanded),
    ));
    assert_ron_round_trip(&EditorFieldNet::MeleeWeapon(MeleeWeaponFieldNet::Reach(
        ReachNet::new(2),
    )));
    assert_ron_round_trip(&EditorFieldNet::MeleeWeapon(MeleeWeaponFieldNet::Shove(
        ShoveNet::new(true),
    )));
}
