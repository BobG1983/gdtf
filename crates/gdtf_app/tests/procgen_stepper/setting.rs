//! GTW-868: the Options screen's DEV-ONLY procgen-stepper toggle drives engagement at
//! runtime.
//!
//! These drive the REAL screen — reach `RunningState::Options`, fire the toggle checkbox's
//! own native [`ValueChange<bool>`](bevy::ui_widgets::ValueChange) (exactly what a focused
//! `Enter` / `Space` makes `bevy_ui_widgets`' `checkbox_on_key_input` emit), and then start
//! a battle — so they cover the whole path the ticket specifies: toggle → typed intent →
//! `GameSettings` → `ProcgenStepperActive` → the stepped / normal generation path.

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

/// Whether the engagement marker is currently in the world.
fn stepper_engaged(app: &App) -> bool {
    app.world().get_resource::<ProcgenStepperActive>().is_some()
}

/// The single entity carrying marker `M`, if exactly one exists.
fn single_with<M: bevy::ecs::component::Component>(app: &mut App) -> Option<Entity> {
    let mut query = app.world_mut().query_filtered::<Entity, With<M>>();
    let found: Vec<Entity> = query.iter(app.world()).collect();
    match found.as_slice() {
        [one] => Some(*one),
        _ => None,
    }
}

/// The procgen-stepper readout's current text, if the screen is up.
fn stepper_value_text(app: &mut App) -> Option<String> {
    let entity = single_with::<ProcgenStepperValueLabel>(app)?;
    app.world()
        .get::<Text>(entity)
        .map(|text| text.as_str().to_owned())
}

/// Build the REAL Load flow with the stepper plugin added DISENGAGED — exactly how the
/// shipped `DevAffordancesPlugin` adds it — and drive it to the Options screen.
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

/// Fire the procgen-stepper checkbox's OWN native `ValueChange<bool>` with `value` — the
/// event `bevy_ui_widgets`' `checkbox_on_key_input` emits for a focused `Enter` / `Space`.
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

/// The toggle reads OFF on first entry, and flipping it ON inserts the engagement marker —
/// the setting is the ONE thing that decides engagement now (no environment involved).
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

/// Flipping the toggle back OFF removes the engagement marker again.
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

/// The load-bearing pin: with the setting flipped ON through the real screen, the NEXT
/// battle generation takes the STEPPED path — `request_battle_setup` is gated off, a
/// `StagedProcgen` drive is in flight, and the battle only completes once the drive is
/// driven to done.
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

    // Driving the engaged drive to done reaches a real battle with populated terrain — the
    // same result the normal path produces, just one step at a time.
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

/// The complement, and the acceptance clause a plain `cargo drun` / `cargo dtest` depends
/// on: with the setting left at its OFF default, the battle reaches `BattleRunning` by the
/// NORMAL to-completion path with no drive ever engaged and no pause.
#[test]
fn the_default_off_setting_leaves_the_normal_path_alone() {
    let mut app = app_on_options_screen();
    assert!(
        !stepper_engaged(&app),
        "precondition: the setting defaults OFF, so nothing is engaged",
    );

    // `drive_into_battle_running` queues Options -> Game itself and asserts BattleRunning.
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
