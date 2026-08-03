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

fn state_of(app: &App) -> BattleScapeState {
    *app.world().resource::<State<BattleScapeState>>().get()
}

fn phase_of(app: &App) -> Option<EndPhase> {
    app.world()
        .get_resource::<EndTransition>()
        .map(EndTransition::phase)
}

fn run_move_on(app: &mut App) {
    let ran = app.world_mut().run_system_once(move_on);
    assert!(ran.is_ok(), "move_on must run once in the unit harness");
    app.update();
}

fn spawn_bolt(app: &mut App) -> Entity {
    app.world_mut().spawn(ShotProjectile).id()
}

fn write_shown_shot(app: &mut App) {
    let shot = deciding_shot(app);
    app.world_mut()
        .resource_mut::<Messages<Played<ShotFired>>>()
        .write(Played::new(shot));
}

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

#[test]
fn an_unspawnable_deciding_bolt_cannot_wedge_the_end_transition() {
    let mut app = battle_running_gate_app();
    write_shown_shot(&mut app);
    app.insert_resource(EndTransition::with_backstop(
        EndPhase::AwaitingDecidingShot {
            seen_busy: SeenBusy::new(false),
        },
        EndBackstopSeconds::new(0.5),
    ));

    run_move_on(&mut app);
    assert_eq!(
        state_of(&app),
        BattleScapeState::BattleRunning,
        "before the backstop elapses the gate still holds",
    );

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

#[test]
fn the_battle_holds_battlerunning_until_the_cursor_drains_the_log() {
    let mut app = battle_running_gate_app();
    app.init_resource::<PlaybackCursor>();
    let mut log = ActLog::default();
    let actor = app.world_mut().spawn_empty().id();
    log.append(RecordedAct::new(
        actor,
        ActProvenance::Clock,
        ActDeed::BleedStarted,
    ));
    app.insert_resource(log);

    run_move_on(&mut app);
    assert_eq!(
        state_of(&app),
        BattleScapeState::BattleRunning,
        "the battle must not end while the presenter still has unplayed acts — otherwise \
         teardown drops them and the player never sees the volley that ended the fight",
    );

    app.insert_resource(ActLog::default());
    run_move_on(&mut app);
    assert_eq!(
        state_of(&app),
        BattleScapeState::AnimateOut,
        "once the cursor has shown everything, the end transition proceeds",
    );
}

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
