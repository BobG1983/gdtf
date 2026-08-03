use super::support::*;

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
    assert_eq!(
        drain_shots_fired(&mut app).len(),
        3,
        "the burst still emits one ShotFired per round (3 rounds)",
    );
}

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
