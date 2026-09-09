use bevy::prelude::*;
use cobalt_test_utils::{clear_mouse, press_mouse};
use gdtf_battle_sim::{
    prelude::{Cell, CellLevel, OccupancyGrid},
    visibility::SquadVisibility,
};

use super::harness::*;

fn place_cover(app: &mut App, cell: CellLevel) {
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_terrain(cell, gdtf_battle_sim::occupancy::TerrainKind::Cover);
    let mut visible: bevy::platform::collections::HashSet<CellLevel> = app
        .world()
        .get_resource::<SquadVisibility>()
        .map(|fog| fog.visible_cells().copied().collect())
        .unwrap_or_default();
    visible.insert(cell);
    app.world_mut()
        .insert_resource(SquadVisibility::new(visible.clone(), visible));
}

#[test]
fn left_click_enemy_with_fire_mode_fires_and_is_mutually_exclusive() {
    let mut app = control_app();
    let shooter_cell = CellLevel::new(Cell::new(2, 2), LEVEL);
    let ganger = spawn_player_shooter(&mut app, shooter_cell);
    set_selection(&mut app, ganger);

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

#[test]
fn left_click_cover_with_fire_mode_fires_at_the_cover_cell() {
    let mut app = control_app();
    let shooter_cell = CellLevel::new(Cell::new(2, 2), LEVEL);
    let ganger = spawn_player_shooter(&mut app, shooter_cell);
    set_selection(&mut app, ganger);

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
         (cover/walls are valid fire targets)",
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

#[test]
fn empty_with_fire_mode_falls_through_to_two_click_move() {
    let mut app = control_app();
    let shooter_cell = CellLevel::new(Cell::new(10, 10), LEVEL);
    let ganger = spawn_player_shooter(&mut app, shooter_cell);
    set_selection(&mut app, ganger);

    let dest = CellLevel::new(Cell::new(11, 10), LEVEL);
    set_hovered(&mut app, Some(dest));

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
