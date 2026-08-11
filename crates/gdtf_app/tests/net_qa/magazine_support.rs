//! Setting a ganger's magazine before the socket cases drive a shot or a reload.

use bevy::{
    app::App,
    ecs::{entity::Entity, relationship::RelationshipTarget},
};
use gdtf_battle_sim::{
    magazine::{LoadedRounds, Magazine},
    weapon::{MeleeWeapon, Wields},
};

/// The ranged weapon this ganger holds a magazine on, when the world gave it one.
fn ranged_weapon_of(app: &App, wielder: Entity) -> Option<Entity> {
    let world = app.world();
    world.get::<Wields>(wielder)?.iter().find(|weapon| {
        world.get::<MeleeWeapon>(*weapon).is_none() && world.get::<Magazine>(*weapon).is_some()
    })
}

/// Empty this ganger's ranged magazine, answering what it holds afterwards.
pub(crate) fn empty_the_magazine(app: &mut App, wielder: Entity) -> Option<LoadedRounds> {
    let weapon = ranged_weapon_of(app, wielder)?;
    let mut magazine = app.world_mut().get_mut::<Magazine>(weapon)?;
    while !*magazine.is_empty() {
        magazine.spend_round();
    }
    Some(magazine.rounds())
}

/// Fill this ganger's ranged magazine to capacity, answering what it holds afterwards.
pub(crate) fn fill_the_magazine(app: &mut App, wielder: Entity) -> Option<LoadedRounds> {
    let weapon = ranged_weapon_of(app, wielder)?;
    let mut magazine = app.world_mut().get_mut::<Magazine>(weapon)?;
    magazine.refill();
    Some(magazine.rounds())
}
