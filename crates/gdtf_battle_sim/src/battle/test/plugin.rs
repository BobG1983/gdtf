//! Tests for the [`BattleSimPlugin`](crate::battle::BattleSimPlugin) wiring: the
//! bundled runtime + lifecycle-message registration, the
//! [`BattleInProgress`](crate::battle::BattleInProgress)-gated `Simulate` band
//! staying inert pre-battle / post-teardown, and the no-trigger empty-input
//! invariant.

use super::support::*;

// === AC1 — BattleSimPlugin bundles the runtime + registers the three lifecycle
// messages; bundled-plugin buffers are present; no double-add across update(). ===

#[test]
fn plugin_bundles_runtime_and_registers_lifecycle_messages() {
    let mut app = headless_app();
    // One update proves no double-add panic across the bundled add_message calls.
    app.update();

    let world = app.world();
    // The three lifecycle buffers BattleSimPlugin registers.
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
    // The bundled OccupancyMaintenancePlugin's buffer (proves it is bundled).
    assert!(
        world.get_resource::<Messages<CoverDestroyed>>().is_some(),
        "BattleSimPlugin must bundle OccupancyMaintenancePlugin (CoverDestroyed buffer)",
    );
    // A representative SimActsPlugin buffer (proves it is bundled).
    assert!(
        world.get_resource::<Messages<FireRequested>>().is_some(),
        "BattleSimPlugin must bundle SimActsPlugin (FireRequested buffer)",
    );
}

// === GTW-212 AC2 — the Simulate band is gated on BattleInProgress, not
// OccupancyGrid: the bundled runtime does NOT panic pre-battle / post-teardown (a
// missing non-Option Res would panic), runs only while the witness is present, and
// the gate's run_if references BattleInProgress (not OccupancyGrid). ===

#[test]
fn bundled_runtime_is_inert_pre_battle_and_post_teardown() {
    let mut app = headless_app();

    // Pre-battle: no witness, so the gated band is skipped — updating must not panic
    // even though the bundled in-set systems take non-Option battle-lifetime Res.
    for _ in 0..3 {
        app.update();
    }
    assert!(
        app.world().get_resource::<BattleInProgress>().is_none(),
        "no setup means no BattleInProgress, so the Simulate band is skipped",
    );

    // Set the battle up — the witness now exists, so the band is live (and an
    // emitted FireRequested is consumed without panic).
    app.world_mut().write_message(SetupBattleRequested::new(
        two_ganger_situation(),
        BattleSeed::new(SEED),
    ));
    app.update();
    assert!(
        app.world().get_resource::<BattleInProgress>().is_some(),
        "a successful setup turns the gate on",
    );
    // An arbitrary actor entity + a valid single-shot mode spec — the gated
    // dispatch must consume the message without panic (the unarmed actor simply
    // fails the fire guard; the point is the LIVE band runs and stays panic-free).
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

    // Tear it down — the witness is gone, the band is skipped again, and further
    // updates do not panic.
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

/// AC2 structure check — the gate's `run_if` references the `BattleInProgress`
/// witness and NOT `OccupancyGrid`. A source-scan over THIS module's text proves the
/// witness swap landed in the `configure_sets` call (the externally invisible wiring
/// the AC2 grep check asks for), keyed on the literal `run_if(resource_exists::<…>)`
/// fragments so a regression back to the `OccupancyGrid` proxy turns this red.
#[test]
fn simulate_gate_run_if_references_battle_in_progress_not_occupancy_grid() {
    let source = include_str!("../plugin.rs");
    // The witness fragment is assembled so this assertion is not its own self-match.
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

// === A no-trigger run is inert: with no lifecycle message emitted, neither
// system mutates the world (the empty-input invariant). ===

#[test]
fn no_lifecycle_message_means_no_setup_or_teardown() {
    let mut app = headless_app();
    for _ in 0..3 {
        app.update();
    }
    assert!(
        app.world().get_resource::<SimRng>().is_none(),
        "no SetupBattleRequested means no SimRng inserted",
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
