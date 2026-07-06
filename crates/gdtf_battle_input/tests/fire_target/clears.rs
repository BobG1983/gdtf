//! The clear paths: fog enemy / empty / own ganger / non-visible / no selection
//! / bare floor / fog cover (C4c, GTW-346, GTW-377).

use gdtf_battle_input::{SelectedFireMode, SelectedShooter};
use gdtf_battle_presenter::FireTargetHighlight;
use gdtf_battle_sim::{
    magazine::mode_tu_cost,
    occupancy::TerrainKind,
    prelude::{Cell, CellLevel, OccupancyGrid, Tu},
    tuning::CombatTuning,
};

use super::harness::*;

/// GTW-376 DEFECT 2 (pin-discriminating, real-armed path) — hovering an enemy on a NON-VISIBLE
/// (fog) cell writes NO highlight, even when the shooter is fully armed (a real
/// weapon-resolved, non-zero `SelectedFireMode`) so the ONLY thing refusing the target is the
/// GTW-346 fog gate (`cell_squad_visible` → `CellVisibility::Visible`, fail-closed). FAILS if
/// the highlight leaks onto a fog enemy. The complementary visible-enemy case
/// ([`fireable_enemy_cost_resolves_off_weapon_and_is_nonzero`]) proves the SAME armed shooter
/// DOES highlight when the enemy cell is VISIBLE, so this isolates the fog gate as the cause.
#[test]
fn fog_enemy_writes_no_highlight_even_when_armed() {
    let mut app = fire_target_app();
    let shooter_cell = CellLevel::new(Cell::new(10, 10), LEVEL);
    let enemy_cell = CellLevel::new(Cell::new(30, 30), LEVEL);

    // Fully arm the shooter via the real resolution path (non-zero resolved mode).
    let (_shooter, tu_max, aiming, _) = spawn_select_then_arm_late(&mut app, shooter_cell, 0.3);
    // Precondition: the shooter is armed with a non-zero mode (so ONLY the fog gate can refuse
    // the target). Checked through the integer `mode_tu_cost` to avoid an f32 `==` (clippy
    // float_cmp) — a non-zero cost proves the mode resolved off the weapon, not the zero default.
    let resolved_mode = **app.world().resource::<SelectedFireMode>();
    let tuning = app.world().resource::<CombatTuning>();
    assert!(
        *mode_tu_cost(&resolved_mode, &tu_max, &aiming, tuning) > 0,
        "precondition: the shooter is armed with a non-zero mode (so only the fog gate refuses)",
    );
    // Spawn the enemy occupant but DO NOT mark its cell visible — it stays in fog.
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

/// C4c — NOT hovering a fireable enemy clears the highlight: an EMPTY cell yields no fire target
/// even with a selection.
#[test]
fn empty_cell_clears_highlight() {
    let mut app = fire_target_app();
    let shooter_cell = CellLevel::new(Cell::new(10, 10), LEVEL);
    let empty_cell = CellLevel::new(Cell::new(20, 20), LEVEL);

    spawn_and_select_shooter(&mut app, shooter_cell);
    set_fire_mode(&mut app, spec(0.2));
    // Seed a stale highlight, then hover an EMPTY cell.
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

/// C4c — hovering your OWN ganger clears the highlight (you cannot fire on your own).
#[test]
fn own_ganger_clears_highlight() {
    let mut app = fire_target_app();
    let shooter_cell = CellLevel::new(Cell::new(10, 10), LEVEL);

    spawn_and_select_shooter(&mut app, shooter_cell);
    set_fire_mode(&mut app, spec(0.2));
    // Mark the shooter's own cell visible so only the friend/foe gate (not the fog) decides it.
    mark_visible(&mut app, shooter_cell);
    set_hovered(&mut app, Some(shooter_cell));

    app.update();

    assert!(
        highlight(&app).is_empty(),
        "hovering your OWN ganger clears the highlight (an own-faction occupant is not fireable)",
    );
}

/// C4c — hovering an enemy on a NON-VISIBLE (unseen) cell clears the highlight (the GTW-346 fog
/// gate: you cannot target what the squad cannot see).
#[test]
fn non_visible_enemy_clears_highlight() {
    let mut app = fire_target_app();
    let shooter_cell = CellLevel::new(Cell::new(10, 10), LEVEL);
    let enemy_cell = CellLevel::new(Cell::new(30, 30), LEVEL);

    spawn_and_select_shooter(&mut app, shooter_cell);
    set_fire_mode(&mut app, spec(0.2));
    // Spawn the enemy occupant but DO NOT mark its cell visible — it stays unseen.
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

/// C4c — with NO selection, hovering even a visible enemy clears the highlight (there is no
/// shooter to fire).
#[test]
fn no_selection_clears_highlight() {
    let mut app = fire_target_app();
    let enemy_cell = CellLevel::new(Cell::new(13, 11), LEVEL);

    place_enemy(&mut app, enemy_cell);
    set_fire_mode(&mut app, spec(0.2));
    // No SelectedShooter set (the plugin inits it to the cleared default).
    app.world_mut().insert_resource(SelectedShooter::cleared());
    set_hovered(&mut app, Some(enemy_cell));

    app.update();

    assert!(
        highlight(&app).is_empty(),
        "with NO selected shooter, hovering an enemy populates no fire target",
    );
}

/// GTW-377 (pin-discriminating) — a bare FLOOR cell (unoccupied, NON-blocking, squad-VISIBLE)
/// writes NO highlight: an empty walkable cell is a MOVE destination, NEVER a fire target. This
/// discriminates the cover rung (it must be BLOCKING structure) from any unoccupied cell — the
/// complementary [`hovering_shootable_cover_populates_cell_and_mode_tu_cost`] proves the SAME
/// shooter DOES highlight a BLOCKING cover cell, isolating `is_blocked` as the cause.
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

/// GTW-377 (pin-discriminating, fog) — a cover/wall cell in FOG (blocking, but NOT squad-VISIBLE)
/// writes NO highlight: you can only shoot cover the squad can currently SEE (the GTW-346 fog
/// gate, `relation = None`, fail-closed). FAILS if the highlight leaks onto fog-hidden cover. The
/// complementary visible-cover case proves the SAME shooter DOES highlight a VISIBLE cover cell,
/// isolating the fog gate as the cause.
#[test]
fn fog_cover_writes_no_highlight() {
    let mut app = fire_target_app();
    let shooter_cell = CellLevel::new(Cell::new(10, 10), LEVEL);
    let cover_cell = CellLevel::new(Cell::new(30, 30), LEVEL);

    spawn_and_select_shooter(&mut app, shooter_cell);
    // Mark the cell COVER (blocking) but DO NOT mark it visible — it stays in fog.
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
