//! Unit tests for the deferred end-of-battle gate — the classification, the two traps it
//! closes, and the GTW-727 backstop.

use std::time::Duration;

use bevy::{
    app::App,
    ecs::system::RunSystemOnce,
    prelude::*,
    state::{app::StatesPlugin, state::State},
};
use gdtf_battle_presenter::{PlaybackCursor, Played, ShotProjectile};
use gdtf_battle_sim::{
    act_log::{ActDeed, ActLog, ActProvenance, RecordedAct},
    shot_fired::ShotFired,
};

use super::{
    move_on,
    phase::{EndBackstopSeconds, EndPhase, EndTransition, SeenBusy},
};
use crate::states::{
    BattleScapeState, running::game::battlescape::battle_running::resources::BattleRunningComplete,
};

/// Builds a headless app with the `BattleScapeState` machine rested at `BattleRunning` and
/// the run-complete latch present — the exact gate `move_on` runs under.
///
/// No playback cursor and no act log are inserted, so `PlaybackGate` fails OPEN exactly as
/// it does in a presenter-less app; the C39 hold is exercised by the integration suite,
/// which has a real cursor.
fn battle_running_gate_app() -> App {
    let mut app = App::new();
    app.add_plugins(StatesPlugin);
    app.init_state::<BattleScapeState>();
    app.init_resource::<Time>();
    app.add_message::<Played<ShotFired>>();
    app.insert_resource(BattleRunningComplete);
    app.world_mut()
        .resource_mut::<NextState<BattleScapeState>>()
        .set(BattleScapeState::BattleRunning);
    app.update();
    app
}

/// The current `BattleScapeState`.
fn state_of(app: &App) -> BattleScapeState {
    *app.world().resource::<State<BattleScapeState>>().get()
}

/// The current end-transition phase, if the latch exists.
fn phase_of(app: &App) -> Option<EndPhase> {
    app.world()
        .get_resource::<EndTransition>()
        .map(EndTransition::phase)
}

/// Runs `move_on` once, then applies the queued state transition.
fn run_move_on(app: &mut App) {
    let ran = app.world_mut().run_system_once(move_on);
    assert!(ran.is_ok(), "move_on must run once in the unit harness");
    app.update();
}

/// Spawns a stand-in bolt so the FX-busy query reads busy.
fn spawn_bolt(app: &mut App) -> Entity {
    app.world_mut().spawn(ShotProjectile).id()
}

/// Writes a shown deciding shot onto the played buffer.
fn write_shown_shot(app: &mut App) {
    let shot = deciding_shot(app);
    app.world_mut()
        .resource_mut::<Messages<Played<ShotFired>>>()
        .write(Played::new(shot));
}

/// FLEE / NON-SHOT END: with no shown shot and an idle pipeline, the gate classifies
/// `NoDecidingShot` and leaves promptly — it must not hang waiting for a pipeline that
/// never goes busy.
#[test]
fn no_deciding_shot_transitions_promptly() {
    let mut app = battle_running_gate_app();
    run_move_on(&mut app);
    assert_eq!(
        state_of(&app),
        BattleScapeState::AnimateOut,
        "a non-shot end (no shown shot, idle pipeline) must transition promptly",
    );
    assert_eq!(phase_of(&app), Some(EndPhase::NoDecidingShot));
}

/// SPAWN-RACE: a deciding shot is SHOWN on the latch frame but its deferred bolt has not
/// materialized, so the pipeline reads idle. The gate must not leave in that gap.
#[test]
fn a_deciding_shot_does_not_transition_in_the_pre_spawn_gap() {
    let mut app = battle_running_gate_app();
    write_shown_shot(&mut app);
    run_move_on(&mut app);
    assert_eq!(
        state_of(&app),
        BattleScapeState::BattleRunning,
        "a deciding shot whose bolt has not yet spawned must NOT transition — idle here is \
         the pre-spawn gap, not a drained pipeline",
    );
    assert_eq!(
        phase_of(&app),
        Some(EndPhase::AwaitingDecidingShot {
            seen_busy: SeenBusy::new(false),
        }),
    );

    spawn_bolt(&mut app);
    run_move_on(&mut app);
    assert_eq!(state_of(&app), BattleScapeState::BattleRunning);
    assert_eq!(
        phase_of(&app),
        Some(EndPhase::AwaitingDecidingShot {
            seen_busy: SeenBusy::new(true),
        }),
        "once the pipeline is busy the phase must record seen_busy",
    );
}

/// AT THE IMPACT: once the deciding shot's pipeline has gone busy and drained, the gate
/// leaves — at the impact, not at the drain.
#[test]
fn a_deciding_shot_transitions_once_its_pipeline_drains() {
    let mut app = battle_running_gate_app();
    write_shown_shot(&mut app);
    let bolt = spawn_bolt(&mut app);
    run_move_on(&mut app);
    assert_eq!(state_of(&app), BattleScapeState::BattleRunning);

    app.world_mut().entity_mut(bolt).despawn();
    run_move_on(&mut app);
    assert_eq!(
        state_of(&app),
        BattleScapeState::AnimateOut,
        "once the deciding shot's pipeline has gone busy and then drained, the gate must \
         advance BattleRunning → AnimateOut",
    );
}

/// GTW-727 C40 — THE HARD-HANG CURE. A deciding shot whose bolt NEVER spawns leaves the
/// pipeline permanently idle-and-never-busy, which is the one branch that used to wait with
/// no timeout at all. The backstop must end the battle anyway rather than stranding the
/// player in a state with no input and no quit key.
#[test]
fn an_unspawnable_deciding_bolt_cannot_wedge_the_end_transition() {
    let mut app = battle_running_gate_app();
    write_shown_shot(&mut app);
    // Seed a SHORT backstop directly, so the test drives real elapsed time rather than
    // sleeping (no wall-clock waits anywhere).
    app.insert_resource(EndTransition::with_backstop(
        EndPhase::AwaitingDecidingShot {
            seen_busy: SeenBusy::new(false),
        },
        EndBackstopSeconds::new(0.5),
    ));

    // No bolt ever spawns: the pipeline stays idle-and-never-busy, so the FX branch alone
    // would hold forever.
    run_move_on(&mut app);
    assert_eq!(
        state_of(&app),
        BattleScapeState::BattleRunning,
        "before the backstop elapses the gate still holds",
    );

    // Drive Time by hand past the backstop.
    app.world_mut()
        .resource_mut::<Time>()
        .advance_by(Duration::from_millis(600));
    run_move_on(&mut app);
    assert_eq!(
        state_of(&app),
        BattleScapeState::AnimateOut,
        "past the backstop the transition must leave anyway — no phase may hold forever",
    );
}

/// GTW-727 C39 — THE SILENT-TRUNCATION CURE. The battle must NOT leave `BattleRunning`
/// while the playback cursor still has unplayed acts, even when the FX pipeline is idle and
/// the end would otherwise be prompt.
///
/// This is the defect that appears the moment the presenter paces itself: a lethal reaction
/// volley resolves shots 1 to 3 in one tick, shot 1 kills, the census decides, and the old
/// gate released at shot 1's impact. Teardown then dropped the act log — so the player never
/// saw shots 2 and 3. The ticket's headline behaviour, absent in live play, with a fully
/// green suite.
#[test]
fn the_battle_holds_battlerunning_until_the_cursor_drains_the_log() {
    let mut app = battle_running_gate_app();
    // A presenter that is BEHIND: one entry recorded, nothing shown.
    app.init_resource::<PlaybackCursor>();
    let mut log = ActLog::default();
    let actor = app.world_mut().spawn_empty().id();
    log.append(RecordedAct::new(
        actor,
        ActProvenance::Clock,
        ActDeed::BleedStarted,
    ));
    app.insert_resource(log);

    // No shown shot and an idle pipeline — the `NoDecidingShot` phase, which leaves
    // PROMPTLY on its own. The cursor is the only thing holding it.
    run_move_on(&mut app);
    assert_eq!(
        state_of(&app),
        BattleScapeState::BattleRunning,
        "the battle must not end while the presenter still has unplayed acts — otherwise \
         teardown drops them and the player never sees the volley that ended the fight",
    );

    // Drain the log (the cursor caught up) — now it may leave.
    app.insert_resource(ActLog::default());
    run_move_on(&mut app);
    assert_eq!(
        state_of(&app),
        BattleScapeState::AnimateOut,
        "once the cursor has shown everything, the end transition proceeds",
    );
}

/// A deciding shot carrying no report — geometry-only is enough: ANY shown shot means a
/// tracer is animating that the transition must wait on.
fn deciding_shot(app: &mut App) -> ShotFired {
    let shooter = app.world_mut().spawn_empty().id();
    let struck = app.world_mut().spawn_empty().id();
    ShotFired {
        shooter,
        muzzle: gdtf_battle_sim::metric::SimPos::new(0.0, 0.0, 0.0),
        trajectory: gdtf_battle_sim::sample_cone::ShotDir::from_direction(Vec3::new(1.0, 0.0, 0.0)),
        impact_cell: gdtf_battle_sim::metric::Cell::new(1, 0),
        impact_level: gdtf_battle_sim::metric::Level::new(0),
        kind: gdtf_battle_sim::resolve_coarse::ShotKind::Ganger(struck),
        damage: gdtf_battle_sim::weapon::DamageType::Kinetic,
        report: None,
    }
}
