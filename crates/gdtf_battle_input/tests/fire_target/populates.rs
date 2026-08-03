use gdtf_battle_input::SelectedFireMode;
use gdtf_battle_sim::{
    magazine::mode_tu_cost,
    prelude::{Cell, CellLevel},
    tuning::CombatTuning,
};

use super::harness::*;

#[test]
fn hovering_fireable_enemy_populates_cell_and_mode_tu_cost() {
    let mut app = fire_target_app();
    let shooter_cell = CellLevel::new(Cell::new(10, 10), LEVEL);
    let enemy_cell = CellLevel::new(Cell::new(13, 11), LEVEL);
    let mode = spec(0.2);

    let (_shooter, tu_max, aiming) = spawn_and_select_shooter(&mut app, shooter_cell);
    place_enemy(&mut app, enemy_cell);
    set_fire_mode(&mut app, mode);
    set_hovered(&mut app, Some(enemy_cell));

    let tuning = app.world().resource::<CombatTuning>();
    let expected_cost = mode_tu_cost(&mode, &tu_max, &aiming, tuning);

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
        "the highlight cost EXACTLY equals mode_tu_cost(SelectedFireMode, TuMax, Aiming, tuning)",
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
    let expected_cost = mode_tu_cost(&resolved_mode, &tu_max, &aiming, tuning);

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
        "the highlight cost EXACTLY equals mode_tu_cost(resolved SelectedFireMode, TuMax, \
         Aiming, tuning)",
    );
    assert!(
        h.cost().is_some_and(|cost| *cost > 0),
        "the resolved fire cost is strictly positive for a normal weapon/mode (not 0 TU); got {:?}",
        h.cost(),
    );
}


#[test]
fn hovering_shootable_cover_populates_cell_and_mode_tu_cost() {
    let mut app = fire_target_app();
    let shooter_cell = CellLevel::new(Cell::new(10, 10), LEVEL);
    let cover_cell = CellLevel::new(Cell::new(13, 11), LEVEL);
    let mode = spec(0.2);

    let (_shooter, tu_max, aiming) = spawn_and_select_shooter(&mut app, shooter_cell);
    place_cover(&mut app, cover_cell);
    set_fire_mode(&mut app, mode);
    set_hovered(&mut app, Some(cover_cell));

    let tuning = app.world().resource::<CombatTuning>();
    let expected_cost = mode_tu_cost(&mode, &tu_max, &aiming, tuning);

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
        "the cover-target highlight cost EXACTLY equals mode_tu_cost(SelectedFireMode, TuMax, \
         Aiming, tuning) — same TU-cost treatment as a ganger target (C2)",
    );
}
