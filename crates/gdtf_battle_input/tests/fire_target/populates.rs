use gdtf_battle_input::SelectedFireMode;
use gdtf_battle_sim::{
    acts::fire_arc_tu_cost,
    magazine::mode_tu_cost,
    prelude::{Cell, CellLevel, Tu},
    tuning::CombatTuning,
};

use super::harness::*;

#[test]
fn hovering_fireable_enemy_populates_cell_and_fire_arc_tu_cost() {
    let mut app = fire_target_app();
    let shooter_cell = CellLevel::new(Cell::new(10, 10), LEVEL);
    let enemy_cell = CellLevel::new(Cell::new(13, 11), LEVEL);
    let mode = spec(0.2);

    let (_shooter, tu_max, aiming) = spawn_and_select_shooter(&mut app, shooter_cell);
    place_enemy(&mut app, enemy_cell);
    set_fire_mode(&mut app, mode);
    set_hovered(&mut app, Some(enemy_cell));

    let tuning = app.world().resource::<CombatTuning>();
    let expected_cost = fire_arc_tu_cost(
        *SHOOTER_FACING,
        shooter_cell.cell(),
        enemy_cell.cell(),
        mode_tu_cost(&mode, &tu_max, &aiming, tuning),
        tuning,
    );

    app.update();

    let h = highlight(&app);
    assert_eq!(
        h.cell(),
        Some(enemy_cell),
        "hovering a fireable enemy populates the highlight with the hovered cell (13,11,L0)",
    );
    assert_eq!(
        h.cost(),
        Some(expected_cost),
        "the highlight cost EXACTLY equals fire_arc_tu_cost(facing, shooter cell, target cell, \
         mode_tu_cost(...), tuning) — the shot plus any turn it needs",
    );
}

#[test]
fn fireable_enemy_cost_resolves_off_weapon_and_is_nonzero() {
    let mut app = fire_target_app();
    let shooter_cell = CellLevel::new(Cell::new(10, 10), LEVEL);
    let enemy_cell = CellLevel::new(Cell::new(13, 11), LEVEL);

    let (_shooter, tu_max, aiming, _) = spawn_select_then_arm_late(&mut app, shooter_cell, 0.3);
    place_enemy(&mut app, enemy_cell);
    set_hovered(&mut app, Some(enemy_cell));

    let resolved_mode = **app.world().resource::<SelectedFireMode>();
    let tuning = app.world().resource::<CombatTuning>();
    let expected_cost = fire_arc_tu_cost(
        *SHOOTER_FACING,
        shooter_cell.cell(),
        enemy_cell.cell(),
        mode_tu_cost(&resolved_mode, &tu_max, &aiming, tuning),
        tuning,
    );

    app.update();

    let h = highlight(&app);
    assert_eq!(
        h.cell(),
        Some(enemy_cell),
        "hovering a fireable VISIBLE enemy populates the highlight with the hovered cell",
    );
    assert_eq!(
        h.cost(),
        Some(expected_cost),
        "the highlight cost EXACTLY equals fire_arc_tu_cost over the resolved SelectedFireMode",
    );
    assert!(
        h.cost().is_some_and(|cost| *cost > 0),
        "the resolved fire cost is strictly positive for a normal weapon/mode (not 0 TU); got {:?}",
        h.cost(),
    );
}

#[test]
fn hovering_shootable_cover_populates_cell_and_fire_arc_tu_cost() {
    let mut app = fire_target_app();
    let shooter_cell = CellLevel::new(Cell::new(10, 10), LEVEL);
    let cover_cell = CellLevel::new(Cell::new(13, 11), LEVEL);
    let mode = spec(0.2);

    let (_shooter, tu_max, aiming) = spawn_and_select_shooter(&mut app, shooter_cell);
    place_cover(&mut app, cover_cell);
    set_fire_mode(&mut app, mode);
    set_hovered(&mut app, Some(cover_cell));

    let tuning = app.world().resource::<CombatTuning>();
    let expected_cost = fire_arc_tu_cost(
        *SHOOTER_FACING,
        shooter_cell.cell(),
        cover_cell.cell(),
        mode_tu_cost(&mode, &tu_max, &aiming, tuning),
        tuning,
    );

    app.update();

    let h = highlight(&app);
    assert_eq!(
        h.cell(),
        Some(cover_cell),
        "hovering a shootable cover cell populates the highlight with the hovered cover cell \
         (13,11,L0) — cover is a valid fire target (C1)",
    );
    assert_eq!(
        h.cost(),
        Some(expected_cost),
        "the cover-target highlight cost EXACTLY equals fire_arc_tu_cost — same TU-cost treatment \
         as a ganger target (C2)",
    );
}

#[test]
fn off_facing_enemy_cost_includes_the_turn() {
    let mut app = fire_target_app();
    let shooter_cell = CellLevel::new(Cell::new(10, 10), LEVEL);
    let behind_cell = CellLevel::new(Cell::new(7, 10), LEVEL);
    let mode = spec(0.2);

    let (_shooter, tu_max, aiming) = spawn_and_select_shooter(&mut app, shooter_cell);
    place_enemy(&mut app, behind_cell);
    set_fire_mode(&mut app, mode);
    set_hovered(&mut app, Some(behind_cell));

    let tuning = app.world().resource::<CombatTuning>();
    let shot_only = mode_tu_cost(&mode, &tu_max, &aiming, tuning);
    let expected_cost = fire_arc_tu_cost(
        *SHOOTER_FACING,
        shooter_cell.cell(),
        behind_cell.cell(),
        shot_only,
        tuning,
    );

    app.update();

    let h = highlight(&app);
    assert_eq!(
        h.cell(),
        Some(behind_cell),
        "hovering a fireable enemy outside the firing arc still populates the highlight",
    );
    assert_eq!(
        h.cost(),
        Some(expected_cost),
        "the highlight cost EXACTLY equals fire_arc_tu_cost for a target the shooter must turn to \
         face",
    );
    assert!(
        *expected_cost > *shot_only,
        "precondition: turning to face this target costs TU, so the total is strictly greater \
         than the shot alone ({expected_cost:?} vs {shot_only:?})",
    );
}

#[test]
fn unaffordable_total_still_populates_the_highlight() {
    let mut app = fire_target_app();
    let shooter_cell = CellLevel::new(Cell::new(10, 10), LEVEL);
    let behind_cell = CellLevel::new(Cell::new(7, 10), LEVEL);
    let mode = spec(0.2);

    let (shooter, tu_max, aiming) = spawn_and_select_shooter(&mut app, shooter_cell);
    place_enemy(&mut app, behind_cell);
    set_fire_mode(&mut app, mode);
    set_hovered(&mut app, Some(behind_cell));
    app.world_mut().entity_mut(shooter).insert(Tu::new(1));

    let tuning = app.world().resource::<CombatTuning>();
    let expected_cost = fire_arc_tu_cost(
        *SHOOTER_FACING,
        shooter_cell.cell(),
        behind_cell.cell(),
        mode_tu_cost(&mode, &tu_max, &aiming, tuning),
        tuning,
    );

    app.update();

    let h = highlight(&app);
    assert!(
        *expected_cost > 1,
        "precondition: the quoted total is more TU than the shooter has left ({expected_cost:?} \
         vs 1)",
    );
    assert!(
        !h.is_empty(),
        "a shooter who cannot afford the total still gets a highlight — the label shows the price",
    );
    assert_eq!(
        h.cost(),
        Some(expected_cost),
        "the unaffordable highlight still quotes the full fire_arc_tu_cost total",
    );
}
