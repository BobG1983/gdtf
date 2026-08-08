use bevy::prelude::*;
use gdtf_battle_input::contextual::ContextualActSystems;
use gdtf_battle_sim::acts::{ExecuteDownedRequested, ShoveRequested, StabilizeDownedRequested};
use gdtf_test_utils::{MessageProbe, drain_message_probe, probed};

use super::{actors::*, harness::*};

fn add_probes(app: &mut App) {
    app.init_resource::<MessageProbe<ExecuteDownedRequested>>();
    app.init_resource::<MessageProbe<StabilizeDownedRequested>>();
    app.init_resource::<MessageProbe<ShoveRequested>>();
    app.add_systems(
        Update,
        (
            drain_message_probe::<ExecuteDownedRequested>,
            drain_message_probe::<StabilizeDownedRequested>,
            drain_message_probe::<ShoveRequested>,
        )
            .after(ContextualActSystems::Drain),
    );
}

fn executes(app: &App) -> usize {
    probed::<ExecuteDownedRequested>(app).len()
}
fn stabilizes(app: &App) -> usize {
    probed::<StabilizeDownedRequested>(app).len()
}
fn shoves(app: &App) -> usize {
    probed::<ShoveRequested>(app).len()
}

fn app_with_three_acts_offered() -> App {
    let mut app = battle_running_app();
    add_probes(&mut app);
    spawn_actor(&mut app, 5, 5, 0);
    spawn_downed(&mut app, 4, 4, 1, None);
    spawn_downed(&mut app, 5, 4, 0, Some(false));
    spawn_alive_enemy(&mut app, 6, 6, 1);
    app.update();
    assert!(
        execute_visible(&mut app) && stabilize_visible(&mut app) && shove_visible(&mut app),
        "sanity: exactly the three acts (Execute/Stabilize/Shove) are offered before the press",
    );
    app
}

#[test]
fn digit_activates_the_matching_visible_rank() {
    let mut app = app_with_three_acts_offered();

    press_digit(&mut app, KeyCode::Digit2);
    app.update();

    assert_eq!(
        stabilizes(&app),
        1,
        "Digit2 must fire exactly the rank-2 act (Stabilize) through the digit-key dispatch path",
    );
    assert_eq!(
        executes(&app),
        0,
        "Digit2 must NOT fire the rank-1 act (Execute)",
    );
    assert_eq!(
        shoves(&app),
        0,
        "Digit2 must NOT fire the rank-3 act (Shove)"
    );
}

#[test]
fn digit_binds_to_slot_not_action_with_a_different_visible_set() {
    let mut app = battle_running_app();
    add_probes(&mut app);
    spawn_actor(&mut app, 5, 5, 0);
    spawn_downed(&mut app, 5, 4, 0, Some(false));
    spawn_alive_enemy(&mut app, 6, 6, 1);
    app.update();
    assert!(
        stabilize_visible(&mut app) && shove_visible(&mut app) && !execute_visible(&mut app),
        "sanity: exactly two acts (Stabilize→rank1, Shove→rank2) are offered, Execute absent",
    );

    press_digit(&mut app, KeyCode::Digit2);
    app.update();

    assert_eq!(
        shoves(&app),
        1,
        "with this visible set Digit2 fires the rank-2 act (Shove) — the binding is per-slot",
    );
    assert_eq!(
        stabilizes(&app),
        0,
        "Stabilize is rank 1 (Digit1) here, so Digit2 does NOT fire it",
    );
}

#[test]
fn digit_beyond_visible_count_is_a_noop() {
    let mut app = app_with_three_acts_offered();

    press_digit(&mut app, KeyCode::Digit4);
    app.update();

    assert_eq!(
        executes(&app) + stabilizes(&app) + shoves(&app),
        0,
        "a digit past the visible count (Digit4 with three acts) dispatches nothing",
    );
}
