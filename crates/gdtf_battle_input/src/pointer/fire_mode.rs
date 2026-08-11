//! Selected fire mode resource and sync on selection change.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_sim::weapon::{
    FireMode, FireModeSpec, MeleeWeapon, ModeKind, MountedWeapon, WieldedBy, Wields, WieldsChanged,
};

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

/// The weapon `shooter` fires: the mounted one it mans, else the ranged one in its hands.
#[must_use]
pub fn firing_weapon_of(
    shooter: Entity,
    wields: &Query<&Wields>,
    mounted: &Query<(), With<MountedWeapon>>,
    melee: &Query<(), With<MeleeWeapon>>,
) -> Option<Entity> {
    wields.get(shooter).ok().and_then(|held| {
        held.firing_weapon(
            |entity| mounted.get(entity).is_ok(),
            |entity| melee.get(entity).is_ok(),
        )
    })
}

/// The selected shooter's spec for `kind` on the weapon it fires, if that weapon offers one.
#[must_use]
pub fn mode_spec_for(
    selected: SelectedShooter,
    wields: &Query<&Wields>,
    weapons: &Query<&FireMode, With<WieldedBy>>,
    mounted: &Query<(), With<MountedWeapon>>,
    melee: &Query<(), With<MeleeWeapon>>,
    kind: ModeKind,
) -> Option<FireModeSpec> {
    let shooter = (*selected)?;
    let weapon = firing_weapon_of(shooter, wields, mounted, melee)?;
    let fire_mode = weapons.get(weapon).ok()?;
    fire_mode.iter().find(|spec| spec.kind == kind).copied()
}

/// The lookups the fire-mode sync resolves the fired weapon against.
#[derive(SystemParam)]
pub struct FiredWeaponModes<'w, 's> {
    /// Weapons each shooter holds.
    wields:  Query<'w, 's, &'static Wields>,
    /// Fire modes of a wielded weapon.
    weapons: Query<'w, 's, &'static FireMode, With<WieldedBy>>,
    /// Mounted probe used to prefer a manned mount over the carried gun.
    mounted: Query<'w, 's, (), With<MountedWeapon>>,
    /// Melee probe used to skip melee weapons when picking the ranged one.
    melee:   Query<'w, 's, (), With<MeleeWeapon>>,
}

impl FiredWeaponModes<'_, '_> {
    /// The fire modes of the weapon this shooter fires, if it has one.
    fn of(&self, shooter: Entity) -> Option<&FireMode> {
        firing_weapon_of(shooter, &self.wields, &self.mounted, &self.melee)
            .and_then(|weapon| self.weapons.get(weapon).ok())
    }
}

/// Copy the fire mode of the weapon the selected shooter fires into the resource.
pub fn sync_fire_mode_on_select(
    selected: Res<SelectedShooter>,
    arms: FiredWeaponModes,
    weapon_just_armed: Query<(), Added<FireMode>>,
    mut wields_changed: WieldsChanged,
    mut fire_mode: ResMut<SelectedFireMode>,
) {
    let weapon_arrived = weapon_just_armed.iter().next().is_some();
    let armament_moved = wields_changed.any();
    if !selected.is_changed() && !weapon_arrived && !armament_moved {
        return;
    }
    let Some(shooter) = **selected else {
        return;
    };
    let Some(weapon) = arms.of(shooter) else {
        return;
    };
    let next = SelectedFireMode::new(weapon.single());
    if *fire_mode != next {
        *fire_mode = next;
    }
}
