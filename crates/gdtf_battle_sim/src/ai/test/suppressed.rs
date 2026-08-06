use bevy::prelude::{App, Entity};

use super::support::{
    ENEMY, PLAYER, active_of, brain_app, drain_moves, ground, place_occupant, spawn_combatant,
};
use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::{Direction, Suppressed, SuppressorCell},
    metric::CellLevel,
};

/// Frames the enemy turn is given to finish.
const FRAME_CAP: usize = 80;

/// How far east the cover strip runs, past anything the enemy can reach.
const STRIP_END: i32 = 45;

/// Pin `ganger` down under fire coming from `from`.
fn suppress(app: &mut App, ganger: Entity, from: CellLevel) {
    app.world_mut()
        .entity_mut(ganger)
        .insert(Suppressed::new(SuppressorCell::new(from)));
}

/// Wall the whole row in cover, so a step east always ends with something at its back.
fn cover_the_row(app: &mut App, y: i32) {
    let Some(mut ledger) = app.world_mut().get_resource_mut::<CoverLedger>() else {
        unreachable!("the sim resources carry a cover ledger");
    };
    for x in 0..=STRIP_END {
        ledger.insert(
            super::support::ground(x, y),
            CoverEntry::seeded(
                CoverHp::new(10),
                HeightBand::Mid,
                ArmorProtection::new(1),
                ArmorHardness::new(1),
            ),
        );
    }
}

#[test]
fn a_pinned_enemy_that_cannot_break_away_ends_its_turn_instead_of_asking_forever() {
    let mut app = brain_app();
    let player_at = ground(40, 5);
    let enemy = spawn_combatant(app.world_mut(), ground(2, 5), ENEMY, Direction::East, 20, 0);
    let player = spawn_combatant(app.world_mut(), player_at, PLAYER, Direction::West, 100, 6);
    place_occupant(&mut app, player_at, player);
    suppress(&mut app, enemy, player_at);

    let mut moves = Vec::new();
    let mut returned = false;
    for _ in 0..FRAME_CAP {
        app.update();
        moves.extend(drain_moves(&mut app));
        if active_of(&app) == PLAYER {
            returned = true;
            break;
        }
    }

    assert!(
        moves.is_empty(),
        "stepping toward the fire that pinned this enemy is a move the sim refuses, so the brain \
         must never ask for it — asking again every frame is what hangs the turn: {moves:?}",
    );
    assert!(
        returned,
        "an enemy that can neither shoot nor legally move has nothing left to do, so the turn \
         must come back to the player inside {FRAME_CAP} frames",
    );
}

#[test]
fn a_pinned_enemy_still_advances_when_the_step_breaks_away_from_the_fire() {
    let mut app = brain_app();
    let player_at = ground(40, 5);
    let enemy = spawn_combatant(app.world_mut(), ground(2, 5), ENEMY, Direction::East, 20, 0);
    let player = spawn_combatant(app.world_mut(), player_at, PLAYER, Direction::West, 100, 6);
    place_occupant(&mut app, player_at, player);
    cover_the_row(&mut app, 5);
    suppress(&mut app, enemy, ground(-40, 5));

    let mut moves = Vec::new();
    for _ in 0..FRAME_CAP {
        app.update();
        moves.extend(drain_moves(&mut app));
        if active_of(&app) == PLAYER {
            break;
        }
    }

    assert!(
        moves.iter().any(|step| step.actor == enemy),
        "a step away from the fire is legal, so being pinned must not stop the advance \
         altogether: {moves:?}",
    );
}
