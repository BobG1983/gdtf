//! The Weapon rows no gate stands in front of: ballistics, damage, handling and magazine.

use gdtf_battle_sim::weapon::{FatalBias, WeaponDamage, WeaponPunch, WeaponShred, WeaponSpec};

use crate::mcp::wire::{
    AccuracyNet, BaseSpreadNet, DamageTypeNet, FatalBiasNet, HandednessNet, KickbackNet,
    MagazineSizeNet, ReloadTuNet, ShoveNet, StableNet, TrajectoryStyleNet, WeaponDamageNet,
    WeaponFieldNet, WeaponPunchNet, WeaponShredNet,
};

/// Write the Stats group's cone spread, answering the value the spec stores.
pub(super) fn base_spread(spec: &mut WeaponSpec, spread: BaseSpreadNet) -> WeaponFieldNet {
    spec.base_spread = spread.to_spread();
    WeaponFieldNet::BaseSpread(BaseSpreadNet::new(*spec.base_spread))
}

/// Write the Stats group's accuracy, answering the value the spec stores.
pub(super) fn accuracy(spec: &mut WeaponSpec, accuracy: AccuracyNet) -> WeaponFieldNet {
    spec.accuracy = accuracy.to_accuracy();
    WeaponFieldNet::Accuracy(AccuracyNet::new(*spec.accuracy))
}

/// Write the Stats group's kickback, answering the value the spec stores.
pub(super) fn kickback(spec: &mut WeaponSpec, kickback: KickbackNet) -> WeaponFieldNet {
    spec.kickback = kickback.to_kickback();
    WeaponFieldNet::Kickback(KickbackNet::new(*spec.kickback))
}

/// Write the damage group's damage, answering the value the spec stores.
pub(super) fn damage(spec: &mut WeaponSpec, value: WeaponDamageNet) -> WeaponFieldNet {
    spec.damage = WeaponDamage::new(*value);
    WeaponFieldNet::Damage(WeaponDamageNet::new(*spec.damage))
}

/// Write the damage group's punch, answering the value the spec stores.
pub(super) fn punch(spec: &mut WeaponSpec, punch: WeaponPunchNet) -> WeaponFieldNet {
    spec.punch = WeaponPunch::new(*punch);
    WeaponFieldNet::Punch(WeaponPunchNet::new(*spec.punch))
}

/// Write the damage group's shred, answering the value the spec stores.
pub(super) fn shred(spec: &mut WeaponSpec, shred: WeaponShredNet) -> WeaponFieldNet {
    spec.shred = WeaponShred::new(*shred);
    WeaponFieldNet::Shred(WeaponShredNet::new(*spec.shred))
}

/// Write the damage group's channel, answering the value the spec stores.
pub(super) const fn damage_type(
    spec: &mut WeaponSpec,
    damage_type: DamageTypeNet,
) -> WeaponFieldNet {
    spec.damage_type = damage_type.to_damage_type();
    WeaponFieldNet::DamageType(DamageTypeNet::from_damage_type(spec.damage_type))
}

/// Write the damage group's fatal bias, answering the value the spec stores.
pub(super) fn fatal_bias(spec: &mut WeaponSpec, bias: FatalBiasNet) -> WeaponFieldNet {
    spec.fatal_bias = FatalBias::new(*bias);
    WeaponFieldNet::FatalBias(FatalBiasNet::new(*spec.fatal_bias))
}

/// Write the damage group's handedness, answering the value the spec stores.
pub(super) const fn handedness(spec: &mut WeaponSpec, handedness: HandednessNet) -> WeaponFieldNet {
    spec.handedness = handedness.to_handedness();
    WeaponFieldNet::Handedness(HandednessNet::from_handedness(spec.handedness))
}

/// Write the Handling group's trajectory, answering the value the spec stores.
pub(super) const fn trajectory(
    spec: &mut WeaponSpec,
    trajectory: TrajectoryStyleNet,
) -> WeaponFieldNet {
    spec.trajectory = trajectory.to_style();
    WeaponFieldNet::Trajectory(TrajectoryStyleNet::from_style(spec.trajectory))
}

/// Write the Handling group's brace tick box, answering the value the spec stores.
pub(super) fn stable(spec: &mut WeaponSpec, stable: StableNet) -> WeaponFieldNet {
    spec.stable = stable.to_stable();
    WeaponFieldNet::Stable(StableNet::new(*spec.stable))
}

/// Write the Handling group's shove tick box, answering the value the spec stores.
pub(super) fn shove(spec: &mut WeaponSpec, shove: ShoveNet) -> WeaponFieldNet {
    spec.shove = shove.to_shove();
    WeaponFieldNet::Shove(ShoveNet::new(*spec.shove))
}

/// Write the Magazine group's size, answering the value the spec stores.
pub(super) fn magazine_size(spec: &mut WeaponSpec, size: MagazineSizeNet) -> WeaponFieldNet {
    spec.magazine.set_size(size.to_size());
    WeaponFieldNet::MagazineSize(MagazineSizeNet::new(*spec.magazine.size()))
}

/// Write the Magazine group's reload cost, answering the value the spec stores.
pub(super) fn magazine_reload_tu(spec: &mut WeaponSpec, reload: ReloadTuNet) -> WeaponFieldNet {
    spec.magazine.set_reload_tu(reload.to_reload_tu());
    WeaponFieldNet::MagazineReloadTu(ReloadTuNet::new(*spec.magazine.reload_tu()))
}
