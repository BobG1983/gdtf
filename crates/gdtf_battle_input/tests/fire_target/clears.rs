use gdtf_battle_input::{SelectedFireMode, SelectedShooter};
use gdtf_battle_presenter::FireTargetHighlight;
use gdtf_battle_sim::{
    magazine::mode_tu_cost,
    occupancy::TerrainKind,
    prelude::{Cell, CellLevel, OccupancyGrid, Tu},
    tuning::CombatTuning,
};

use super::harness::*;

#[test]
fn fog_enemy_writes_no_highlight_even_when_armed() {
    let mut app = fire_target_app();
    let shooter_cell = CellLevel::new(Cell::new(10, 10), LEVEL);
    let enemy_cell = CellLevel::new(Cell::new(30, 30), LEVEL);

    let (_shooter, tu_max, aiming, _) = spawn_select_then_arm_late(&mut app, shooter_cell, 0.3);
    let resolved_mode = **app.world().resource::<SelectedFireMode>();
    let tuning = app.world().resource::<CombatTuning>();
    assert!(
        *mode_tu_cost(&resolved_mode, &tu_max, &aiming, tuning) > 0,
        "precondition: the shooter is armed with a non-zero mode (so only the fog gate refuses)",
    );
    let enemy = app.world_mut().spawn(ENEMY_FACTION).id();
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(enemy_cell, Some(enemy));
    set_hovered(&mut app, Some(enemy_cell));

    app.update();

    assert!(
        highlight(&app).is_empty(),
        "hovering an armed shooter's target on a NON-VISIBLE cell writes NO highlight \
         (GTW-346 fog gate, fail-closed)",
    );
}

#[test]
fn empty_cell_clears_highlight() {
    let mut app = fire_target_app();
    let shooter_cell = CellLevel::new(Cell::new(10, 10), LEVEL);
    let empty_cell = CellLevel::new(Cell::new(20, 20), LEVEL);

    spawn_and_select_shooter(&mut app, shooter_cell);
    set_fire_mode(&mut app, spec(0.2));
    app.world_mut().insert_resource(FireTargetHighlight::new(
        CellLevel::new(Cell::new(5, 5), LEVEL),
        Tu::new(9),
    ));
    set_hovered(&mut app, Some(empty_cell));

    app.update();

    assert!(
        highlight(&app).is_empty(),
        "hovering an EMPTY cell clears the fire-target highlight (no fireable enemy there)",
    );
}

#[test]
fn own_ganger_clears_highlight() {
    let mut app = fire_target_app();
    let shooter_cell = CellLevel::new(Cell::new(10, 10), LEVEL);

    spawn_and_select_shooter(&mut app, shooter_cell);
    set_fire_mode(&mut app, spec(0.2));
    mark_visible(&mut app, shooter_cell);
    set_hovered(&mut app, Some(shooter_cell));

    app.update();

    assert!(
        highlight(&app).is_empty(),
        "hovering your OWN ganger clears the highlight (an own-faction occupant is not fireable)",
    );
}

#[test]
fn non_visible_enemy_clears_highlight() {
    let mut app = fire_target_app();
    let shooter_cell = CellLevel::new(Cell::new(10, 10), LEVEL);
    let enemy_cell = CellLevel::new(Cell::new(30, 30), LEVEL);

    spawn_and_select_shooter(&mut app, shooter_cell);
    set_fire_mode(&mut app, spec(0.2));
    let enemy = app.world_mut().spawn(ENEMY_FACTION).id();
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_occupant(enemy_cell, Some(enemy));
    set_hovered(&mut app, Some(enemy_cell));

    app.update();

    assert!(
        highlight(&app).is_empty(),
        "hovering an enemy on a NON-VISIBLE cell clears the highlight (GTW-346 fog gate)",
    );
}

#[test]
fn no_selection_clears_highlight() {
    let mut app = fire_target_app();
    let enemy_cell = CellLevel::new(Cell::new(13, 11), LEVEL);

    place_enemy(&mut app, enemy_cell);
    set_fire_mode(&mut app, spec(0.2));
    app.world_mut().insert_resource(SelectedShooter::cleared());
    set_hovered(&mut app, Some(enemy_cell));

    app.update();

    assert!(
        highlight(&app).is_empty(),
        "with NO selected shooter, hovering an enemy populates no fire target",
    );
}

#[test]
fn bare_floor_writes_no_highlight() {
    let mut app = fire_target_app();
    let shooter_cell = CellLevel::new(Cell::new(10, 10), LEVEL);
    let floor_cell = CellLevel::new(Cell::new(13, 11), LEVEL);

    spawn_and_select_shooter(&mut app, shooter_cell);
    place_floor(&mut app, floor_cell);
    set_fire_mode(&mut app, spec(0.2));
    set_hovered(&mut app, Some(floor_cell));

    app.update();

    assert!(
        highlight(&app).is_empty(),
        "hovering a bare FLOOR cell writes NO fire-target highlight (it is a move destination, \
         not shootable structure)",
    );
}

#[test]
fn fog_cover_writes_no_highlight() {
    let mut app = fire_target_app();
    let shooter_cell = CellLevel::new(Cell::new(10, 10), LEVEL);
    let cover_cell = CellLevel::new(Cell::new(30, 30), LEVEL);

    spawn_and_select_shooter(&mut app, shooter_cell);
    app.world_mut()
        .resource_mut::<OccupancyGrid>()
        .set_terrain(cover_cell, TerrainKind::Cover);
    set_fire_mode(&mut app, spec(0.2));
    set_hovered(&mut app, Some(cover_cell));

    app.update();

    assert!(
        highlight(&app).is_empty(),
        "hovering cover on a NON-VISIBLE cell writes NO highlight (GTW-346 fog gate, fail-closed)",
    );
}
