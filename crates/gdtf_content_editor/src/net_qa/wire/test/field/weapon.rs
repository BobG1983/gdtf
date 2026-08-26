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
    StableNet, TrajectoryStyleNet, WeaponDamageNet, WeaponPunchNet, WeaponShredNet,
};

#[test]
fn every_weapon_stat_field_arm_round_trips() {
    assert_ron_round_trip(&EditorFieldNet::WeaponName(a_name()));
    assert_ron_round_trip(&EditorFieldNet::WeaponBaseSpread(BaseSpreadNet::new(0.25)));
    assert_ron_round_trip(&EditorFieldNet::WeaponAccuracy(AccuracyNet::new(0.75)));
    assert_ron_round_trip(&EditorFieldNet::WeaponKickback(KickbackNet::new(0.5)));
    assert_ron_round_trip(&EditorFieldNet::WeaponDamage(WeaponDamageNet::new(9)));
    assert_ron_round_trip(&EditorFieldNet::WeaponPunch(WeaponPunchNet::new(2)));
    assert_ron_round_trip(&EditorFieldNet::WeaponShred(WeaponShredNet::new(1)));
    assert_ron_round_trip(&EditorFieldNet::WeaponFatalBias(FatalBiasNet::new(0.4)));
    for damage_type in DamageType::ALL {
        assert_ron_round_trip(&EditorFieldNet::WeaponDamageType(
            DamageTypeNet::from_damage_type(damage_type),
        ));
    }
    for handedness in [Handedness::OneHanded, Handedness::TwoHanded] {
        assert_ron_round_trip(&EditorFieldNet::WeaponHandedness(
            HandednessNet::from_handedness(handedness),
        ));
    }
}

#[test]
fn every_weapon_handling_field_arm_round_trips() {
    for style in [TrajectoryStyle::Straight, TrajectoryStyle::Arc] {
        assert_ron_round_trip(&EditorFieldNet::WeaponTrajectory(
            TrajectoryStyleNet::from_style(style),
        ));
    }
    assert_ron_round_trip(&EditorFieldNet::WeaponStable(StableNet::new(true)));
    assert_ron_round_trip(&EditorFieldNet::WeaponShove(ShoveNet::new(false)));
    assert_ron_round_trip(&EditorFieldNet::WeaponMagazineSize(MagazineSizeNet::new(
        24,
    )));
    assert_ron_round_trip(&EditorFieldNet::WeaponMagazineReloadTu(ReloadTuNet::new(6)));
}

#[test]
fn every_weapon_dot_field_arm_round_trips() {
    assert_ron_round_trip(&EditorFieldNet::WeaponDot(DotEnabledNet::new(true)));
    assert_ron_round_trip(&EditorFieldNet::WeaponDotDamage(DotDamageNet::new(4)));
    assert_ron_round_trip(&EditorFieldNet::WeaponDotTurns(DotTurnsNet::new(3)));
    assert_ron_round_trip(&EditorFieldNet::WeaponDotDamageType(
        DamageTypeNet::from_damage_type(DamageType::Plasma),
    ));
}

#[test]
fn every_weapon_on_death_field_arm_round_trips() {
    assert_ron_round_trip(&EditorFieldNet::WeaponOnDeath(OnDeathEnabledNet::new(true)));
    for variant in [OnDeathVariantNet::Explode, OnDeathVariantNet::LeaveField] {
        assert_ron_round_trip(&EditorFieldNet::WeaponOnDeathVariant(variant));
    }
    assert_ron_round_trip(&EditorFieldNet::WeaponOnDeathHitType(
        HitTypeNet::from_hit_type(HitType::Blast {
            radius: BlastRadius::new(2),
        }),
    ));
    assert_ron_round_trip(&EditorFieldNet::WeaponOnDeathDamage(ExplodeDamageNet::new(
        12,
    )));
    assert_ron_round_trip(&EditorFieldNet::WeaponOnDeathDamageType(
        DamageTypeNet::from_damage_type(DamageType::Blast),
    ));
    assert_ron_round_trip(&EditorFieldNet::WeaponOnDeathField(FieldKeyNet::from_key(
        &FieldKey::new("promethium_pool".to_owned()),
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
