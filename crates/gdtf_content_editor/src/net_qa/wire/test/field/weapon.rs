use gdtf_battle_sim::{
    effects::fields::FieldKey,
    weapon::{BlastRadius, DamageType, Handedness, HitType, TrajectoryStyle},
};

use super::{
    super::{assert_ron_round_trip, assert_schema_is_usable},
    support::a_name,
};
use crate::net_qa::wire::{
    AccuracyNet, BaseSpreadNet, DamageTypeNet, DotDamageNet, DotEnabledNet, DotTurnsNet,
    EditorFieldNet, ExplodeDamageNet, FatalBiasNet, FieldKeyNet, HandednessNet, HitTypeNet,
    KickbackNet, MagazineSizeNet, OnDeathEnabledNet, OnDeathVariantNet, ReloadTuNet, ShoveNet,
    StableNet, TrajectoryStyleNet, WeaponDamageNet, WeaponFieldNet, WeaponPunchNet, WeaponShredNet,
};

// One Weapon field arm, under the form that owns it.
fn weapon(field: WeaponFieldNet) -> EditorFieldNet {
    EditorFieldNet::Weapon(field)
}

#[test]
fn every_weapon_stat_field_arm_round_trips() {
    assert_ron_round_trip(&weapon(WeaponFieldNet::Name(a_name())));
    assert_ron_round_trip(&weapon(WeaponFieldNet::BaseSpread(BaseSpreadNet::new(
        0.25,
    ))));
    assert_ron_round_trip(&weapon(WeaponFieldNet::Accuracy(AccuracyNet::new(0.75))));
    assert_ron_round_trip(&weapon(WeaponFieldNet::Kickback(KickbackNet::new(0.5))));
    assert_ron_round_trip(&weapon(WeaponFieldNet::Damage(WeaponDamageNet::new(9))));
    assert_ron_round_trip(&weapon(WeaponFieldNet::Punch(WeaponPunchNet::new(2))));
    assert_ron_round_trip(&weapon(WeaponFieldNet::Shred(WeaponShredNet::new(1))));
    assert_ron_round_trip(&weapon(WeaponFieldNet::FatalBias(FatalBiasNet::new(0.4))));
    for damage_type in DamageType::ALL {
        assert_ron_round_trip(&weapon(WeaponFieldNet::DamageType(
            DamageTypeNet::from_damage_type(damage_type),
        )));
    }
    for handedness in [Handedness::OneHanded, Handedness::TwoHanded] {
        assert_ron_round_trip(&weapon(WeaponFieldNet::Handedness(
            HandednessNet::from_handedness(handedness),
        )));
    }
}

#[test]
fn every_weapon_handling_field_arm_round_trips() {
    for style in [TrajectoryStyle::Straight, TrajectoryStyle::Arc] {
        assert_ron_round_trip(&weapon(WeaponFieldNet::Trajectory(
            TrajectoryStyleNet::from_style(style),
        )));
    }
    assert_ron_round_trip(&weapon(WeaponFieldNet::Stable(StableNet::new(true))));
    assert_ron_round_trip(&weapon(WeaponFieldNet::Shove(ShoveNet::new(false))));
    assert_ron_round_trip(&weapon(WeaponFieldNet::MagazineSize(MagazineSizeNet::new(
        24,
    ))));
    assert_ron_round_trip(&weapon(WeaponFieldNet::MagazineReloadTu(ReloadTuNet::new(
        6,
    ))));
}

#[test]
fn every_weapon_dot_field_arm_round_trips() {
    assert_ron_round_trip(&weapon(WeaponFieldNet::Dot(DotEnabledNet::new(true))));
    assert_ron_round_trip(&weapon(WeaponFieldNet::DotDamage(DotDamageNet::new(4))));
    assert_ron_round_trip(&weapon(WeaponFieldNet::DotTurns(DotTurnsNet::new(3))));
    assert_ron_round_trip(&weapon(WeaponFieldNet::DotDamageType(
        DamageTypeNet::from_damage_type(DamageType::Plasma),
    )));
}

#[test]
fn every_weapon_on_death_field_arm_round_trips() {
    assert_ron_round_trip(&weapon(WeaponFieldNet::OnDeath(OnDeathEnabledNet::new(
        true,
    ))));
    for variant in [OnDeathVariantNet::Explode, OnDeathVariantNet::LeaveField] {
        assert_ron_round_trip(&weapon(WeaponFieldNet::OnDeathVariant(variant)));
    }
    assert_ron_round_trip(&weapon(WeaponFieldNet::OnDeathHitType(
        HitTypeNet::from_hit_type(HitType::Blast {
            radius: BlastRadius::new(2),
        }),
    )));
    assert_ron_round_trip(&weapon(WeaponFieldNet::OnDeathDamage(
        ExplodeDamageNet::new(12),
    )));
    assert_ron_round_trip(&weapon(WeaponFieldNet::OnDeathDamageType(
        DamageTypeNet::from_damage_type(DamageType::Blast),
    )));
    assert_ron_round_trip(&weapon(WeaponFieldNet::OnDeathField(
        FieldKeyNet::from_key(&FieldKey::new("promethium_pool".to_owned())),
    )));
}

#[test]
fn the_weapon_values_trace_usable_shapes() {
    assert_schema_is_usable::<BaseSpreadNet>("BaseSpreadNet");
    assert_schema_is_usable::<AccuracyNet>("AccuracyNet");
    assert_schema_is_usable::<KickbackNet>("KickbackNet");
    assert_schema_is_usable::<TrajectoryStyleNet>("TrajectoryStyleNet");
    assert_schema_is_usable::<StableNet>("StableNet");
    assert_schema_is_usable::<MagazineSizeNet>("MagazineSizeNet");
    assert_schema_is_usable::<ReloadTuNet>("ReloadTuNet");
    assert_schema_is_usable::<DotEnabledNet>("DotEnabledNet");
    assert_schema_is_usable::<DotDamageNet>("DotDamageNet");
    assert_schema_is_usable::<DotTurnsNet>("DotTurnsNet");
    assert_schema_is_usable::<OnDeathEnabledNet>("OnDeathEnabledNet");
    assert_schema_is_usable::<OnDeathVariantNet>("OnDeathVariantNet");
    assert_schema_is_usable::<ExplodeDamageNet>("ExplodeDamageNet");
    assert_schema_is_usable::<FieldKeyNet>("FieldKeyNet");
}
