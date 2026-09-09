//! What the gangers in this suite carry, and what the mount hands them.

use bevy::{app::App, ecs::relationship::Relationship, prelude::Entity};
use gdtf_battle_sim::{
    test_support::{TEST_MOUNTED_WEAPON_KEY, single_mode, test_weapon_spec},
    weapon::{
        Accuracy, BaseSpread, DamageType, FatalBias, FireMode, Kickback, MeleeWeapon,
        MountedWeapon, Stable, WeaponName, WeaponRegistry, WeaponSpec, WieldedBy, Wields,
    },
};

pub(crate) const OWN_KEY: &str = "test-own-gun";

/// The gun the shooter that smashes a mount carries, keyed apart from the carried sidearm.
pub(crate) const SHOOTER_KEY: &str = "test-shooter-gun";

pub(crate) const MOUNT_DAMAGE_TYPE: DamageType = DamageType::Plasma;

pub(crate) const OWN_DAMAGE_TYPE: DamageType = DamageType::Las;

pub(crate) fn gun_spec(damage_type: DamageType) -> WeaponSpec {
    WeaponSpec {
        base_spread: BaseSpread::new(0.2),
        kickback: Kickback::new(0.2),
        fatal_bias: FatalBias::new(0.0),
        damage_type,
        fire_mode: FireMode::new(vec![single_mode(0.3, 1)]),
        ..test_weapon_spec()
    }
}

/// A gun tight enough to put its round on the cell it was aimed at, whatever the seed.
pub(crate) fn aimed_gun_spec(damage_type: DamageType) -> WeaponSpec {
    WeaponSpec {
        base_spread: BaseSpread::new(0.01),
        accuracy: Accuracy::new(4.0),
        stable: Stable::new(true),
        ..gun_spec(damage_type)
    }
}

pub(crate) fn test_ranged_registry() -> WeaponRegistry {
    WeaponRegistry::new([
        (
            WeaponName::new(OWN_KEY.to_owned()),
            gun_spec(OWN_DAMAGE_TYPE),
        ),
        (
            WeaponName::new(TEST_MOUNTED_WEAPON_KEY.to_owned()),
            gun_spec(MOUNT_DAMAGE_TYPE),
        ),
        (
            WeaponName::new(SHOOTER_KEY.to_owned()),
            aimed_gun_spec(OWN_DAMAGE_TYPE),
        ),
    ])
}

pub(crate) fn wields_mount(app: &mut App, ganger: Entity) -> bool {
    let world = app.world_mut();
    let mut query = world.query_filtered::<&WieldedBy, bevy::prelude::With<MountedWeapon>>();
    query
        .iter(world)
        .any(|wielded_by| wielded_by.get() == ganger)
}

/// The damage type of the weapon this ganger would fire right now, mount or carried gun.
pub(crate) fn firing_damage_type(app: &App, ganger: Entity) -> Option<DamageType> {
    let world = app.world();
    let weapon = world.get::<Wields>(ganger)?.firing_weapon(
        |entity| world.get::<MountedWeapon>(entity).is_some(),
        |entity| world.get::<MeleeWeapon>(entity).is_some(),
    )?;
    world.get::<DamageType>(weapon).copied()
}
