use gdtf_battle_sim::{
    equipment::attachments::{AttachmentName, AttachmentSlot, SlotCapacity},
    weapon::{DamageType, FightModeKind, Handedness, Reach},
};

use super::{assert_ron_round_trip, assert_schema_is_usable};
use crate::net_qa::wire::{
    AttachmentKeyNet, AttachmentSlotNet, DamageTypeNet, EditorDraftNameNet, EditorFieldNet,
    FatalBiasNet, FightModeKindNet, FightModeSpecNet, HandednessNet, ReachNet, ShoveNet,
    SlotCapacityNet, StrikesNet, TuCostNet, WeaponDamageNet, WeaponPunchNet, WeaponShredNet,
    WeaponSlotNet,
};

fn a_mode() -> FightModeSpecNet {
    FightModeSpecNet::new(
        FightModeKindNet::Thrust,
        TuCostNet::new(6),
        StrikesNet::new(2),
    )
}

fn a_slot() -> WeaponSlotNet {
    WeaponSlotNet::new(
        AttachmentSlotNet::from_slot(AttachmentSlot::Pommel),
        SlotCapacityNet::new(1),
    )
}

#[test]
fn every_melee_weapon_value_round_trips() {
    for handedness in [Handedness::OneHanded, Handedness::TwoHanded] {
        assert_ron_round_trip(&HandednessNet::from_handedness(handedness));
    }
    assert_ron_round_trip(&ReachNet::from_reach(Reach::new(3)));
    assert_ron_round_trip(&ShoveNet::new(true));
    for kind in [FightModeKind::Swing, FightModeKind::Thrust] {
        assert_ron_round_trip(&FightModeKindNet::from_kind(kind));
    }
    assert_ron_round_trip(&TuCostNet::new(6));
    assert_ron_round_trip(&StrikesNet::new(2));
    assert_ron_round_trip(&FightModeSpecNet::from_spec(a_mode().to_spec()));
    assert_ron_round_trip(&SlotCapacityNet::new(2));
    assert_ron_round_trip(&WeaponSlotNet::from_declaration((
        AttachmentSlot::Muzzle,
        SlotCapacity::new(2),
    )));
    assert_ron_round_trip(&AttachmentKeyNet::from_key(&AttachmentName::new(
        "chain_teeth".to_owned(),
    )));
}

#[test]
fn every_melee_weapon_field_arm_round_trips() {
    assert_ron_round_trip(&EditorFieldNet::MeleeWeaponName(EditorDraftNameNet::new(
        "chain cleaver",
    )));
    assert_ron_round_trip(&EditorFieldNet::MeleeWeaponDamage(WeaponDamageNet::new(7)));
    assert_ron_round_trip(&EditorFieldNet::MeleeWeaponPunch(WeaponPunchNet::new(2)));
    assert_ron_round_trip(&EditorFieldNet::MeleeWeaponShred(WeaponShredNet::new(1)));
    assert_ron_round_trip(&EditorFieldNet::MeleeWeaponDamageType(
        DamageTypeNet::from_damage_type(DamageType::Rend),
    ));
    assert_ron_round_trip(&EditorFieldNet::MeleeWeaponFatalBias(FatalBiasNet::new(
        0.25,
    )));
    assert_ron_round_trip(&EditorFieldNet::MeleeWeaponHandedness(
        HandednessNet::TwoHanded,
    ));
    assert_ron_round_trip(&EditorFieldNet::MeleeWeaponReach(ReachNet::new(2)));
    assert_ron_round_trip(&EditorFieldNet::MeleeWeaponShove(ShoveNet::new(true)));
}

#[test]
fn a_fight_mode_and_a_slot_read_back_as_the_sims_own() {
    let spec = a_mode().to_spec();
    assert_eq!(FightModeSpecNet::from_spec(spec), a_mode());
    let declaration = (AttachmentSlot::Pommel, SlotCapacity::new(1));
    assert_eq!(WeaponSlotNet::from_declaration(declaration), a_slot());
}

#[test]
fn the_melee_weapon_values_trace_usable_shapes() {
    assert_schema_is_usable::<HandednessNet>("HandednessNet");
    assert_schema_is_usable::<ReachNet>("ReachNet");
    assert_schema_is_usable::<ShoveNet>("ShoveNet");
    assert_schema_is_usable::<FightModeKindNet>("FightModeKindNet");
    assert_schema_is_usable::<TuCostNet>("TuCostNet");
    assert_schema_is_usable::<StrikesNet>("StrikesNet");
    assert_schema_is_usable::<FightModeSpecNet>("FightModeSpecNet");
    assert_schema_is_usable::<SlotCapacityNet>("SlotCapacityNet");
    assert_schema_is_usable::<WeaponSlotNet>("WeaponSlotNet");
    assert_schema_is_usable::<AttachmentKeyNet>("AttachmentKeyNet");
}
