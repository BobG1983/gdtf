//! AC3 — `FireRequested` dispatch runs `fire()`: a shot at an in-line target mutates the
//! target's battle surfaces (real shot effect); same seed reproduces it. Plus GTW-242 —
//! the firing-arc + turn-to-fire gate layered on the fire dispatch. Value-agnostic on
//! every magnitude: the assertions are RELATIONS (the TU drop equals the fire cost in-arc,
//! the turn cost plus the fire cost out-of-arc, and is UNCHANGED on reject), never pinned.

use super::support::*;

#[test]
fn fire_dispatch_runs_fire_and_mutates_the_target() {
    let (mut app, shooter, target) = fire_scenario();
    let mode = single_mode(0.2, 1);
    let hp_before = app.world().get::<Hp>(target).copied();
    let tu_before = app.world().get::<Tu>(shooter).copied();

    app.world_mut().write_message(FireRequested::new(
        shooter,
        mode,
        Cell::new(8, 5),
        Level::new(0),
    ));
    app.update();

    let hp_after = app.world().get::<Hp>(target).copied();
    let wounds_after = app.world().get::<Wounds>(target).map(|w| **w);
    let life_after = app.world().get::<LifeState>(target).copied();
    let tu_after = app.world().get::<Tu>(shooter).copied();

    // The verb RAN: either the target's battle surfaces changed (a real shot effect)
    // OR (seed-dependent miss) the shooter's Tu strictly decreased (the charge). Never
    // a pinned magnitude.
    let target_changed =
        hp_after != hp_before || wounds_after != Some(6) || life_after != Some(LifeState::Alive);
    let tu_dropped = matches!((tu_before, tu_after), (Some(b), Some(a)) if *a < *b);
    assert!(
        target_changed || tu_dropped,
        "fire dispatch must run the verb — target surfaces changed or the shooter's \
         Tu dropped (hp {hp_before:?}->{hp_after:?}, tu {tu_before:?}->{tu_after:?})",
    );
}

/// Drain the buffered [`ShotFired`] messages emitted this run, in order. `drain`
/// empties the buffer, so the test runs ONE `update()` then probes (the
/// `drain_battle_ready` precedent).
fn drain_shots_fired(app: &mut App) -> Vec<ShotFired> {
    app.world_mut()
        .resource_mut::<Messages<ShotFired>>()
        .drain()
        .collect()
}

/// Drain the buffered [`FireDeclaration`] combat-log messages emitted this run (GTW-328).
fn drain_fire_declarations(app: &mut App) -> Vec<FireDeclaration> {
    app.world_mut()
        .resource_mut::<Messages<FireDeclaration>>()
        .drain()
        .collect()
}

// GTW-328 — a proceeding shot emits exactly ONE FireDeclaration carrying the shooter, the
// resolved target occupant (the in-line target placed at the aimed cell), and the request's
// mode kind. One declaration even for a multi-round burst is asserted separately; here a
// single shot proves the payload wiring.
#[test]
fn fire_dispatch_emits_one_fire_declaration_with_shooter_target_and_mode() {
    let (mut app, shooter, target) = fire_scenario();

    app.world_mut().write_message(FireRequested::new(
        shooter,
        single_mode(0.2, 1),
        Cell::new(8, 5),
        Level::new(0),
    ));
    app.update();

    let declarations = drain_fire_declarations(&mut app);
    assert_eq!(
        declarations.len(),
        1,
        "a proceeding fire emits exactly one FireDeclaration: {declarations:?}",
    );
    let Some(decl) = declarations.first() else {
        return;
    };
    assert_eq!(
        decl.shooter, shooter,
        "the declaration carries the firing shooter entity",
    );
    assert_eq!(
        decl.target,
        Some(target),
        "the declaration resolves the target occupant standing at the aimed cell",
    );
    assert_eq!(
        decl.mode,
        ModeKind::Single,
        "the declaration carries the request's fire-mode kind",
    );
}

// GTW-328 — a BURST volley declares the shot ONCE (the log announces "fired a burst" a
// single time) even though it fires multiple rounds (multiple ShotFired). This pins the
// one-declaration-per-request contract distinct from the per-round outcome signal.
#[test]
fn fire_dispatch_emits_one_declaration_per_request_even_for_a_burst() {
    let (mut app, shooter, _target) = fire_scenario();
    let burst = FireModeSpec::new(
        ModeKind::Burst,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.3),
        ModeShots::new(3),
    );

    app.world_mut().write_message(FireRequested::new(
        shooter,
        burst,
        Cell::new(8, 5),
        Level::new(0),
    ));
    app.update();

    let declarations = drain_fire_declarations(&mut app);
    assert_eq!(
        declarations.len(),
        1,
        "a burst declares the shot ONCE per fire request (not per round): {declarations:?}",
    );
    assert_eq!(
        declarations.first().map(|d| d.mode),
        Some(ModeKind::Burst),
        "the declaration carries the burst mode kind",
    );
    // Cross-check: the per-round outcome signal still fires three times (3 rounds).
    assert_eq!(
        drain_shots_fired(&mut app).len(),
        3,
        "the burst still emits one ShotFired per round (3 rounds)",
    );
}

// GTW-328 — a shot aimed at an EMPTY cell (no occupant placed) declares with target=None:
// the log says "fired at <cell>" with no target name. The shooter fires up an empty column.
#[test]
fn fire_dispatch_declaration_target_is_none_for_an_empty_cell() {
    let mut app = headless_app();
    let shooter = spawn_shooter(app.world_mut(), 2, 5, single_mode(0.2, 1), false);

    app.world_mut().write_message(FireRequested::new(
        shooter,
        single_mode(0.2, 1),
        Cell::new(8, 5),
        Level::new(0),
    ));
    app.update();

    let declarations = drain_fire_declarations(&mut app);
    assert_eq!(declarations.len(), 1, "the shot still declares once");
    assert_eq!(
        declarations.first().and_then(|d| d.target),
        None,
        "with no occupant at the aimed cell, the declaration's target is None",
    );
}

// GTW-328 — a REJECTED out-of-arc shot (cannot afford turn + fire) emits NO FireDeclaration:
// the log only announces shots that are actually taken. Mirrors the GTW-242 reject fixture.
#[test]
fn rejected_shot_emits_no_fire_declaration() {
    let mut app = headless_app();
    let Some(fire_cost) = fire_cost_in(&app) else {
        assert!(
            app.world().get_resource::<CombatTuning>().is_some(),
            "tuning present",
        );
        return;
    };
    // fire_cost <= tu < fire_cost + turn_cost: affords the shot, NOT the turn+shot → Reject.
    let tu_start = fire_cost + 1;
    let shooter = spawn_arc_shooter(app.world_mut(), 5, 5, Direction::East, tu_start);
    let _target = place_arc_target(&mut app, 5, 8);

    app.world_mut().write_message(FireRequested::new(
        shooter,
        single_mode(0.2, 1),
        Cell::new(5, 8),
        Level::new(0),
    ));
    app.update();

    assert!(
        drain_fire_declarations(&mut app).is_empty(),
        "a rejected (unaffordable turn+fire) shot is never taken — it declares nothing",
    );
}

// GTW-328 — the FireDeclaration emit adds NO RNG draw: two same-seed dispatches over
// identical worlds produce byte-equal post-fire state. (If the declaration path rolled
// anything, two runs would diverge.) This pins the determinism-preserved clause on the
// real dispatch path, alongside the existing fire-determinism test.
#[test]
fn fire_declaration_emit_preserves_determinism() {
    let snapshot = || {
        let (mut app, shooter, target) = fire_scenario();
        app.world_mut().write_message(FireRequested::new(
            shooter,
            single_mode(0.2, 1),
            Cell::new(8, 5),
            Level::new(0),
        ));
        app.update();
        (
            app.world().get::<Hp>(target).map(|h| **h),
            app.world().get::<Wounds>(target).map(|w| **w),
            app.world().get::<LifeState>(target).copied(),
            app.world().get::<Tu>(shooter).map(|t| **t),
        )
    };
    assert_eq!(
        snapshot(),
        snapshot(),
        "the FireDeclaration emit must add no RNG draw — same seed reproduces the state",
    );
}

// GTW-290 — a single in-arc shot emits ONE ShotFired sourced from the round's
// already-computed ShotOutcome: the shooter ref, a muzzle at the shooter's cell, an
// impact at the target cell, and a Ganger kind. Geometry is asserted by RELATION (the
// muzzle/impact cells), not by pinned f32 magnitudes.
#[test]
fn fire_dispatch_emits_one_shot_fired_per_round_with_geometry() {
    // The fire_scenario shooter sits at (2,5) facing East; the HIGH-band target is in
    // line at (8,5,0) — an in-arc, one-round (single_mode shots=1) shot.
    let (mut app, shooter, target) = fire_scenario();

    app.world_mut().write_message(FireRequested::new(
        shooter,
        single_mode(0.2, 1),
        Cell::new(8, 5),
        Level::new(0),
    ));
    app.update();

    let shots = drain_shots_fired(&mut app);
    assert_eq!(
        shots.len(),
        1,
        "a single-shot fire emits exactly one ShotFired (one round → one tracer)",
    );
    let Some(shot) = shots.first() else {
        return;
    };
    assert_eq!(
        shot.shooter, shooter,
        "the ShotFired must carry the firing shooter entity",
    );
    assert_eq!(
        shot.kind,
        ShotKind::Ganger(target),
        "the in-line shot struck the target ganger — the kind rides off the ShotOutcome",
    );
    // The muzzle is the shooter's 3D fire origin — it floors to the shooter's own cell
    // (2,5). pos_to_cell is the sim-unit floor (no brittle f32 cast), a unit-kind relation.
    let (muzzle_cell, _muzzle_level) = crate::metric::pos_to_cell(shot.muzzle);
    assert_eq!(
        muzzle_cell,
        Cell::new(2, 5),
        "the muzzle sits at the shooter's cell (sourced from ShotOutcome.muzzle)",
    );
    // The impact (cell, level) is the resolved outcome cell — the in-line target cell.
    assert_eq!(
        (shot.impact_cell, shot.impact_level),
        (Cell::new(8, 5), Level::new(0)),
        "the impact cell is the resolved outcome cell (the in-line target)",
    );
    // The trajectory is a non-degenerate sim-unit direction (the cone draw's vector).
    assert!(
        shot.trajectory.vec().length_squared() > 0.0,
        "the trajectory is the round's sampled non-zero unit direction",
    );
}

// GTW-306 — the emitted ShotFired carries the FIRING WEAPON'S DamageType, sourced from
// the shooter's own DamageType component at fire time (pure exposure, no fire-result
// change). To prove the value FLOWS FROM the live component (not a hardcoded constant or a
// default), the shooter's weapon DamageType is OVERRIDDEN to a non-default node (Plasma)
// before the shot, then the emitted message is asserted to carry exactly that node.
#[test]
fn fire_dispatch_shot_fired_carries_the_weapon_damage_type() {
    let (mut app, shooter, _target) = fire_scenario();
    // Override the wielded weapon's damage type to a node distinct from the fixture's
    // default (Kinetic) — so a passing assert can only mean the type was read off the
    // weapon ENTITY's component (GTW-323 slice 2: `ganger → Wields → the weapon
    // entity`), not a constant baked into the emit. Resolve the weapon entity through
    // the relationship, then override its `DamageType`.
    let weapon = app.world().get::<Wields>(shooter).and_then(Wields::weapon);
    if let Some(weapon) = weapon
        && let Ok(mut entity) = app.world_mut().get_entity_mut(weapon)
    {
        entity.insert(DamageType::Plasma);
    }

    app.world_mut().write_message(FireRequested::new(
        shooter,
        single_mode(0.2, 1),
        Cell::new(8, 5),
        Level::new(0),
    ));
    app.update();

    let shots = drain_shots_fired(&mut app);
    assert_eq!(
        shots.len(),
        1,
        "a single-shot fire emits exactly one ShotFired",
    );
    let Some(shot) = shots.first() else {
        return;
    };
    assert_eq!(
        shot.damage,
        DamageType::Plasma,
        "the ShotFired must carry the firing weapon's DamageType (sourced from the \
         shooter's component at fire time), not a constant: {shot:?}",
    );
}

// GTW-290 — a BURST volley emits one ShotFired PER ROUND (so the presenter draws a
// tracer per round). A 3-shot mode over a 10-round magazine fires 3 rounds → 3 messages.
#[test]
fn fire_dispatch_emits_one_shot_fired_per_burst_round() {
    let (mut app, shooter, _target) = fire_scenario();
    // A 3-round burst (the spawn_shooter magazine loads 10, so all 3 fire).
    let burst = FireModeSpec::new(
        ModeKind::Burst,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.3),
        ModeShots::new(3),
    );

    app.world_mut().write_message(FireRequested::new(
        shooter,
        burst,
        Cell::new(8, 5),
        Level::new(0),
    ));
    app.update();

    let shots = drain_shots_fired(&mut app);
    assert_eq!(
        shots.len(),
        3,
        "a 3-round burst emits one ShotFired per round (3 tracers): {shots:?}",
    );
    assert!(
        shots.iter().all(|s| s.shooter == shooter),
        "every burst round's ShotFired carries the same firing shooter",
    );
}

// GTW-290 — a fail-closed fire (a DOWNED shooter never fires) emits NO ShotFired: the
// empty volley has no rounds, so the per-round emit loop writes nothing.
#[test]
fn fail_closed_fire_emits_no_shot_fired() {
    let (mut app, shooter, _target) = fire_scenario();
    // Down the shooter so can_fire fails inside fire() → empty volley, no rounds.
    if let Ok(mut entity) = app.world_mut().get_entity_mut(shooter) {
        entity.insert(LifeState::Downed);
    }

    app.world_mut().write_message(FireRequested::new(
        shooter,
        single_mode(0.2, 1),
        Cell::new(8, 5),
        Level::new(0),
    ));
    app.update();

    let shots = drain_shots_fired(&mut app);
    assert!(
        shots.is_empty(),
        "a fail-closed (downed-shooter) fire resolves no round, so emits no ShotFired",
    );
}

// GTW-302 — a LANDED ganger hit carries the round's HitReport on the ShotFired: the
// report is Some, its kind is the SAME ShotKind the message kind carries (proving the
// report rides the SAME resolved round, not a re-derivation), and — because the round
// struck the target ganger — the report's applied-damage block is present with the struck
// part. The fire_scenario in-line target at (8,5) is struck on a single round; the assert
// is value-agnostic on the severity MAGNITUDE (seed/tuning dependent), pinning only the
// STRUCTURE the FCT presenter reads.
#[test]
fn landed_ganger_hit_carries_some_report_matching_the_round() {
    let (mut app, shooter, target) = fire_scenario();

    app.world_mut().write_message(FireRequested::new(
        shooter,
        single_mode(0.2, 1),
        Cell::new(8, 5),
        Level::new(0),
    ));
    app.update();

    let shots = drain_shots_fired(&mut app);
    assert_eq!(shots.len(), 1, "a single-shot fire emits one ShotFired");
    let Some(shot) = shots.first() else {
        return;
    };
    // The in-line round struck the target ganger — so the message kind is Ganger(target).
    assert_eq!(
        shot.kind,
        ShotKind::Ganger(target),
        "the in-line shot struck the target ganger",
    );
    // GTW-302: the report is carried (Some) — never dropped.
    assert!(
        shot.report.is_some(),
        "a fired round must carry its parallel HitReport (Some), got None: {shot:?}",
    );
    let Some(report) = shot.report.clone() else {
        return;
    };
    // The report rides the SAME resolved round: its kind equals the message's kind
    // (both copied from the round's ShotOutcome — pure exposure, no recompute).
    assert_eq!(
        report.kind, shot.kind,
        "the report's kind must equal the round's kind (same resolved round, no recompute)",
    );
    // A Ganger hit carries the applied-damage block AND the struck part (the FCT reads
    // the severity tier, HP damage, life-after, and armor-broken from `applied`; the
    // struck part from `part`).
    assert!(
        report.applied.is_some(),
        "a ganger hit must carry the applied-damage block (Some): {report:?}",
    );
    assert!(
        report.part.is_some(),
        "a ganger hit must carry the struck body part (Some): {report:?}",
    );
}

// GTW-302 — a CLEAN MISS (no ganger in the round's path) carries a report that REFLECTS
// the miss: the report is Some (never dropped), its kind is a NON-ganger ShotKind, and its
// applied-damage block is None (a non-ganger outcome wounds nothing — resolve_and_apply's
// kind gate). The shooter fires up an EMPTY column (no occupant placed), so no round can
// strike a ganger.
#[test]
fn clean_miss_carries_some_report_with_no_applied_damage() {
    let mut app = headless_app();
    // An armed shooter at (2,5) facing East, NO target placed anywhere in the grid — so
    // the in-arc round down the empty column strikes ground / a slab / nothing, never a
    // ganger.
    let shooter = spawn_shooter(app.world_mut(), 2, 5, single_mode(0.2, 1), false);

    app.world_mut().write_message(FireRequested::new(
        shooter,
        single_mode(0.2, 1),
        Cell::new(8, 5),
        Level::new(0),
    ));
    app.update();

    let shots = drain_shots_fired(&mut app);
    assert_eq!(shots.len(), 1, "a single-shot fire emits one ShotFired");
    let Some(shot) = shots.first() else {
        return;
    };
    // No occupant anywhere → the round never struck a ganger.
    assert!(
        !matches!(shot.kind, ShotKind::Ganger(_)),
        "with no occupant, the round must NOT strike a ganger: {:?}",
        shot.kind,
    );
    // GTW-302: the report is still carried (Some) — a miss is a first-class FCT event.
    assert!(
        shot.report.is_some(),
        "even a clean miss carries its parallel HitReport (Some), got None: {shot:?}",
    );
    let Some(report) = shot.report.clone() else {
        return;
    };
    // The report reflects the miss: same non-ganger kind as the message, and NO applied
    // damage (resolve_and_apply folds a non-ganger round to a no-effect report).
    assert_eq!(
        report.kind, shot.kind,
        "the miss report's kind must equal the round's non-ganger kind (no recompute)",
    );
    assert!(
        report.applied.is_none(),
        "a non-ganger (miss) round applies no damage — its report's applied is None: {report:?}",
    );
    assert!(
        report.part.is_none(),
        "a non-ganger (miss) round struck no body part — its report's part is None: {report:?}",
    );
}

// GTW-302 — PURE EXPOSURE, not a recompute: the report carried on the ShotFired equals the
// report a same-seed dispatch over an identical world produces (byte-equal). If any
// recompute (a fresh draw) crept into the message path, two same-seed runs would diverge.
// This pins the "equals the volley's already-computed report" clause via the determinism
// property (AC7) on the REAL dispatch path.
#[test]
fn shot_fired_report_is_deterministic_for_the_same_seed() {
    let dispatched_report = || {
        let (mut app, shooter, _target) = fire_scenario();
        app.world_mut().write_message(FireRequested::new(
            shooter,
            single_mode(0.2, 1),
            Cell::new(8, 5),
            Level::new(0),
        ));
        app.update();
        drain_shots_fired(&mut app)
            .first()
            .and_then(|s| s.report.clone())
    };
    assert_eq!(
        dispatched_report(),
        dispatched_report(),
        "the same BattleSeed must reproduce the SAME ShotFired.report — the report is \
         exposed from the volley, never recomputed on the message path",
    );
}

#[test]
fn fire_dispatch_is_deterministic_for_the_same_seed() {
    let snapshot = |seed_run: u64| {
        // The seed is fixed by insert_sim_resources; seed_run only labels the call.
        let _ = seed_run;
        let (mut app, shooter, target) = fire_scenario();
        let mode = single_mode(0.2, 1);
        app.world_mut().write_message(FireRequested::new(
            shooter,
            mode,
            Cell::new(8, 5),
            Level::new(0),
        ));
        app.update();
        (
            app.world().get::<Hp>(target).map(|h| **h),
            app.world().get::<Wounds>(target).map(|w| **w),
            app.world().get::<LifeState>(target).copied(),
            app.world().get::<Tu>(shooter).map(|t| **t),
        )
    };
    assert_eq!(
        snapshot(0),
        snapshot(1),
        "the same BattleSeed must reproduce the same post-fire state via dispatch",
    );
}

/// Spawn an armed shooter facing `facing` at `(x, y, 0)` with a given starting `tu`
/// pool — the GTW-242 fixture (reuses `spawn_shooter`, then overrides the facing + the
/// pool so a test can place the shooter at the boundary of an affordability gate). The
/// shooter is NOT aiming (hip-fire) so the fire cost has no aim premium muddying the
/// relation. Returns the shooter [`Entity`].
fn spawn_arc_shooter(world: &mut World, x: i32, y: i32, facing: Direction, tu: u8) -> Entity {
    let mode = single_mode(0.2, 1);
    let shooter = spawn_shooter(world, x, y, mode, false);
    if let Ok(mut entity) = world.get_entity_mut(shooter) {
        entity.insert((Facing::new(facing), Tu::new(tu)));
    }
    shooter
}

/// Place a HIGH-band ganger target at `(x, y, 0)` in the occupancy grid + spawn its
/// battle-state bundle — the GTW-242 target fixture (mirrors `fire_scenario`).
fn place_arc_target(app: &mut App, x: i32, y: i32) -> Entity {
    let target = app.world_mut().spawn(target_bundle(30, 6)).id();
    let at = CellLevel::new(Cell::new(x, y), Level::new(0));
    if let Some(mut grid) = app.world_mut().get_resource_mut::<OccupancyGrid>() {
        grid.set_occupant(at, Some(target));
        grid.set_occupant_band(at, Some(HeightBand::High));
    }
    target
}

/// The fire-TU cost a hip-fired `single_mode(0.2, 1)` shot charges under the app's
/// current [`CombatTuning`] (the same `mode_tu_cost` source `fire()` debits) — read from
/// the resource so the relation tracks the tunable value, never a pinned magnitude.
fn fire_cost_in(app: &App) -> Option<u8> {
    let tuning = app.world().get_resource::<CombatTuning>()?;
    let mode = single_mode(0.2, 1);
    Some(*mode_tu_cost(
        &mode,
        &TuMax::new(100),
        &Aiming::new(false),
        tuning,
    ))
}

/// The per-45°-step turn cost off the app's current [`CombatTuning`].
fn turn_tu_in(app: &App) -> Option<u8> {
    app.world()
        .get_resource::<CombatTuning>()
        .map(|t| *t.turn_tu)
}

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
        .map(|(f, t)| Direction::East.steps_to(Direction::South) * t + f);
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
