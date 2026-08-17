use bevy::prelude::{App, Entity, Messages};

use super::support::{
    ENEMY, PLAYER, active_of, brain_app, drain_fires, ground, place_occupant, spawn_combatant,
    tu_of,
};
use crate::{
    acts::{ReloadOutcome, ReloadResult},
    ganger::Direction,
    magazine::Magazine,
    metric::Cell,
    weapon::Wields,
};

const FRAME_CAP: usize = 80;

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
    let mut fired_after_reload = false;
    let mut returned = false;
    for _ in 0..FRAME_CAP {
        app.update();
        if drain_reload_results(&mut app)
            .iter()
            .any(|result| result.actor == enemy && result.outcome == ReloadOutcome::Reloaded)
        {
            reloaded = true;
            assert_eq!(
                magazine_rounds(&app, enemy),
                Some(30),
                "the real reload dispatch must refill the empty magazine",
            );
        }
        if reloaded
            && drain_fires(&mut app).iter().any(|fire| {
                fire.shooter == enemy
                    && fire.target_cell == Cell::new(8, 5)
                    && *fire.target_level == 0
            })
        {
            fired_after_reload = true;
        }
        if active_of(&app) == PLAYER {
            returned = true;
            break;
        }
    }

    assert!(
        reloaded,
        "an out-of-ammo enemy must emit ReloadRequested and the dispatch must Reloaded",
    );
    assert!(
        fired_after_reload,
        "after reloading, the enemy must fire the visible player at (8,5,0)",
    );
    assert!(
        returned,
        "the enemy turn must terminate and hand control back to the player within the cap",
    );
    assert!(
        tu_of(&app, enemy) < 100,
        "the enemy spent TU reloading and firing: {}",
        tu_of(&app, enemy),
    );
}
