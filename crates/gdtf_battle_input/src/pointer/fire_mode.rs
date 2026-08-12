//! The fire mode a weapon is set to, and the lookups that resolve it.

use bevy::{ecs::system::SystemParam, prelude::*};
use gdtf_battle_sim::weapon::{
    FireMode, FireModeSpec, MeleeWeapon, ModeKind, MountedWeapon, WieldedBy, Wields,
};

use crate::SelectedShooter;

/// The fire mode a weapon is set to, held on the weapon entity.
#[derive(Component, Deref, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChosenFireMode(ModeKind);

impl ChosenFireMode {
    /// Wrap the kind picked for this weapon.
    #[must_use]
    pub const fn new(kind: ModeKind) -> Self {
        Self(kind)
    }
}

/// The spec a weapon fires: its chosen kind's entry, else its own single.
/// Answers nothing when the chosen kind is not on its own list.
#[must_use]
pub fn chosen_spec(modes: &FireMode, chosen: Option<&ChosenFireMode>) -> Option<FireModeSpec> {
    match chosen {
        Some(chosen) => modes.iter().find(|spec| spec.kind == **chosen).copied(),
        None => Some(modes.single()),
    }
}

/// Set `weapon`'s chosen mode, writing only when the mode it fires actually changes.
pub fn set_chosen_mode(
    weapon: Entity,
    kind: ModeKind,
    weapons: &Query<&FireMode, With<WieldedBy>>,
    chosen: &mut Query<&mut ChosenFireMode>,
    commands: &mut Commands,
) {
    let next = ChosenFireMode::new(kind);
    if let Ok(mut current) = chosen.get_mut(weapon) {
        if *current != next {
            *current = next;
        }
    } else {
        // With nothing picked the weapon already reads as its own single.
        let unset = weapons.get(weapon).ok().map(|modes| modes.single().kind);
        if unset != Some(kind) {
            commands.entity(weapon).insert(next);
        }
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

/// The lookups a reader resolves the fired weapon's mode against.
#[derive(SystemParam)]
pub struct FiredWeaponModes<'w, 's> {
    /// Weapons each shooter holds.
    wields:  Query<'w, 's, &'static Wields>,
    /// Fire modes and chosen kind of a wielded weapon.
    weapons: Query<'w, 's, (&'static FireMode, Option<&'static ChosenFireMode>), With<WieldedBy>>,
    /// Mounted probe used to prefer a manned mount over the carried gun.
    mounted: Query<'w, 's, (), With<MountedWeapon>>,
    /// Melee probe used to skip melee weapons when picking the ranged one.
    melee:   Query<'w, 's, (), With<MeleeWeapon>>,
}

impl FiredWeaponModes<'_, '_> {
    /// The spec the weapon this shooter fires is set to, if it resolves.
    #[must_use]
    pub fn spec_of(&self, shooter: Entity) -> Option<FireModeSpec> {
        let weapon = firing_weapon_of(shooter, &self.wields, &self.mounted, &self.melee)?;
        let (modes, chosen) = self.weapons.get(weapon).ok()?;
        chosen_spec(modes, chosen)
    }

    /// The spec the selected shooter's weapon is set to, if anything is selected.
    #[must_use]
    pub fn spec_of_selected(&self, selected: SelectedShooter) -> Option<FireModeSpec> {
        self.spec_of((*selected)?)
    }
}
