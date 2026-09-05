use bevy::prelude::*;
use cobalt_test_utils::press_left;
use gdtf_battle_input::{GdtfBattleInputPlugin, PendingActIntent, SelectedShooter};
use gdtf_battle_sim::{
    ganger::LifeState,
    prelude::{Cell, CellLevel, Level, OccupancyGrid},
};

use super::harness::*;

#[test]
fn plugin_init_resources_the_selection_substrate() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(GdtfBattleInputPlugin);
    app.update();

    assert_eq!(
        selected(&app),
        None,
        "SelectedShooter must be init_resource-d and start None",
    );
    let pending = app.world().get_resource::<PendingActIntent>();
    assert!(
        pending.is_some_and(PendingActIntent::is_empty),
        "PendingActIntent must be init_resource-d and start empty",
    );
}

#[test]
fn left_click_on_occupied_cell_selects_the_occupant() {
    let level = Level::new(0);
    let mut app = selection_app(level);

    let cell = CellLevel::new(Cell::new(5, 7), level);
    let ganger = place_player_ganger(&mut app, cell);
    set_hovered(&mut app, Some(cell));

    press_left(&mut app);
    app.update();

    assert_eq!(
        selected(&app),
        Some(ganger),
        "a left-click on a player-faction occupied hovered cell must select its occupant",
    );
}

#[test]
fn left_click_on_empty_cell_clears_the_selection() {
    let level = Level::new(0);
    let mut app = selection_app(level);

    let ganger = mint_entity();
    app.world_mut()
        .insert_resource(SelectedShooter::new(ganger));
    set_hovered(&mut app, Some(CellLevel::new(Cell::new(1, 1), level)));

    press_left(&mut app);
    app.update();

    assert_eq!(
        selected(&app),
        None,
        "a left-click on an empty cell with a non-player selection must clear",
    );
}

#[test]
fn selection_is_player_faction_gated() {
    let level = Level::new(0);

    {
        let mut app = selection_app(level);
        let cell = CellLevel::new(Cell::new(2, 2), level);
        let ganger = place_player_ganger(&mut app, cell);
        set_hovered(&mut app, Some(cell));
        press_left(&mut app);
        app.update();
        assert_eq!(
            selected(&app),
            Some(ganger),
            "a player-faction occupant must be selectable",
        );
    }

    {
        let mut app = selection_app(level);
        let enemy = app.world_mut().spawn(ENEMY_FACTION).id();
        let cell = CellLevel::new(Cell::new(40, 40), level);
        app.world_mut()
            .resource_mut::<OccupancyGrid>()
            .set_occupant(cell, Some(enemy));
        set_hovered(&mut app, Some(cell));
        press_left(&mut app);
        app.update();
        assert_ne!(
            selected(&app),
            Some(enemy),
            "an enemy-faction occupant must never become a player-own selection",
        );
        assert_eq!(
            selected(&app),
            None,
            "an enemy occupant with no prior selection stays None (the NoOp leaves the empty \
             selection untouched — behaviorally identical to the old CLEAR end-state)",
        );
    }
}

#[test]
fn clicking_an_enemy_is_a_no_op_on_the_player_selection() {
    let level = Level::new(0);
    let mut app = selection_app(level);

    let own_cell = CellLevel::new(Cell::new(3, 3), level);
    let player_ganger = place_player_ganger(&mut app, own_cell);
    app.world_mut()
        .insert_resource(SelectedShooter::new(player_ganger));

    let enemy = app.world_mut().spawn(ENEMY_FACTION).id();
    let enemy_cell = CellLevel::new(Cell::new(40, 40), level);
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(enemy_cell, Some(enemy));
    set_hovered(&mut app, Some(enemy_cell));

    assert_eq!(
        selected(&app),
        Some(player_ganger),
        "precondition: the player ganger is selected before clicking the enemy",
    );

    press_left(&mut app);
    app.update();

    assert_eq!(
        selected(&app),
        Some(player_ganger),
        "clicking an enemy you can't fire on must NOT clear the player's selection \
         (NoOp) — the enemy is inspected via the hover panel, never selected/cleared",
    );
    assert_ne!(
        selected(&app),
        None,
        "the enemy click must produce NO transient None — no 'No ganger selected' flash",
    );
}

#[test]
fn left_click_on_downed_own_ganger_does_not_select_it() {
    let level = Level::new(0);
    let mut app = selection_app(level);

    let alive_cell = CellLevel::new(Cell::new(2, 2), level);
    let alive = place_player_ganger(&mut app, alive_cell);
    app.world_mut().insert_resource(SelectedShooter::new(alive));

    let downed_cell = CellLevel::new(Cell::new(8, 8), level);
    let downed = place_player_ganger_with_life(&mut app, downed_cell, LifeState::Downed);
    set_hovered(&mut app, Some(downed_cell));

    press_left(&mut app);
    app.update();

    assert_ne!(
        selected(&app),
        Some(downed),
        "a Downed own ganger must NEVER become the SelectedShooter (it cannot act)",
    );
    assert_eq!(
        selected(&app),
        Some(alive),
        "clicking a Downed ally is a NO-OP — the acting selection is left untouched, not cleared",
    );
}

#[test]
fn clicking_with_no_hovered_cell_is_a_no_op_on_the_player_selection() {
    let level = Level::new(0);
    let mut app = selection_app(level);

    let own_cell = CellLevel::new(Cell::new(3, 3), level);
    let player_ganger = place_player_ganger(&mut app, own_cell);
    app.world_mut()
        .insert_resource(SelectedShooter::new(player_ganger));

    set_hovered(&mut app, None);

    assert_eq!(
        selected(&app),
        Some(player_ganger),
        "precondition: the player ganger is selected before the no-hover click",
    );

    press_left(&mut app);
    app.update();

    assert_eq!(
        selected(&app),
        Some(player_ganger),
        "a left-click with NO hovered cell (over UI / margin / off map) must NOT clear the \
         player's selection (NoOp) — reverting it to Clear wipes the selection",
    );
    assert_ne!(
        selected(&app),
        None,
        "the no-hover click must produce NO transient None — no 'No ganger selected' flash",
    );
}
