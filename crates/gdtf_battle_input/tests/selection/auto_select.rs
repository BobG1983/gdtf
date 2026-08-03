use bevy::prelude::*;
use gdtf_battle_input::{GdtfBattleInputPlugin, SelectedShooter};
use gdtf_battle_presenter::{ActiveLevel, ViewMode};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    ganger::LifeState,
    prelude::{Cell, CellLevel, Faction, Level},
    test_support::GangerEntityBuilder,
};

use super::harness::*;


fn placed_ganger(app: &mut App, faction: Faction, cell: CellLevel) -> Entity {
    GangerEntityBuilder::new()
        .faction(faction)
        .at(cell)
        .spawn(app.world_mut())
}

#[test]
fn auto_selects_deterministic_first_player_ganger() {
    let level = Level::new(0);
    let mut app = selection_app(level);

    let first = placed_ganger(
        &mut app,
        PLAYER_FACTION,
        CellLevel::new(Cell::new(2, 3), level),
    );
    let enemy = placed_ganger(
        &mut app,
        ENEMY_FACTION,
        CellLevel::new(Cell::new(0, 0), level),
    );
    let mid = placed_ganger(
        &mut app,
        PLAYER_FACTION,
        CellLevel::new(Cell::new(1, 1), level),
    );
    let _last = placed_ganger(
        &mut app,
        PLAYER_FACTION,
        CellLevel::new(Cell::new(9, 9), level),
    );

    assert_eq!(
        selected(&app),
        None,
        "precondition: nothing selected at start"
    );

    app.update();

    assert_eq!(
        selected(&app),
        Some(mid),
        "the lowest-Position player ganger by (level, y, x) must be auto-selected — by \
         cell ordering, NOT Entity id (forbidden pick = `_last`) NOR spawn/iteration \
         order (forbidden pick = `first`)",
    );
    assert_ne!(
        selected(&app),
        Some(first),
        "a spawn/iteration-order pick (first-spawned) must NOT win — only cell ordering",
    );
    assert_ne!(
        selected(&app),
        Some(enemy),
        "the enemy ganger (even at the lowest cell) must NEVER be auto-selected",
    );
}

#[test]
fn auto_select_orders_level_major() {
    let mut app = selection_app(Level::new(0));

    let _high_first = placed_ganger(
        &mut app,
        PLAYER_FACTION,
        CellLevel::new(Cell::new(0, 0), Level::new(2)),
    );
    let lower_storey = placed_ganger(
        &mut app,
        PLAYER_FACTION,
        CellLevel::new(Cell::new(50, 50), Level::new(0)),
    );
    let _high_last = placed_ganger(
        &mut app,
        PLAYER_FACTION,
        CellLevel::new(Cell::new(0, 0), Level::new(3)),
    );

    app.update();

    assert_eq!(
        selected(&app),
        Some(lower_storey),
        "a lower storey wins the (level, y, x) ordering even with a larger (y, x) — by \
         cell ordering, NOT Entity id (forbidden pick = `_high_last`) NOR spawn/iteration \
         order (forbidden pick = `_high_first`)",
    );
}

#[test]
fn auto_select_does_not_override_existing_selection() {
    let level = Level::new(0);
    let mut app = selection_app(level);

    let chosen = placed_ganger(
        &mut app,
        PLAYER_FACTION,
        CellLevel::new(Cell::new(9, 9), level),
    );
    let _lower = placed_ganger(
        &mut app,
        PLAYER_FACTION,
        CellLevel::new(Cell::new(0, 0), level),
    );
    app.world_mut()
        .insert_resource(SelectedShooter::new(chosen));

    for _ in 0..4 {
        app.update();
    }

    assert_eq!(
        selected(&app),
        Some(chosen),
        "an existing selection must NOT be overridden by the auto-select",
    );
}

#[test]
fn auto_select_enemy_only_stays_none() {
    let level = Level::new(0);
    let mut app = selection_app(level);

    placed_ganger(
        &mut app,
        ENEMY_FACTION,
        CellLevel::new(Cell::new(0, 0), level),
    );
    placed_ganger(
        &mut app,
        ENEMY_FACTION,
        CellLevel::new(Cell::new(1, 1), level),
    );

    for _ in 0..4 {
        app.update();
    }

    assert_eq!(
        selected(&app),
        None,
        "with no player-faction ganger present the selection must stay None",
    );
}

#[test]
fn a_selection_that_downs_clears_and_advances_to_an_alive_ganger() {
    let level = Level::new(0);
    let mut app = selection_app(level);

    let advance_to = placed_ganger(
        &mut app,
        PLAYER_FACTION,
        CellLevel::new(Cell::new(0, 0), level),
    );
    let downs = GangerEntityBuilder::new()
        .faction(PLAYER_FACTION)
        .life_state(LifeState::Alive)
        .at(CellLevel::new(Cell::new(9, 9), level))
        .spawn(app.world_mut());
    app.world_mut().insert_resource(SelectedShooter::new(downs));

    app.update();
    assert_eq!(
        selected(&app),
        Some(downs),
        "precondition: the higher-ordered ganger is selected while it is Alive",
    );

    app.world_mut().entity_mut(downs).insert(LifeState::Downed);
    app.update();

    assert_ne!(
        selected(&app),
        Some(downs),
        "a selection stranded on a just-Downed ganger must NOT persist (it cannot act)",
    );
    assert_eq!(
        selected(&app),
        Some(advance_to),
        "the stale selection clears and ADVANCES to the next Alive player ganger the same update",
    );
}

#[test]
fn auto_select_inert_without_battle_in_progress() {
    let level = Level::new(0);
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(GdtfBattleInputPlugin);
    // NOTE: no BattleInProgress inserted; PlayerFaction present so only the battle gate
    app.world_mut().insert_resource(ActiveLevel::new(level));
    app.world_mut().insert_resource(ViewMode::default());
    app.world_mut()
        .insert_resource(PlayerFaction::new(PLAYER_FACTION));

    placed_ganger(
        &mut app,
        PLAYER_FACTION,
        CellLevel::new(Cell::new(0, 0), level),
    );

    for _ in 0..4 {
        app.update();
    }

    assert_eq!(
        selected(&app),
        None,
        "the auto-select must be inert (selection stays None) without BattleInProgress",
    );
}
