//! GTW-242 — the firing-arc + turn-to-fire gate layered on the fire dispatch.

use super::support::*;

// GTW-242 AC1 — an IN-ARC shot (target dead ahead) spends only the fire TU and leaves
// the facing unchanged. Shooter East at (5,5), target East at (8,5) — 0° off-axis.
#[test]
fn in_arc_shot_spends_only_fire_tu_and_keeps_facing() {
    let mut app = headless_app();
    let shooter = spawn_arc_shooter(app.world_mut(), 5, 5, Direction::East, 200);
    let _target = place_arc_target(&mut app, 8, 5);
    let fire_cost = fire_cost_in(&app);
    let tu_before = app.world().get::<Tu>(shooter).map(|t| **t);

    app.world_mut().write_message(FireRequested::new(
        shooter,
        single_mode(0.2, 1),
        Cell::new(8, 5),
        Level::new(0),
    ));
    app.update();

    let facing_after = app.world().get::<Facing>(shooter).map(|f| **f);
    let tu_after = app.world().get::<Tu>(shooter).map(|t| **t);
    assert_eq!(
        facing_after,
        Some(Direction::East),
        "an in-arc shot must NOT change the facing",
    );
    assert_eq!(
        tu_before.zip(tu_after).map(|(b, a)| b - a),
        fire_cost,
        "an in-arc shot drops TU by exactly the fire cost (no turn cost)",
    );
}

// GTW-242 AC2 — an OUT-OF-ARC shot with enough TU turns to face the target THEN fires:
// the facing becomes from_cells(actor, target) and TU drops by exactly turn + fire.
// Shooter East at (5,5), target South at (5,8) — 90° off a 120° (±60°) arc.
#[test]
fn out_of_arc_with_enough_tu_turns_then_fires() {
    let mut app = headless_app();
    let shooter = spawn_arc_shooter(app.world_mut(), 5, 5, Direction::East, 200);
    let _target = place_arc_target(&mut app, 5, 8);
    let fire_cost = fire_cost_in(&app);
    let turn_tu = turn_tu_in(&app);
    // The expected combined drop: steps_to(East -> South) * turn_tu + fire_cost.
    let expected_drop = fire_cost
        .zip(turn_tu)
        .map(|(f, t)| *Direction::East.steps_to(Direction::South) * t + f);
    let tu_before = app.world().get::<Tu>(shooter).map(|t| **t);

    app.world_mut().write_message(FireRequested::new(
        shooter,
        single_mode(0.2, 1),
        Cell::new(5, 8),
        Level::new(0),
    ));
    app.update();

    let facing_after = app.world().get::<Facing>(shooter).map(|f| **f);
    let tu_after = app.world().get::<Tu>(shooter).map(|t| **t);
    assert_eq!(
        facing_after,
        Direction::from_cells(Cell::new(5, 5), Cell::new(5, 8)),
        "an out-of-arc shot must turn to face from_cells(actor, target) (South)",
    );
    assert_eq!(
        tu_before.zip(tu_after).map(|(b, a)| b - a),
        expected_drop,
        "an out-of-arc shot drops TU by exactly turn_cost + fire_cost",
    );
}

// GTW-242 AC3 (the crux) — an OUT-OF-ARC shot that can afford the FIRE but NOT
// turn + fire is REJECTED: no TU spent, no facing change, no shot. Same geometry as
// AC2; the pool is set to fire_cost + 1 < fire_cost + turn_cost (turn_cost == 2 here).
#[test]
fn out_of_arc_unaffordable_turn_is_rejected_no_spend_no_turn() {
    let mut app = headless_app();
    // The fire cost (the same value `fire()` debits) decides where the affordability
    // gap sits. Read it before spawning the shooter at the boundary inside it.
    let Some(fire_cost) = fire_cost_in(&app) else {
        // CombatTuning is always inserted by headless_app; the None arm is unreachable,
        // but a test must not unwrap/expect/panic — assert the precondition holds.
        assert!(
            app.world().get_resource::<CombatTuning>().is_some(),
            "tuning present",
        );
        return;
    };
    // fire_cost <= tu < fire_cost + turn_cost: afford the shot, NOT the turn+shot.
    // turn_cost (East -> South) == steps_to(2) * turn_tu(1) == 2, so fire_cost + 1 sits
    // strictly inside the gap.
    let tu_start = fire_cost + 1;
    let shooter = spawn_arc_shooter(app.world_mut(), 5, 5, Direction::East, tu_start);
    let target = place_arc_target(&mut app, 5, 8);
    let hp_before = app.world().get::<Hp>(target).map(|h| **h);
    let life_before = app.world().get::<LifeState>(target).copied();

    app.world_mut().write_message(FireRequested::new(
        shooter,
        single_mode(0.2, 1),
        Cell::new(5, 8),
        Level::new(0),
    ));
    app.update();

    let facing_after = app.world().get::<Facing>(shooter).map(|f| **f);
    let tu_after = app.world().get::<Tu>(shooter).map(|t| **t);
    let hp_after = app.world().get::<Hp>(target).map(|h| **h);
    let life_after = app.world().get::<LifeState>(target).copied();
    assert_eq!(
        facing_after,
        Some(Direction::East),
        "a rejected shot must NOT change the facing",
    );
    assert_eq!(
        tu_after,
        Some(tu_start),
        "a rejected shot must spend NO TU (pool unchanged)",
    );
    assert_eq!(
        (hp_after, life_after),
        (hp_before, life_before),
        "a rejected shot must resolve NO shot (target surfaces unchanged)",
    );
}

// GTW-242 AC5 — the arc is DATA-DRIVEN. A WIDE arc (360°) makes the (5,8) target in-arc
// (no turn — facing unchanged, only fire_cost); a NARROW arc forces an otherwise-in-arc
// off-axis target out-of-arc (now turns). Mutate CombatTuning before the request.
#[test]
fn arc_is_data_driven_wide_never_turns_narrow_forces_turn() {
    // (a) WIDE arc 360° — the 90°-off (5,8) target is in-arc: no turn, only fire_cost.
    let mut wide = headless_app();
    if let Some(mut t) = wide.world_mut().get_resource_mut::<CombatTuning>() {
        t.firing_arc = crate::tuning::FiringArc::new(360.0);
    }
    let s_wide = spawn_arc_shooter(wide.world_mut(), 5, 5, Direction::East, 200);
    let _t_wide = place_arc_target(&mut wide, 5, 8);
    let fire_cost = fire_cost_in(&wide);
    let tu_before = wide.world().get::<Tu>(s_wide).map(|t| **t);
    wide.world_mut().write_message(FireRequested::new(
        s_wide,
        single_mode(0.2, 1),
        Cell::new(5, 8),
        Level::new(0),
    ));
    wide.update();
    assert_eq!(
        wide.world().get::<Facing>(s_wide).map(|f| **f),
        Some(Direction::East),
        "a 360° arc makes every target in-arc — the facing must NOT change",
    );
    assert_eq!(
        tu_before
            .zip(wide.world().get::<Tu>(s_wide).map(|t| **t))
            .map(|(b, a)| b - a),
        fire_cost,
        "a 360° in-arc shot drops only the fire cost (no turn)",
    );

    // (b) NARROW arc 10° — a near-on-axis target (8,6, ~18° off East) is now OUT-of-arc
    // and must turn (facing changes to SouthEast = from_cells((5,5),(8,6))).
    let mut narrow = headless_app();
    if let Some(mut t) = narrow.world_mut().get_resource_mut::<CombatTuning>() {
        t.firing_arc = crate::tuning::FiringArc::new(10.0);
    }
    let s_narrow = spawn_arc_shooter(narrow.world_mut(), 5, 5, Direction::East, 200);
    let _t_narrow = place_arc_target(&mut narrow, 8, 6);
    narrow.world_mut().write_message(FireRequested::new(
        s_narrow,
        single_mode(0.2, 1),
        Cell::new(8, 6),
        Level::new(0),
    ));
    narrow.update();
    assert_eq!(
        narrow.world().get::<Facing>(s_narrow).map(|f| **f),
        Direction::from_cells(Cell::new(5, 5), Cell::new(8, 6)),
        "a narrow arc forces an off-axis target out-of-arc — the shooter must turn",
    );
}

// GTW-242 AC6 — a CO-LOCATED target (the shooter's own cell, zero vector) is in-arc: it
// spends only the fire cost, leaves the facing unchanged, and never panics / NaNs.
#[test]
fn co_located_target_is_in_arc_only_fire_cost_no_panic() {
    let mut app = headless_app();
    let shooter = spawn_arc_shooter(app.world_mut(), 5, 5, Direction::North, 200);
    let _target = place_arc_target(&mut app, 5, 5);
    let fire_cost = fire_cost_in(&app);
    let tu_before = app.world().get::<Tu>(shooter).map(|t| **t);

    app.world_mut().write_message(FireRequested::new(
        shooter,
        single_mode(0.2, 1),
        Cell::new(5, 5),
        Level::new(0),
    ));
    app.update();

    assert_eq!(
        app.world().get::<Facing>(shooter).map(|f| **f),
        Some(Direction::North),
        "a co-located target needs no turn — the facing must NOT change",
    );
    assert_eq!(
        tu_before
            .zip(app.world().get::<Tu>(shooter).map(|t| **t))
            .map(|(b, a)| b - a),
        fire_cost,
        "a co-located in-arc shot drops only the fire cost (no turn, no NaN)",
    );
}
