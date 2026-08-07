//! Selected fire mode resource and sync on selection change.

use bevy::prelude::*;
use gdtf_battle_sim::weapon::{FireMode, FireModeSpec, MeleeWeapon, ModeKind, WieldedBy, Wields};

use crate::SelectedShooter;

/// Currently selected fire mode for the active shooter.
#[derive(Resource, Deref, Debug, Clone, Copy, PartialEq)]
pub struct SelectedFireMode(FireModeSpec);

impl SelectedFireMode {
    /// Wrap a fire mode spec.
    #[must_use]
    pub const fn new(spec: FireModeSpec) -> Self {
        Self(spec)
    }
}

impl Default for SelectedFireMode {
    fn default() -> Self {
        Self(FireModeSpec::new(
            gdtf_battle_sim::weapon::ModeKind::Single,
            gdtf_battle_sim::weapon::ModeConeMult::new(1.0),
            gdtf_battle_sim::weapon::ModeTuPercent::new(0.0),
            gdtf_battle_sim::weapon::ModeShots::new(1),
        ))
    }
}

/// The ranged weapon `shooter` is wielding, if it holds one.
#[must_use]
pub fn ranged_weapon_of(
    shooter: Entity,
    wields: &Query<&Wields>,
    melee: &Query<(), With<MeleeWeapon>>,
) -> Option<Entity> {
    wields
        .get(shooter)
        .ok()
        .and_then(|held| held.ranged_weapon(|entity| melee.get(entity).is_ok()))
}

/// The selected shooter's ranged-weapon spec for `kind`, if it offers one.
#[must_use]
pub fn mode_spec_for(
    selected: SelectedShooter,
    wields: &Query<&Wields>,
    weapons: &Query<&FireMode, With<WieldedBy>>,
    melee: &Query<(), With<MeleeWeapon>>,
    kind: ModeKind,
) -> Option<FireModeSpec> {
    let shooter = (*selected)?;
    let weapon = ranged_weapon_of(shooter, wields, melee)?;
    let fire_mode = weapons.get(weapon).ok()?;
    fire_mode.iter().find(|spec| spec.kind == kind).copied()
}

/// Copy the selected shooter's ranged weapon fire mode into the resource.
pub fn sync_fire_mode_on_select(
    selected: Res<SelectedShooter>,
    wields: Query<&Wields>,
    weapons: Query<&FireMode, With<WieldedBy>>,
    melee: Query<(), With<MeleeWeapon>>,
    weapon_just_armed: Query<(), Added<FireMode>>,
    mut fire_mode: ResMut<SelectedFireMode>,
) {
    let weapon_arrived = weapon_just_armed.iter().next().is_some();
    if !selected.is_changed() && !weapon_arrived {
        return;
    }
    let Some(shooter) = **selected else {
        return;
    };
    let Some(weapon) =
        ranged_weapon_of(shooter, &wields, &melee).and_then(|weapon| weapons.get(weapon).ok())
    else {
        return;
    };
    let next = SelectedFireMode::new(weapon.single());
    if *fire_mode != next {
        *fire_mode = next;
    }
}
