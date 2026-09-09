use bevy::prelude::{App, Entity};

use super::support::{
    ENEMY, PLAYER, brain_app, drain_moves, drive_until_player_turn, ground, place_occupant,
    spawn_combatant,
};
use crate::{
    armor::{ArmorHardness, ArmorProtection},
    cover::{CoverEntry, CoverHp, CoverLedger, HeightBand},
    ganger::{Direction, Suppressed, SuppressorCell},
    metric::CellLevel,
    terrain::entity::TerrainPieceKind,
};

/// How far east the cover strip runs, past anything the enemy can reach.
const STRIP_END: i32 = 45;

/// Pin `ganger` down under fire coming from `from`.
fn suppress(app: &mut App, ganger: Entity, from: CellLevel) {
    app.world_mut()
        .entity_mut(ganger)
        .insert(Suppressed::new(SuppressorCell::new(from)));
}

/// Stand a wall tall enough to break the line back to the fire that pinned the enemy.
fn wall_the_line(app: &mut App, at: CellLevel) {
    let Some(mut ledger) = app.world_mut().get_resource_mut::<CoverLedger>() else {
        unreachable!("the sim resources carry a cover ledger");
    };
    ledger.insert(
        at,
        CoverEntry::seeded(
            CoverHp::new(10),
            HeightBand::High,
            ArmorProtection::new(1),
            ArmorHardness::new(1),
            TerrainPieceKind::Wall,
        ),
    );
}

/// Every move the enemy turn asks for, over the frames it takes the turn to come back.
fn moves_of_one_enemy_turn(app: &mut App) -> Vec<crate::acts::MoveRequested> {
    let mut moves = Vec::new();
    drive_until_player_turn(app, |app| moves.extend(drain_moves(app)));
    moves
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
                TerrainPieceKind::Cover,
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

    // An enemy that can neither shoot nor legally move ends its turn.
    let mut moves = Vec::new();
    drive_until_player_turn(&mut app, |app| moves.extend(drain_moves(app)));

    assert!(
        moves.is_empty(),
        "stepping toward the fire that pinned this enemy is a move the sim refuses, so the brain \
         must never ask for it — asking again every frame is what hangs the turn: {moves:?}",
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
    drive_until_player_turn(&mut app, |app| moves.extend(drain_moves(app)));

    assert!(
        moves.iter().any(|step| step.actor == enemy),
        "a step away from the fire is legal, so being pinned must not stop the advance \
         altogether: {moves:?}",
    );
}

#[test]
fn a_pinned_enemy_advances_when_the_step_only_breaks_the_line_back_to_the_fire() {
    let mut app = brain_app();
    let player_at = ground(40, 5);
    let enemy = spawn_combatant(
        app.world_mut(),
        ground(12, 5),
        ENEMY,
        Direction::East,
        20,
        0,
    );
    let player = spawn_combatant(app.world_mut(), player_at, PLAYER, Direction::West, 100, 6);
    place_occupant(&mut app, player_at, player);
    place_occupant(&mut app, ground(12, 5), enemy);
    wall_the_line(&mut app, ground(6, 5));
    suppress(&mut app, enemy, ground(0, 5));

    let moves = moves_of_one_enemy_turn(&mut app);

    assert!(
        moves.iter().any(|step| step.actor == enemy),
        "the brain judges a break-away by the same rule the sim does, so a step east that puts a \
         wall between the enemy and the cell the fire came from must be asked for: {moves:?}",
    );
}

#[test]
fn a_pinned_enemy_with_nothing_to_break_the_line_stays_put() {
    let mut app = brain_app();
    let player_at = ground(40, 5);
    let enemy = spawn_combatant(
        app.world_mut(),
        ground(12, 5),
        ENEMY,
        Direction::East,
        20,
        0,
    );
    let player = spawn_combatant(app.world_mut(), player_at, PLAYER, Direction::West, 100, 6);
    place_occupant(&mut app, player_at, player);
    place_occupant(&mut app, ground(12, 5), enemy);
    suppress(&mut app, enemy, ground(0, 5));

    let moves = moves_of_one_enemy_turn(&mut app);

    assert!(
        moves.is_empty(),
        "the same step across open ground gains distance but stays in view of the shot cell and \
         ends behind nothing, so the brain must not ask for it: {moves:?}",
    );
}
