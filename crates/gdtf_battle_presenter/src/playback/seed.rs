//! Seed drawn components from live sim state at battle start.

use bevy::prelude::*;
use gdtf_battle_sim::{
    act_log::{MagazineFacts, PoseFacts, SuppressedNow, VitalsFacts},
    ganger::{Aiming, Facing, Hp, LifeState, Position, Stance, Suppressed, Tu, Wounds},
    inflicted_wound::InflictedWounds,
    injuries::InflictedInjuries,
    magazine::Magazine,
    weapon::WieldedBy,
};

use super::drawn::{DrawnLife, DrawnMagazine, DrawnPose, DrawnPosition, DrawnVitals};

type SeedData = (
    Entity,
    &'static Position,
    &'static Facing,
    &'static Stance,
    &'static Aiming,
    Option<&'static Suppressed>,
    &'static LifeState,
    &'static Tu,
    &'static Hp,
    &'static Wounds,
    Option<&'static InflictedWounds>,
    Option<&'static InflictedInjuries>,
);

type WeaponSeedData = (Entity, &'static Magazine);

type UnseededWeapon = (With<WieldedBy>, Without<DrawnMagazine>);

/// Insert drawn mirrors for gangers and weapons that lack them.
pub fn seed_drawn_state(
    mut commands: Commands,
    gangers: Query<SeedData, Without<DrawnPosition>>,
    weapons: Query<WeaponSeedData, UnseededWeapon>,
) {
    for (
        entity,
        position,
        facing,
        stance,
        aiming,
        suppressed,
        life,
        tu,
        hp,
        wounds,
        inflicted,
        injuries,
    ) in &gangers
    {
        commands.entity(entity).insert((
            DrawnPosition::seeded(*position),
            DrawnPose::new(PoseFacts::new(
                *facing,
                *stance,
                *aiming,
                SuppressedNow::new(suppressed.is_some()),
            )),
            DrawnLife::new(*life),
            DrawnVitals::new(VitalsFacts::new(
                *tu,
                *hp,
                *wounds,
                inflicted.cloned().unwrap_or_default(),
                injuries.cloned().unwrap_or_default(),
            )),
        ));
    }
    for (weapon, magazine) in &weapons {
        commands
            .entity(weapon)
            .insert(DrawnMagazine::new(MagazineFacts::new(*magazine)));
    }
}
