use bevy::prelude::*;
use gdtf_app::test_support::{ContextualPanelRoot, ExecuteButton, OpenDoorButton, StabilizeButton};
use gdtf_battle_input::contextual::ContextualActSystems;
use gdtf_battle_sim::{
    acts::{ExecuteDownedRequested, StabilizeDownedRequested},
    prelude::Position,
};
use gdtf_test_utils::{MessageProbe, drain_message_probe, press_ui_button, probed};

use super::{actors::*, harness::*};

fn add_execute_probe(app: &mut App) {
    app.init_resource::<MessageProbe<ExecuteDownedRequested>>();
    app.add_systems(
        Update,
        drain_message_probe::<ExecuteDownedRequested>.after(ContextualActSystems::Drain),
    );
}

fn executes(app: &App) -> Vec<ExecuteDownedRequested> {
    probed::<ExecuteDownedRequested>(app)
}

fn add_stabilize_probe(app: &mut App) {
    app.init_resource::<MessageProbe<StabilizeDownedRequested>>();
    app.add_systems(
        Update,
        drain_message_probe::<StabilizeDownedRequested>.after(ContextualActSystems::Drain),
    );
}

fn stabilizes(app: &App) -> Vec<StabilizeDownedRequested> {
    probed::<StabilizeDownedRequested>(app)
}

#[test]
fn adjacent_downed_enemy_offers_execute() {
    let mut app = battle_running_app();
    spawn_actor(&mut app, 5, 5, 0);
    spawn_downed(&mut app, 6, 6, 1, None);
    app.update();

    assert!(
        execute_visible(&mut app),
        "an 8-adjacent downed enemy must reveal the Execute button",
    );
    assert!(
        root_visible(&mut app),
        "an offered Execute must reveal the panel root",
    );
    assert!(
        !stabilize_visible(&mut app),
        "no downed ally in reach -> the Stabilize button stays hidden",
    );
}

#[test]
fn adjacent_downed_ally_offers_stabilize() {
    let mut app = battle_running_app();
    spawn_actor(&mut app, 5, 5, 0);
    spawn_downed(&mut app, 5, 6, 0, Some(false));
    app.update();

    assert!(
        stabilize_visible(&mut app),
        "an 8-adjacent unstabilized downed ally must reveal the Stabilize button",
    );
    assert!(
        root_visible(&mut app),
        "an offered Stabilize must reveal the panel root",
    );
    assert!(
        !execute_visible(&mut app),
        "no downed enemy in reach -> the Execute button stays hidden",
    );
}

#[test]
fn no_adjacent_downed_hides_panel() {
    let mut app = battle_running_app();
    spawn_actor(&mut app, 5, 5, 0);
    spawn_downed(&mut app, 20, 20, 1, None);
    app.update();

    assert!(
        !root_visible(&mut app),
        "no downed neighbour in reach -> the panel root stays hidden",
    );
    assert!(
        !execute_visible(&mut app),
        "no downed enemy in reach -> the Execute button stays hidden",
    );
    assert!(
        !stabilize_visible(&mut app),
        "no downed ally in reach -> the Stabilize button stays hidden",
    );
    assert_eq!(
        visibility::<OpenDoorButton>(&mut app),
        Some(Visibility::Hidden),
        "the Open Door button is a deferred act and stays hidden",
    );
}

#[test]
fn moving_actor_away_hides_panel_without_respawn() {
    let mut app = battle_running_app();
    let actor = spawn_actor(&mut app, 5, 5, 0);
    spawn_downed(&mut app, 6, 6, 1, None);
    app.update();

    assert!(
        execute_visible(&mut app),
        "sanity: the Execute button is shown"
    );
    let execute_before = single_with::<ExecuteButton>(&mut app);
    let stabilize_before = single_with::<StabilizeButton>(&mut app);
    let root_before = single_with::<ContextualPanelRoot>(&mut app);

    if let Some(mut pos) = app.world_mut().get_mut::<Position>(actor) {
        *pos = at(40, 40);
    }
    app.update();

    assert!(
        !root_visible(&mut app),
        "moving the actor out of reach must hide the panel root",
    );
    assert!(
        !execute_visible(&mut app),
        "moving the actor out of reach must hide the Execute button",
    );

    assert_eq!(
        single_with::<ExecuteButton>(&mut app),
        execute_before,
        "the Execute button entity must persist (Visibility toggle, not respawn)",
    );
    assert_eq!(
        single_with::<StabilizeButton>(&mut app),
        stabilize_before,
        "the Stabilize button entity must persist (Visibility toggle, not respawn)",
    );
    assert_eq!(
        single_with::<ContextualPanelRoot>(&mut app),
        root_before,
        "the panel root entity must persist (Visibility toggle, not respawn)",
    );
}

#[test]
fn pressing_execute_emits_execute_downed_requested_for_target() {
    let mut app = battle_running_app();
    add_execute_probe(&mut app);
    let actor = spawn_actor(&mut app, 5, 5, 0);
    let target = spawn_downed(&mut app, 6, 6, 1, None);

    app.update();
    assert!(
        execute_visible(&mut app),
        "sanity: the Execute button is offered before the press",
    );
    let Some(execute_btn) = single_with::<ExecuteButton>(&mut app) else {
        return;
    };

    press_ui_button(&mut app, execute_btn);
    app.update();

    let emitted = executes(&app);
    assert_eq!(
        emitted.len(),
        1,
        "pressing Execute with a target offered must emit exactly one ExecuteDownedRequested",
    );
    assert_eq!(emitted[0].actor, actor, "the actor is the SelectedShooter");
    assert_eq!(
        emitted[0].target, target,
        "the target is the carried downed neighbour",
    );
}

#[test]
fn pressing_stabilize_emits_stabilize_downed_requested_for_target() {
    let mut app = battle_running_app();
    add_stabilize_probe(&mut app);
    let actor = spawn_actor(&mut app, 5, 5, 0);
    let target = spawn_downed(&mut app, 5, 6, 0, Some(false));

    app.update();
    assert!(
        stabilize_visible(&mut app),
        "sanity: the Stabilize button is offered before the press",
    );
    let Some(stabilize_btn) = single_with::<StabilizeButton>(&mut app) else {
        return;
    };

    press_ui_button(&mut app, stabilize_btn);
    app.update();

    let emitted = stabilizes(&app);
    assert_eq!(
        emitted.len(),
        1,
        "pressing Stabilize with a target offered must emit exactly one StabilizeDownedRequested",
    );
    assert_eq!(emitted[0].actor, actor, "the actor is the SelectedShooter");
    assert_eq!(
        emitted[0].target, target,
        "the target is the carried downed ally",
    );
}
