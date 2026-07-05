//! The FIRE rung: enemy fire, fire-at-cover, and the empty-cell fall-through
//! (AC3/AC4, GTW-377).

use bevy::prelude::*;
use gdtf_battle_sim::{Cell, CellLevel, OccupancyGrid, SquadVisibility};
use gdtf_test_utils::{clear_mouse, press_mouse};

use super::harness::*;

/// Marks `cell` as shootable COVER (a BLOCKING `TerrainKind::Cover` marker) and squad-VISIBLE —
/// the GTW-377 fire-at-cover target state. No occupant is placed (cover is structure, not a
/// ganger). Without the visible mark the new FIRE-AT-COVER rung would refuse the fire (the fog
/// gate is fail-closed on a cell absent from the fog).
fn place_cover(app: &mut App, cell: CellLevel) {
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_terrain(cell, gdtf_battle_sim::TerrainKind::Cover);
    let mut visible: bevy::platform::collections::HashSet<CellLevel> = app
        .world()
        .get_resource::<SquadVisibility>()
        .map(|fog| fog.visible_cells().copied().collect())
        .unwrap_or_default();
    visible.insert(cell);
    app.world_mut()
        .insert_resource(SquadVisibility::new(visible.clone(), visible));
}

// ---------------------------------------------------------------------------------
// AC3 — left-click ENEMY + fire mode -> FIRE, mutually exclusive.
// ---------------------------------------------------------------------------------

/// AC3 — with a fire mode selected, a player-faction selection, and a Left press on an
/// ENEMY-occupied cell where `can_fire` passes, exactly one `FireRequested` is emitted,
/// NO `MoveRequested`, and `SelectedShooter` is UNCHANGED that edge.
#[test]
fn left_click_enemy_with_fire_mode_fires_and_is_mutually_exclusive() {
    let mut app = control_app();
    let shooter_cell = CellLevel::new(Cell::new(2, 2), LEVEL);
    let ganger = spawn_player_shooter(&mut app, shooter_cell);
    set_selection(&mut app, ganger);
    set_fire_mode(&mut app, spec(0.2, 1));

    let target = CellLevel::new(Cell::new(6, 2), LEVEL);
    let _enemy = place_enemy(&mut app, target);
    set_hovered(&mut app, Some(target));

    press_mouse(&mut app, MouseButton::Left);
    app.update();

    assert_eq!(
        fires(&app).len(),
        1,
        "exactly one FireRequested on a Left press over an enemy with a fire mode",
    );
    assert!(
        moves(&app).is_empty(),
        "a FIRE edge must emit no MoveRequested",
    );
    assert_eq!(
        selected(&app),
        Some(ganger),
        "a FIRE edge must leave SelectedShooter unchanged",
    );
    let fire = fires(&app);
    assert_eq!(fire[0].shooter, ganger, "the fire shooter = the selection");
    assert_eq!(
        fire[0].target_cell,
        Cell::new(6, 2),
        "the fire target = the hovered cell",
    );
}

// ---------------------------------------------------------------------------------
// GTW-377 — left-click SHOOTABLE COVER / WALL + fire mode -> FIRE AT COVER (the cell is a
// valid fire target, not just gangers); the FireRequested aims at the cover cell.
// ---------------------------------------------------------------------------------

/// GTW-377 C1 / C3 / C6a / C6b (decision path) — with a fire mode selected, a player-faction
/// selection, and a Left press on a SHOOTABLE cover/wall cell (blocking structure, NO occupant,
/// squad-VISIBLE), exactly one `FireRequested` is emitted AIMED AT the cover cell, NO
/// `MoveRequested` (a blocked cell is never a move target), and `SelectedShooter` is UNCHANGED.
/// This proves the discriminator recognizes the cover cell as a valid fire target (C1) and the
/// click ROUTES the real fire request toward that cell (C3) — the input half of the end-to-end
/// fire-at-cover (the sim-side depletion is the bridge test).
#[test]
fn left_click_cover_with_fire_mode_fires_at_the_cover_cell() {
    let mut app = control_app();
    let shooter_cell = CellLevel::new(Cell::new(2, 2), LEVEL);
    let ganger = spawn_player_shooter(&mut app, shooter_cell);
    set_selection(&mut app, ganger);
    set_fire_mode(&mut app, spec(0.2, 1));

    // A cover cell straight ahead — blocking structure, NO occupant, squad-VISIBLE.
    let cover = CellLevel::new(Cell::new(6, 2), LEVEL);
    place_cover(&mut app, cover);
    set_hovered(&mut app, Some(cover));

    press_mouse(&mut app, MouseButton::Left);
    app.update();

    let fire = fires(&app);
    assert_eq!(
        fire.len(),
        1,
        "exactly one FireRequested on a Left press over shootable cover with a fire mode \
         (cover/walls are valid fire targets, GTW-377)",
    );
    assert!(
        moves(&app).is_empty(),
        "a FIRE-AT-COVER edge must emit no MoveRequested (a blocked cell is never a move target)",
    );
    assert_eq!(
        selected(&app),
        Some(ganger),
        "a FIRE-AT-COVER edge must leave SelectedShooter unchanged",
    );
    assert_eq!(fire[0].shooter, ganger, "the fire shooter = the selection");
    assert_eq!(
        fire[0].target_cell,
        Cell::new(6, 2),
        "the FireRequested aims at the hovered COVER cell (the click fires AT the cover)",
    );
    assert_eq!(
        fire[0].target_level, LEVEL,
        "the FireRequested carries the cover cell's storey",
    );
}

// ---------------------------------------------------------------------------------
// AC4 (GTW-356) — left-click EMPTY + fire mode + selection -> falls through to the
// two-click MOVE path: click-1 SETS the target (no fire); click-2 same cell COMMITS.
// ---------------------------------------------------------------------------------

/// AC4 — with a fire mode selected, a player-faction selection, and a Left press on an
/// EMPTY in-bounds unblocked cell, the fire mode does NOT lock out move: click-1 falls
/// through to SET the move target (no `FireRequested`, no `MoveRequested`); click-2 on the
/// SAME cell commits exactly one `MoveRequested` (the flagged fall-through precedence, now
/// over the two-click flow).
#[test]
fn empty_with_fire_mode_falls_through_to_two_click_move() {
    let mut app = control_app();
    let shooter_cell = CellLevel::new(Cell::new(10, 10), LEVEL);
    let ganger = spawn_player_shooter(&mut app, shooter_cell);
    set_selection(&mut app, ganger);
    set_fire_mode(&mut app, spec(0.2, 1));

    let dest = CellLevel::new(Cell::new(11, 10), LEVEL);
    set_hovered(&mut app, Some(dest));

    // Click-1: falls through FIRE (empty cell) to SET the move target — no fire, no move.
    press_mouse(&mut app, MouseButton::Left);
    app.update();
    assert!(
        fires(&app).is_empty(),
        "the fire mode must NOT lock out move on an empty cell (no FireRequested)",
    );
    assert!(
        moves(&app).is_empty(),
        "click-1 only sets the target (no immediate MoveRequested)",
    );
    assert_eq!(
        move_target(&app),
        Some(dest),
        "click-1 with a fire mode over an empty cell still SETS the move target",
    );

    // Click-2 on the SAME cell: COMMIT the move.
    clear_mouse(&mut app);
    set_hovered(&mut app, Some(dest));
    press_mouse(&mut app, MouseButton::Left);
    app.update();
    assert_eq!(
        moves(&app).len(),
        1,
        "click-2 on the same cell commits exactly one MoveRequested",
    );
    assert!(
        fires(&app).is_empty(),
        "the commit must still emit no FireRequested",
    );
}
