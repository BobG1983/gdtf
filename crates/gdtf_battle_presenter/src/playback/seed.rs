//! [`seed_drawn_state`] — insert the `Drawn*` mirrors ONCE per entity, seeded from the
//! state that entity spawned with (GTW-727 C17).

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

/// The ganger state the mirrors are seeded from — factored into a [`QueryData`] tuple so
/// the system's query stays under clippy's `type_complexity` gate.
///
/// [`QueryData`]: bevy::ecs::query::QueryData
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

/// The weapon columns the magazine mirror is seeded from — factored into a [`QueryData`]
/// tuple so the system's query stays under clippy's `type_complexity` gate.
///
/// [`QueryData`]: bevy::ecs::query::QueryData
type WeaponSeedData = (Entity, &'static Magazine);

/// The filter selecting a wielded weapon that has NOT been given its drawn mirror yet — so
/// each weapon leaves this query the moment it is seeded, and pays one archetype move
/// rather than one per frame.
type UnseededWeapon = (With<WieldedBy>, Without<DrawnMagazine>);

/// `Update` ([`Replay`](crate::PresenterSystems::Replay), before
/// [`advance_playback`](super::advance_playback)): give every ganger that has none yet its
/// four drawn mirrors, and every wielded weapon its drawn magazine — seeded from the state
/// the entity currently holds.
///
/// **Insertion happens exactly once per entity, never per frame.** The `Without<Drawn…>`
/// filters mean an entity leaves this query the moment it is seeded, so each entity pays
/// one archetype move at spawn and none afterwards.
///
/// Seeding from live state (rather than from a log entry) is correct and necessary: the act
/// log records only TRANSITIONS, and a first observation deliberately records nothing — so
/// a freshly spawned roster produces no entries at all, and there is nothing for the cursor
/// to apply. The mirrors therefore start EQUAL to the sim, which is exactly right: at spawn
/// the drawn world and the sim world agree, and they only diverge once the sim acts.
///
/// Param-only (`bevy-traps.md` #7): [`Commands`] plus the two read-only seed queries.
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
