//! GTW-766: the DEV procgen stepper driven over the `net_qa` wire.
//!
//! The FIRST `net_qa` + `dev_tools` + `test-support` integration suite (run with `cargo test
//! -p gdtf_app --features test-support,net_qa,dev_tools --test stepper_control`): it proves a
//! `QaRequest::StepperControl` routed through the REAL `net_qa` router reaches the SAME
//! `PendingStepCommand` / `AutoRunning` latch the egui panel's buttons write, driving the
//! REAL staged procgen — not a shadow copy — and that a request with no live drive is
//! rejected `StepperInactive`.
//!
//! The `#![cfg(all(debug_assertions, feature = "net_qa", feature = "dev_tools"))]` gate
//! (below, after this crate doc so the doc survives a feature-off build — the `net_qa` /
//! `procgen_stepper` suite precedent) compiles the whole file to an empty crate without BOTH
//! features, so the CI static suite and the plain `cargo dtest` (neither enables `net_qa` nor
//! `dev_tools`) never touch it.
#![cfg(all(debug_assertions, feature = "net_qa", feature = "dev_tools"))]

use std::sync::mpsc::{Receiver, Sender};

use bevy::{app::App, prelude::NextState, state::state::State, time::TimeUpdateStrategy};
use gdtf_app::test_support::{
    AppState, AutoStepDelay, BattleScapeState, IncomingRequest, NetQaPlugin, ProcgenStepperPlugin,
    Responder, RunningState,
};
use gdtf_battle_sim::{
    procgen::{ProcgenStage, StagedProcgen},
    rng::BattleSeed,
};
use gdtf_qa_protocol::envelope::{
    AutoRunNet, QaError, QaRequest, QaResponse, StepperCommandNet, StepperReceipt,
};
use gdtf_test_utils::{GdtfLoadTestAppBuilder, GdtfTestAppBuilder, advance_until};

/// A generous budget for the real `DefaultPlugins` async asset loads + the full state descent
/// (the `procgen_stepper` harness precedent).
const BUDGET: u32 = 512;

/// The fixed seed the engaged runs inject, so the staged drive is deterministic.
const FIXED_SEED: u64 = 0xC0FF_EE42;

/// Reads the current [`RunningState`] if it is active.
fn running_state(app: &App) -> Option<RunningState> {
    app.world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get())
}

/// Reads the current [`BattleScapeState`] if it is active.
fn battlescape_state(app: &App) -> Option<BattleScapeState> {
    app.world()
        .get_resource::<State<BattleScapeState>>()
        .map(|state| *state.get())
}

/// The stage the live staged drive is on — `None` when no `StagedProcgen` exists (the stepper
/// is not engaged, or the drive has finished and been cleaned up).
fn driver_stage(app: &App) -> Option<ProcgenStage> {
    app.world()
        .get_resource::<StagedProcgen>()
        .map(StagedProcgen::stage)
}

/// The number of placements the live staged drive has landed — `None` when no `StagedProcgen`
/// exists. Under the GTW-732 per-PREFAB granularity, one `Next` lands exactly one placement, so
/// this is the direct measure that a wire `Next` advanced the real drive by one unit.
fn driver_placements(app: &App) -> Option<usize> {
    app.world()
        .get_resource::<StagedProcgen>()
        .map(|driver| driver.placed_footprints().len())
}

/// Push a request onto the router's inbox exactly as the listener thread would, returning the
/// channel its reply arrives on.
fn send(tx: &Sender<IncomingRequest>, request: QaRequest) -> Receiver<QaResponse> {
    let (responder, reply_rx) = Responder::channel();
    let sent = tx.send(IncomingRequest::new(request, responder));
    assert!(sent.is_ok(), "the router inbox must be open");
    reply_rx
}

/// Build the REAL Load flow with the stepper FORCED enabled and the `net_qa` router wired to
/// an injected inbox, then drive into `BattleScapeState::Generation` with a live drive in
/// flight (a `StagedProcgen` engaged and idling, awaiting a command).
fn app_engaged_with_net_qa(seed: u64) -> (App, Sender<IncomingRequest>) {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut app = GdtfLoadTestAppBuilder::new()
        .starting_in(AppState::Load)
        .build();
    app.world_mut().insert_resource(BattleSeed::new(seed));
    app.add_plugins(ProcgenStepperPlugin::with_enabled(true));
    app.add_plugins(NetQaPlugin::with_channels(rx));

    let reached_menu = advance_until(
        &mut app,
        |app| running_state(app) == Some(RunningState::Menu),
        BUDGET,
    );
    assert!(
        reached_menu,
        "the REAL Load flow must reach RunningState::Menu within {BUDGET} updates; last \
         observed RunningState was {:?}",
        running_state(&app),
    );

    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    let engaged = advance_until(&mut app, |app| driver_stage(app).is_some(), BUDGET);
    assert!(
        engaged,
        "the stepper must engage a live StagedProcgen drive in Generation; last observed \
         BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    (app, tx)
}

/// Build a headless app resting in [`AppState::Running`] with no battle and no stepper drive,
/// with the REAL `net_qa` router wired — the "no drive in flight" fixture.
fn app_with_no_drive() -> (App, Sender<IncomingRequest>) {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    app.add_plugins(NetQaPlugin::with_channels(rx));
    app.update();
    (app, tx)
}

/// A `Next` over the wire latches the REAL `PendingStepCommand` — the SAME latch a panel Next
/// click writes — and the drive advances exactly one PLACEMENT (the wire path reaches the real
/// stepper, not a shadow copy). GTW-732: one `Next` lands one prefab (the player, still within
/// the Assemble stage), so the drive advance is measured as placements landed, not stage change.
#[test]
fn stepper_next_over_the_wire_advances_the_real_drive() {
    let (mut app, tx) = app_engaged_with_net_qa(FIXED_SEED);
    assert_eq!(
        driver_stage(&app),
        Some(ProcgenStage::Assemble),
        "a freshly engaged drive sits at the first stage before any command",
    );
    assert_eq!(
        driver_placements(&app),
        Some(0),
        "a freshly engaged drive has landed no placements before any command",
    );

    let reply = send(&tx, QaRequest::StepperControl(StepperCommandNet::Next));
    app.update();
    let got = reply.try_recv();
    assert!(
        matches!(
            got,
            Ok(QaResponse::StepperControlled(StepperReceipt::Latched))
        ),
        "a Next over the wire must latch the real PendingStepCommand, got {got:?}",
    );

    // The latch drains once per frame; advance a few so it is definitely consumed. A single
    // Next latches ONE command, so the drive advances exactly ONE placement and no further
    // (GTW-732 per-prefab granularity: the first Next lands the player prefab).
    for _ in 0..4 {
        app.update();
    }
    let after = driver_placements(&app);
    assert_eq!(
        after,
        Some(1),
        "the wire Next must reach PendingStepCommand and advance the real drive exactly one \
         placement (0 -> 1); observed {after:?}",
    );
}

/// A `Skip` over the wire latches the REAL `PendingStepCommand` and drives the whole staged
/// pipeline to completion — reaching `BattleRunning`, exactly as a panel Skip click does.
#[test]
fn stepper_skip_over_the_wire_drives_to_completion() {
    let (mut app, tx) = app_engaged_with_net_qa(FIXED_SEED);
    let reply = send(&tx, QaRequest::StepperControl(StepperCommandNet::Skip));
    app.update();
    let got = reply.try_recv();
    assert!(
        matches!(
            got,
            Ok(QaResponse::StepperControlled(StepperReceipt::Latched))
        ),
        "a Skip over the wire must latch the real PendingStepCommand, got {got:?}",
    );

    let reached = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        BUDGET,
    );
    assert!(
        reached,
        "a single Skip over the wire must drive the whole staged pipeline to completion and \
         reach BattleRunning; last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
}

/// An `Auto { running: true }` over the wire sets the REAL `AutoRunning` latch, and the drive
/// free-runs the whole pipeline to `BattleRunning` on its own timer with NO further command —
/// exactly as the panel's Start Auto button does.
#[test]
fn stepper_auto_over_the_wire_free_runs_to_completion() {
    let (mut app, tx) = app_engaged_with_net_qa(FIXED_SEED);
    let reply = send(
        &tx,
        QaRequest::StepperControl(StepperCommandNet::Auto {
            running: AutoRunNet::new(true),
        }),
    );
    app.update();
    let got = reply.try_recv();
    assert!(
        matches!(
            got,
            Ok(QaResponse::StepperControlled(StepperReceipt::Latched))
        ),
        "an Auto(true) over the wire must set the real AutoRunning latch, got {got:?}",
    );

    // Step the virtual clock in the SAME per-stage pace `AutoStepTimer` repeats on, so each
    // update fires the timer exactly once — no further wire command is ever sent.
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::ManualDuration(
            AutoStepDelay::DEFAULT.duration(),
        ));
    let reached = advance_until(
        &mut app,
        |app| battlescape_state(app) == Some(BattleScapeState::BattleRunning),
        BUDGET,
    );
    app.world_mut()
        .insert_resource(TimeUpdateStrategy::Automatic);
    assert!(
        reached,
        "an Auto(true) over the wire, with no further command, must free-run the whole drive \
         to BattleRunning on its own timer; last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );
}

/// A `StepperControl` sent with no live drive in flight is rejected `StepperInactive` at route
/// time — a typed rejection, never a silent no-op and never a panic (contract clause 3).
#[test]
fn stepper_control_with_no_drive_is_rejected_stepper_inactive() {
    let (mut app, tx) = app_with_no_drive();
    let reply = send(&tx, QaRequest::StepperControl(StepperCommandNet::Next));
    app.update();
    let got = reply.try_recv();
    assert!(
        matches!(got, Ok(QaResponse::Error(QaError::StepperInactive))),
        "a StepperControl with no live drive must be rejected StepperInactive, got {got:?}",
    );
}
