//! Left-click select / clear + the player-faction gate (AC1, GTW-238).

use bevy::prelude::*;
use gdtf_battle_input::{GdtfBattleInputPlugin, PendingActIntent, SelectedShooter};
use gdtf_battle_sim::prelude::{Cell, CellLevel, Level, OccupancyGrid};
use gdtf_test_utils::press_left;

use super::harness::*;

// ---------------------------------------------------------------------------------
// AC1 — the plugin init_resources the selection substrate.
// ---------------------------------------------------------------------------------

/// AC1 — `GdtfBattleInputPlugin` `init_resource`s `SelectedShooter` (present + `None`)
/// and the `PendingActIntent` queue (present + empty) after one update.
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

// ---------------------------------------------------------------------------------
// AC3/AC4 — left-click selects the occupant; empty clears; faction-agnostic.
// ---------------------------------------------------------------------------------

/// A left-click on a PLAYER-faction-OCCUPIED hovered cell selects that occupant, driving
/// the REAL GTW-238 `left_click_act` SELECT branch.
#[test]
fn left_click_on_occupied_cell_selects_the_occupant() {
    let level = Level::new(0);
    let mut app = selection_app(level);

    // Place a player-faction occupant at a known cell and hover it.
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

/// A left-click on an EMPTY hovered cell with a NON-player-faction selection clears the
/// selection to `None` (GTW-238 CLEAR branch — the selection is not a player ganger, so
/// neither FIRE nor MOVE applies and the chain falls through to CLEAR). The
/// player-selection + empty-cell → MOVE case is covered in `control.rs` (AC2).
#[test]
fn left_click_on_empty_cell_clears_the_selection() {
    let level = Level::new(0);
    let mut app = selection_app(level);

    // Pre-seed a NON-player selection (a bare entity with no Faction), then click an
    // empty (unoccupied) hovered cell. The selection is not a player ganger, so MOVE
    // does not apply and the chain clears.
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

/// GTW-238 — selection is FACTION-GATED: a PLAYER-faction occupant selects, but an
/// ENEMY-faction occupant does NOT become a player-own selection (the FIRE/MOVE/CLEAR
/// chain handles it, never SELECT). Drives the REAL `left_click_act` for each.
#[test]
fn selection_is_player_faction_gated() {
    let level = Level::new(0);

    // A player-faction occupant selects.
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

    // An enemy-faction occupant is NOT selected (FIRE can't fire on it, SELECT rejects the
    // enemy faction). With NO prior selection the end-state stays `None`: clause 3.5 (GTW-287)
    // returns NoOp on the enemy click, leaving the (already-empty) selection untouched — never
    // selecting the enemy as own.
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

/// GTW-287 (Bug C) — with a PLAYER ganger selected, clicking an ENEMY-occupied cell you
/// cannot FIRE on is a NO-OP: `SelectedShooter` stays that player ganger across the update,
/// with NO transient `None` (no "No ganger selected" flash, no auto-select revert). Drives
/// the REAL `left_click_act` over `selection_app`.
///
/// Pin-discriminating: before GTW-287 the chain fell FIRE(fail) -> SELECT(fail, enemy
/// faction) -> MOVE(fail, occupied) -> CLEAR, wiping the selection to `None`. With the
/// GTW-287 clause 3.5 the enemy click returns `NoOp` and leaves the selection untouched.
/// The selected ganger carries NO firing components, so FIRE fails closed (an enemy you
/// can't fire on) — the case the contract targets.
#[test]
fn clicking_an_enemy_is_a_no_op_on_the_player_selection() {
    let level = Level::new(0);
    let mut app = selection_app(level);

    // A PLAYER-faction ganger, pre-selected (it carries no firing components, so FIRE fails
    // closed). Its occupancy cell is its own; it is NOT the click target.
    let own_cell = CellLevel::new(Cell::new(3, 3), level);
    let player_ganger = place_player_ganger(&mut app, own_cell);
    app.world_mut()
        .insert_resource(SelectedShooter::new(player_ganger));

    // An ENEMY occupant at the cell the player clicks.
    let enemy = app.world_mut().spawn(ENEMY_FACTION).id();
    let enemy_cell = CellLevel::new(Cell::new(40, 40), level);
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(enemy_cell, Some(enemy));
    set_hovered(&mut app, Some(enemy_cell));

    // Precondition: the player ganger is selected before the click.
    assert_eq!(
        selected(&app),
        Some(player_ganger),
        "precondition: the player ganger is selected before clicking the enemy",
    );

    press_left(&mut app);
    app.update();

    // The enemy click is a NO-OP: the selection is UNCHANGED (no clear, no transient None).
    assert_eq!(
        selected(&app),
        Some(player_ganger),
        "clicking an enemy you can't fire on must NOT clear the player's selection (GTW-287 \
         NoOp) — the enemy is inspected via the hover panel, never selected/cleared",
    );
    assert_ne!(
        selected(&app),
        None,
        "the enemy click must produce NO transient None — no 'No ganger selected' flash",
    );
}

/// GTW-288 — with a PLAYER ganger selected, a left-click with NO hovered cell (the
/// over-UI / margin / off-map case the GTW-286 viewport gate produces by resolving
/// `InspectTarget` to `None`) is a NO-OP: `SelectedShooter` stays that player ganger across
/// the update, with NO transient `None`. This is the regression GTW-286 introduced (every
/// bottom action-bar click cleared the selection, flickering the status panel and breaking
/// Mode/Stance) and GTW-288 fixes — the no-hover branch now returns `NoOp`, not `Clear`.
///
/// Pin-discriminating: reverting the no-hover branch to `Clear` wipes the selection to
/// `None`, failing the unchanged-selection assertion — RED. With the GTW-288 `NoOp` the
/// no-hover click leaves the selection untouched. Distinct from the empty-IN-GRID-cell case
/// (`left_click_on_empty_cell_clears_the_selection`), where `InspectTarget` is `Some` and the
/// chain genuinely CLEARs.
#[test]
fn clicking_with_no_hovered_cell_is_a_no_op_on_the_player_selection() {
    let level = Level::new(0);
    let mut app = selection_app(level);

    // A PLAYER-faction ganger, pre-selected (so the no-hover branch has a selection to
    // potentially clobber). Its occupancy cell is not the click target — there IS no target.
    let own_cell = CellLevel::new(Cell::new(3, 3), level);
    let player_ganger = place_player_ganger(&mut app, own_cell);
    app.world_mut()
        .insert_resource(SelectedShooter::new(player_ganger));

    // The GTW-286 over-UI / margin / off-map case: no hovered cell at all.
    set_hovered(&mut app, None);

    // Precondition: the player ganger is selected before the click.
    assert_eq!(
        selected(&app),
        Some(player_ganger),
        "precondition: the player ganger is selected before the no-hover click",
    );

    press_left(&mut app);
    app.update();

    // The no-hover click is a NO-OP: the selection is UNCHANGED (no clear, no transient None).
    assert_eq!(
        selected(&app),
        Some(player_ganger),
        "a left-click with NO hovered cell (over UI / margin / off map) must NOT clear the \
         player's selection (GTW-288 NoOp) — reverting it to Clear wipes the selection",
    );
    assert_ne!(
        selected(&app),
        None,
        "the no-hover click must produce NO transient None — no 'No ganger selected' flash",
    );
}
