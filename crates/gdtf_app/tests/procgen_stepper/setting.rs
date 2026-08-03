use bevy::{
    app::App,
    ecs::{entity::Entity, prelude::With, system::RunSystemOnce},
    prelude::{Commands, Text},
    state::state::NextState,
    ui_widgets::ValueChange,
};
use gdtf_app::test_support::{
    ProcgenStepperActive, ProcgenStepperPlugin, ProcgenStepperToggle, ProcgenStepperValueLabel,
    RunningState,
};
use gdtf_battle_sim::procgen::StagedProcgen;
use gdtf_test_utils::advance_until;

use super::harness::{
    BUDGET, FIXED_SEED, app_ready_for_battle, battlescape_state, drive_into_battle_running,
    drive_stepper_to_done, running_state, terrain_fingerprint,
};

fn stepper_engaged(app: &App) -> bool {
    app.world().get_resource::<ProcgenStepperActive>().is_some()
}

fn single_with<M: bevy::ecs::component::Component>(app: &mut App) -> Option<Entity> {
    let mut query = app.world_mut().query_filtered::<Entity, With<M>>();
    let found: Vec<Entity> = query.iter(app.world()).collect();
    match found.as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

fn stepper_value_text(app: &mut App) -> Option<String> {
    let entity = single_with::<ProcgenStepperValueLabel>(app)?;
    app.world()
        .get::<Text>(entity)
        .map(|text| text.as_str().to_owned())
}

fn app_on_options_screen() -> App {
    let mut app = app_ready_for_battle(FIXED_SEED);
    app.add_plugins(ProcgenStepperPlugin::with_enabled(false));
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Options);
    let reached = advance_until(
        &mut app,
        |app| running_state(app) == Some(RunningState::Options),
        BUDGET,
    );
    assert!(
        reached,
        "the app must reach RunningState::Options within {BUDGET} updates; last observed \
         RunningState was {:?}",
        running_state(&app),
    );
    app
}

fn set_stepper_toggle(app: &mut App, value: bool) {
    let checkbox = single_with::<ProcgenStepperToggle>(app);
    assert!(
        checkbox.is_some(),
        "a dev_tools build must spawn exactly one ProcgenStepperToggle on the Options screen",
    );
    let Some(checkbox) = checkbox else {
        return;
    };
    let triggered = app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            commands.trigger(ValueChange {
                source: checkbox,
                value,
                is_final: true,
            });
        });
    assert!(
        triggered.is_ok(),
        "the one-shot ValueChange trigger system must run",
    );
    app.update();
}

#[test]
fn toggling_the_setting_on_engages_the_stepper() {
    let mut app = app_on_options_screen();
    assert_eq!(
        stepper_value_text(&mut app).as_deref(),
        Some("Off"),
        "the dev procgen-stepper setting must default OFF on first entry",
    );
    assert!(
        !stepper_engaged(&app),
        "precondition: nothing may be engaged before the toggle is flipped",
    );

    set_stepper_toggle(&mut app, true);
    assert!(
        stepper_engaged(&app),
        "flipping the toggle ON must insert ProcgenStepperActive",
    );
    assert_eq!(
        stepper_value_text(&mut app).as_deref(),
        Some("On"),
        "and the readout must follow the setting",
    );
}

#[test]
fn toggling_the_setting_off_disengages_the_stepper() {
    let mut app = app_on_options_screen();
    set_stepper_toggle(&mut app, true);
    assert!(
        stepper_engaged(&app),
        "precondition: the toggle must have engaged the stepper",
    );

    set_stepper_toggle(&mut app, false);
    assert!(
        !stepper_engaged(&app),
        "flipping the toggle OFF must remove ProcgenStepperActive again",
    );
    assert_eq!(
        stepper_value_text(&mut app).as_deref(),
        Some("Off"),
        "and the readout must follow the setting back",
    );
}

#[test]
fn a_flipped_on_setting_steps_the_next_generation() {
    let mut app = app_on_options_screen();
    set_stepper_toggle(&mut app, true);

    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    let reached_generation = advance_until(
        &mut app,
        |app| app.world().get_resource::<StagedProcgen>().is_some(),
        BUDGET,
    );
    assert!(
        reached_generation,
        "the toggled-on stepper must engage a StagedProcgen drive for the next Generation \
         entry; last observed BattleScapeState was {:?}",
        battlescape_state(&app),
    );

    drive_stepper_to_done(&mut app);
    let reached_running = advance_until(
        &mut app,
        |app| {
            battlescape_state(app) == Some(gdtf_app::test_support::BattleScapeState::BattleRunning)
        },
        BUDGET,
    );
    assert!(
        reached_running,
        "the stepped drive must reach BattleRunning once stepped to done; last observed \
         BattleScapeState was {:?}",
        battlescape_state(&app),
    );
    let fingerprint = terrain_fingerprint(&app);
    assert!(
        matches!(fingerprint, Some(n) if n > 0),
        "the stepped path must set up a populated TerrainIndex; got {fingerprint:?}",
    );
}

#[test]
fn the_default_off_setting_leaves_the_normal_path_alone() {
    let mut app = app_on_options_screen();
    assert!(
        !stepper_engaged(&app),
        "precondition: the setting defaults OFF, so nothing is engaged",
    );

    drive_into_battle_running(&mut app);
    assert!(
        app.world().get_resource::<StagedProcgen>().is_none(),
        "no stepper drive may ever have been engaged with the setting off",
    );
    let fingerprint = terrain_fingerprint(&app);
    assert!(
        matches!(fingerprint, Some(n) if n > 0),
        "the normal path must set up a populated TerrainIndex; got {fingerprint:?}",
    );
}
