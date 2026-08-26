//! The Weapon rows no gate stands in front of: ballistics, damage, handling and magazine.

use gdtf_battle_sim::weapon::{FatalBias, WeaponDamage, WeaponPunch, WeaponShred, WeaponSpec};

use crate::net_qa::{
    commands::write::form_fault::FormWriteFault,
    wire::{
        AccuracyNet, BaseSpreadNet, DamageTypeNet, EditorFieldNet, FatalBiasNet, HandednessNet,
        KickbackNet, MagazineSizeNet, ReloadTuNet, ShoveNet, StableNet, TrajectoryStyleNet,
        WeaponDamageNet, WeaponPunchNet, WeaponShredNet,
    },
};

// The three drags the def panel's Stats group draws.
fn stats(spec: &mut WeaponSpec, field: EditorFieldNet) -> Result<EditorFieldNet, FormWriteFault> {
    match field {
        EditorFieldNet::WeaponBaseSpread(spread) => {
            spec.base_spread = spread.to_spread();
            Ok(EditorFieldNet::WeaponBaseSpread(BaseSpreadNet::new(
                *spec.base_spread,
            )))
        }
        EditorFieldNet::WeaponAccuracy(accuracy) => {
            spec.accuracy = accuracy.to_accuracy();
            Ok(EditorFieldNet::WeaponAccuracy(AccuracyNet::new(
                *spec.accuracy,
            )))
        }
        EditorFieldNet::WeaponKickback(kickback) => {
            spec.kickback = kickback.to_kickback();
            Ok(EditorFieldNet::WeaponKickback(KickbackNet::new(
                *spec.kickback,
            )))
        }
        _ => Err(FormWriteFault::ForeignArm),
    }
}

// The six rows the shared damage group draws, the same widget the Melee Weapon form uses.
fn damage(spec: &mut WeaponSpec, field: EditorFieldNet) -> Result<EditorFieldNet, FormWriteFault> {
    match field {
        EditorFieldNet::WeaponDamage(value) => {
            spec.damage = WeaponDamage::new(*value);
            Ok(EditorFieldNet::WeaponDamage(WeaponDamageNet::new(
                *spec.damage,
            )))
        }
        EditorFieldNet::WeaponPunch(punch) => {
            spec.punch = WeaponPunch::new(*punch);
            Ok(EditorFieldNet::WeaponPunch(WeaponPunchNet::new(
                *spec.punch,
            )))
        }
        EditorFieldNet::WeaponShred(shred) => {
            spec.shred = WeaponShred::new(*shred);
            Ok(EditorFieldNet::WeaponShred(WeaponShredNet::new(
                *spec.shred,
            )))
        }
        EditorFieldNet::WeaponDamageType(damage_type) => {
            spec.damage_type = damage_type.to_damage_type();
            Ok(EditorFieldNet::WeaponDamageType(
                DamageTypeNet::from_damage_type(spec.damage_type),
            ))
        }
        EditorFieldNet::WeaponFatalBias(bias) => {
            spec.fatal_bias = FatalBias::new(*bias);
            Ok(EditorFieldNet::WeaponFatalBias(FatalBiasNet::new(
                *spec.fatal_bias,
            )))
        }
        EditorFieldNet::WeaponHandedness(handedness) => {
            spec.handedness = handedness.to_handedness();
            Ok(EditorFieldNet::WeaponHandedness(
                HandednessNet::from_handedness(spec.handedness),
            ))
        }
        _ => Err(FormWriteFault::ForeignArm),
    }
}

// The Handling group's three rows and the Magazine group's two.
fn handling(
    spec: &mut WeaponSpec,
    field: EditorFieldNet,
) -> Result<EditorFieldNet, FormWriteFault> {
    match field {
        EditorFieldNet::WeaponTrajectory(trajectory) => {
            spec.trajectory = trajectory.to_style();
            Ok(EditorFieldNet::WeaponTrajectory(
                TrajectoryStyleNet::from_style(spec.trajectory),
            ))
        }
        EditorFieldNet::WeaponStable(stable) => {
            spec.stable = stable.to_stable();
            Ok(EditorFieldNet::WeaponStable(StableNet::new(*spec.stable)))
        }
        EditorFieldNet::WeaponShove(shove) => {
            spec.shove = shove.to_shove();
            Ok(EditorFieldNet::WeaponShove(ShoveNet::new(*spec.shove)))
        }
        EditorFieldNet::WeaponMagazineSize(size) => {
            spec.magazine.set_size(size.to_size());
            Ok(EditorFieldNet::WeaponMagazineSize(MagazineSizeNet::new(
                *spec.magazine.size(),
            )))
        }
        EditorFieldNet::WeaponMagazineReloadTu(reload) => {
            spec.magazine.set_reload_tu(reload.to_reload_tu());
            Ok(EditorFieldNet::WeaponMagazineReloadTu(ReloadTuNet::new(
                *spec.magazine.reload_tu(),
            )))
        }
        _ => Err(FormWriteFault::ForeignArm),
    }
}

/// Write one ungated Weapon row, answering the field as the spec stores it.
pub(super) fn write(
    spec: &mut WeaponSpec,
    field: EditorFieldNet,
) -> Result<EditorFieldNet, FormWriteFault> {
    match field {
        EditorFieldNet::WeaponBaseSpread(_)
        | EditorFieldNet::WeaponAccuracy(_)
        | EditorFieldNet::WeaponKickback(_) => stats(spec, field),
        EditorFieldNet::WeaponDamage(_)
        | EditorFieldNet::WeaponPunch(_)
        | EditorFieldNet::WeaponShred(_)
        | EditorFieldNet::WeaponDamageType(_)
        | EditorFieldNet::WeaponFatalBias(_)
        | EditorFieldNet::WeaponHandedness(_) => damage(spec, field),
        _ => handling(spec, field),
    }
}
