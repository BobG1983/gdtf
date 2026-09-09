use bevy::prelude::{App, Entity, Messages};

use super::support::{
    ENEMY, PLAYER, brain_app, drain_fires, ground, place_occupant, spawn_combatant, tu_of,
};
use crate::{
    acts::{ReloadOutcome, ReloadResult},
    ganger::Direction,
    magazine::Magazine,
    metric::Cell,
    weapon::Wields,
};

fn drain_reload_results(app: &mut App) -> Vec<ReloadResult> {
    app.world_mut()
        .resource_mut::<Messages<ReloadResult>>()
        .drain()
        .collect()
}

fn magazine_rounds(app: &App, wielder: Entity) -> Option<u16> {
    let wields = app.world().get::<Wields>(wielder)?;
    let weapon = wields.weapon()?;
    app.world()
        .get::<Magazine>(weapon)
        .map(|magazine| *magazine.rounds())
}

#[test]
fn out_of_ammo_enemy_reloads_then_fires() {
    let mut app = brain_app();
    let player_at = ground(8, 5);
    let enemy = spawn_combatant(
        app.world_mut(),
        ground(2, 5),
        ENEMY,
        Direction::East,
        100,
        0,
    );
    let player = spawn_combatant(app.world_mut(), player_at, PLAYER, Direction::West, 100, 2);
    place_occupant(&mut app, player_at, player);

    assert_eq!(
        magazine_rounds(&app, enemy),
        Some(0),
        "precondition: the enemy starts with an empty magazine",
    );

    let mut reloaded = false;
    loop {
        app.update();
        if drain_reload_results(&mut app)
            .iter()
            .any(|result| result.actor == enemy && result.outcome == ReloadOutcome::Reloaded)
        {
            reloaded = true;
            let rounds = magazine_rounds(&app, enemy);
            assert!(
                rounds.is_some_and(|count| count > 0),
                "the real reload dispatch must refill the empty magazine, got {rounds:?}",
            );
        }
        if reloaded
            && drain_fires(&mut app).iter().any(|fire| {
                fire.shooter == enemy
                    && fire.target_cell == Cell::new(8, 5)
                    && *fire.target_level == 0
            })
        {
            break;
        }
    }

    assert!(
        tu_of(&app, enemy) < 100,
        "the enemy spent TU reloading and firing: {}",
        tu_of(&app, enemy),
    );
}
