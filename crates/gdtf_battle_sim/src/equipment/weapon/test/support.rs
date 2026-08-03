pub(super) use bevy::prelude::World;

pub(super) use super::super::*;
pub(super) use crate::magazine::{Magazine, ReloadTu};

pub(super) const fn spec(cone: f32, tu: f32, shots: u16) -> FireModeSpec {
    kind_spec(ModeKind::Single, cone, tu, shots)
}

pub(super) const fn kind_spec(kind: ModeKind, cone: f32, tu: f32, shots: u16) -> FireModeSpec {
    FireModeSpec::new(
        kind,
        ModeConeMult::new(cone),
        ModeTuPercent::new(tu),
        ModeShots::new(shots),
    )
}

pub(super) fn profile(
    damage: i32,
    punch: i32,
    shred: i32,
    damage_type: DamageType,
) -> DamageProfile {
    DamageProfile::new(
        WeaponDamage::new(damage),
        WeaponPunch::new(punch),
        WeaponShred::new(shred),
        damage_type,
    )
}

pub(super) fn handling(mag: u16, stable: bool) -> HandlingProfile {
    HandlingProfile::new(
        Magazine::loaded(MagazineSize::new(mag), ReloadTu::new(12)),
        FireMode::new(vec![spec(1.0, 0.5, 1)]),
        Stable::new(stable),
        Shove::new(false),
        Handedness::OneHanded,
    )
}

pub(super) fn weapon_bundle(
    base: f32,
    accuracy: f32,
    kick: f32,
    bias: f32,
    damage: DamageProfile,
    handling: HandlingProfile,
) -> WeaponBundle {
    WeaponBundle::new(
        WeaponName::new("test-weapon".to_owned()),
        BaseSpread::new(base),
        Accuracy::new(accuracy),
        Kickback::new(kick),
        FatalBias::new(bias),
        damage,
        handling,
    )
}
