use super::support::*;

// === AC1 — BattleSimPlugin bundles the runtime + registers the three lifecycle
// messages; bundled-plugin buffers are present; no double-add across update(). ===

#[test]
fn plugin_bundles_runtime_and_registers_lifecycle_messages() {
    let mut app = headless_app();
    app.update();

    let world = app.world();
    assert!(
        world
            .get_resource::<Messages<SetupBattleRequested>>()
            .is_some(),
        "BattleSimPlugin must register the SetupBattleRequested buffer",
    );
    assert!(
        world
            .get_resource::<Messages<TeardownBattleRequested>>()
            .is_some(),
        "BattleSimPlugin must register the TeardownBattleRequested buffer",
    );
    assert!(
        world.get_resource::<Messages<BattleReady>>().is_some(),
        "BattleSimPlugin must register the BattleReady buffer",
    );
    assert!(
        world
            .get_resource::<Messages<TerrainPieceDestroyed>>()
            .is_some(),
        "BattleSimPlugin must bundle OccupancyMaintenancePlugin (TerrainPieceDestroyed buffer)",
    );
    assert!(
        world.get_resource::<Messages<FireRequested>>().is_some(),
        "BattleSimPlugin must bundle SimActsPlugin (FireRequested buffer)",
    );
}

#[test]
fn bundled_runtime_is_inert_pre_battle_and_post_teardown() {
    let mut app = headless_app();

    for _ in 0..3 {
        app.update();
    }
    assert!(
        app.world().get_resource::<BattleInProgress>().is_none(),
        "no setup means no BattleInProgress, so the Simulate band is skipped",
    );

    app.world_mut().write_message(SetupBattleRequested::new(
        two_ganger_situation(),
        BattleSeed::new(SEED),
    ));
    app.update();
    assert!(
        app.world().get_resource::<BattleInProgress>().is_some(),
        "a successful setup turns the gate on",
    );
    let actor = app.world_mut().spawn_empty().id();
    let mode = FireModeSpec::new(
        ModeKind::Single,
        ModeConeMult::new(1.0),
        ModeTuPercent::new(0.2),
        ModeShots::new(1),
    );
    app.world_mut().write_message(FireRequested::new(
        actor,
        mode,
        Cell::new(7, 8),
        Level::new(0),
    ));
    app.update();

    app.world_mut().write_message(TeardownBattleRequested);
    app.update();
    assert!(
        app.world().get_resource::<BattleInProgress>().is_none(),
        "teardown removes the witness, re-closing the gate",
    );
    for _ in 0..3 {
        app.update();
    }
}

#[test]
fn simulate_gate_run_if_references_battle_in_progress_not_occupancy_grid() {
    let source = include_str!("../plugin.rs");
    let witness = concat!("BattleIn", "Progress");
    let stale = concat!("Occupancy", "Grid");
    let gate_on = |ty: &str| format!("run_if(resource_exists::<{ty}>)");
    assert!(
        source.contains(&gate_on(witness)),
        "the Simulate band's run_if must gate on the BattleInProgress witness",
    );
    assert!(
        !source.contains(&gate_on(stale)),
        "the Simulate band must NOT gate on the incidental OccupancyGrid proxy any more",
    );
}

#[test]
fn no_lifecycle_message_means_no_setup_or_teardown() {
    let mut app = headless_app();
    for _ in 0..3 {
        app.update();
    }
    assert!(
        app.world().get_resource::<ShotRng>().is_none(),
        "no SetupBattleRequested means no RNG streams inserted",
    );
    assert!(
        app.world().get_resource::<OccupancyGrid>().is_none(),
        "no SetupBattleRequested means no OccupancyGrid inserted",
    );
    assert_eq!(
        drain_battle_ready(&mut app),
        0,
        "no SetupBattleRequested means no BattleReady signalled",
    );
}
