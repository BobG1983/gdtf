use gdtf_battle_sim::{
    acts::fire_arc_tu_cost,
    magazine::mode_tu_cost,
    prelude::{Cell, CellLevel, Tu},
    tuning::CombatTuning,
    weapon::FireMode,
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

    let Some(resolved_mode) = resolved_mode(&app) else {
        unreachable!("the shooter has been armed, so its gun resolves to a mode");
    };
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
        "the highlight cost EXACTLY equals fire_arc_tu_cost over the mode the gun resolves to",
    );
    assert!(
        h.cost().is_some_and(|cost| *cost > 0),
        "the resolved fire cost is strictly positive for a normal weapon/mode (not 0 TU); got {:?}",
        h.cost(),
    );
}

#[test]
fn a_gun_rewritten_in_place_is_what_the_highlight_prices() {
    let mut app = fire_target_app();
    let shooter_cell = CellLevel::new(Cell::new(10, 10), LEVEL);
    let enemy_cell = CellLevel::new(Cell::new(13, 11), LEVEL);

    let (_shooter, tu_max, aiming) = spawn_and_select_shooter(&mut app, shooter_cell);
    place_enemy(&mut app, enemy_cell);
    set_fire_mode(&mut app, spec(0.2));
    set_hovered(&mut app, Some(enemy_cell));
    app.update();

    // Same kind, new price, written straight onto the gun the shooter fires.
    let rewritten = spec(0.6);
    let Some(gun) = selected_gun(&app) else {
        unreachable!("the selected shooter holds the gun the harness armed it with");
    };
    app.world_mut()
        .entity_mut(gun)
        .insert(FireMode::new(vec![rewritten]));

    let tuning = app.world().resource::<CombatTuning>();
    let expected_cost = fire_arc_tu_cost(
        *SHOOTER_FACING,
        shooter_cell.cell(),
        enemy_cell.cell(),
        mode_tu_cost(&rewritten, &tu_max, &aiming, tuning),
        tuning,
    );
    let stale_cost = fire_arc_tu_cost(
        *SHOOTER_FACING,
        shooter_cell.cell(),
        enemy_cell.cell(),
        mode_tu_cost(&spec(0.2), &tu_max, &aiming, tuning),
        tuning,
    );
    assert_ne!(
        expected_cost, stale_cost,
        "the rewrite has to change the price, or the case cannot tell which entry was read",
    );
    assert_eq!(
        highlight(&app).cost(),
        Some(stale_cost),
        "the highlight was drawn at the armed price before the rewrite",
    );

    set_hovered(&mut app, Some(enemy_cell));
    app.update();

    assert_eq!(
        highlight(&app).cost(),
        Some(expected_cost),
        "the highlight prices the gun's own entry, so the old price here means it was read off a \
         copy taken before the rewrite",
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
