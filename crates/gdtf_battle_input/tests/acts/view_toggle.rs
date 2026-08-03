use bevy::prelude::*;
use gdtf_battle_input::{ActIntent, PendingActIntent};
use gdtf_battle_presenter::ViewMode;
use gdtf_test_utils::{clear_keys, press_key};

use super::harness::*;

// =================================================================================

fn view_mode(app: &App) -> ViewMode {
    app.world()
        .get_resource::<ViewMode>()
        .copied()
        .unwrap_or(ViewMode::DownToActive)
}

#[test]
fn toggle_full_view_intent_flips_view_mode_and_round_trips() {
    let mut app = acts_app();

    assert_eq!(
        view_mode(&app),
        ViewMode::DownToActive,
        "the harness seeds the default ViewMode (DownToActive)",
    );
    let active_before = active_storey(&app);

    app.world_mut()
        .resource_mut::<PendingActIntent>()
        .push(ActIntent::ToggleFullView);
    app.update();
    assert_eq!(
        view_mode(&app),
        ViewMode::FullView,
        "one ToggleFullView drain must flip ViewMode DownToActive -> FullView",
    );
    assert_eq!(
        active_storey(&app),
        active_before,
        "toggling FullView must NOT change ActiveLevel (C5)",
    );

    app.world_mut()
        .resource_mut::<PendingActIntent>()
        .push(ActIntent::ToggleFullView);
    app.update();
    assert_eq!(
        view_mode(&app),
        ViewMode::DownToActive,
        "a second ToggleFullView drain must flip ViewMode back FullView -> DownToActive",
    );
    assert_eq!(
        active_storey(&app),
        active_before,
        "the round-trip toggle still must NOT change ActiveLevel (C5)",
    );
}

#[test]
fn bound_full_view_key_press_pushes_toggle_full_view() {
    let mut app = acts_app();
    assert_eq!(
        view_mode(&app),
        ViewMode::DownToActive,
        "the harness starts in DownToActive",
    );

    press_key(&mut app, test_keybinds().toggle_full_view());
    app.update();
    clear_keys(&mut app);
    assert_eq!(
        view_mode(&app),
        ViewMode::FullView,
        "pressing the bound full-view key must push ToggleFullView (drained same update -> \
         FullView)",
    );

    press_key(&mut app, test_keybinds().toggle_full_view());
    app.update();
    clear_keys(&mut app);
    assert_eq!(
        view_mode(&app),
        ViewMode::DownToActive,
        "a second bound-key press must push ToggleFullView again (drained -> DownToActive)",
    );
}
