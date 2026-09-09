//! Stepping the shown storey and flipping full view, one `app.update()` per call.

use bevy::app::App;
use gdtf_battle_presenter::{ActiveLevel, ReachableOverlayEnabled, ViewMode};
use gdtf_game::qa_wire::{
    cell::LevelNet,
    misc::{ReachableOverlayNet, ViewModeNet},
};
use serde::Deserialize;

use super::{
    battle_fixture::{decoded, drive_into_battle_running, menu_app_with_mcp, run_one_frame},
    battle_setup::hold_the_screen_still,
    command_exchange::{
        VIEW_LEVEL_DOWN, VIEW_LEVEL_UP, VIEW_TOGGLE_FULL_VIEW, VIEW_TOGGLE_REACHABLE_OVERLAY,
    },
};

/// What all three view commands answer with.
#[derive(Debug, Deserialize)]
struct ShownBody {
    level:     LevelNet,
    full_view: ViewModeNet,
}

/// What the reachable-overlay toggle answers with.
#[derive(Debug, Deserialize)]
struct OverlayBody {
    overlay: ReachableOverlayNet,
}

fn overlay_flag(app: &App) -> ReachableOverlayNet {
    let Some(enabled) = app.world().get_resource::<ReachableOverlayEnabled>() else {
        unreachable!("the presenter puts ReachableOverlayEnabled up when its plugin is built");
    };
    ReachableOverlayNet::from_presenter(*enabled)
}

fn active_level(app: &App) -> LevelNet {
    let Some(active) = app.world().get_resource::<ActiveLevel>() else {
        unreachable!("the presenter inits ActiveLevel when its plugin is built");
    };
    LevelNet::new(***active)
}

fn view_mode(app: &App) -> ViewModeNet {
    let Some(mode) = app.world().get_resource::<ViewMode>() else {
        unreachable!("the presenter inits ViewMode when its plugin is built");
    };
    ViewModeNet::from_view(*mode)
}

#[test]
fn a_level_up_raises_the_shown_storey_in_the_frame_that_follows_the_call() {
    let (mut app, tx) = menu_app_with_mcp();
    drive_into_battle_running(&mut app);
    assert_eq!(
        active_level(&app),
        LevelNet::new(0),
        "the battle opens on the ground floor, which is what the step is measured from",
    );

    let body: ShownBody = decoded(
        VIEW_LEVEL_UP,
        run_one_frame(&mut app, &tx, VIEW_LEVEL_UP, "()"),
    );

    assert_eq!(
        body.level,
        LevelNet::new(1),
        "the reply reports the storey the step lands on: {body:?}",
    );
    assert_eq!(
        active_level(&app),
        LevelNet::new(1),
        "the intent has to be drained in the frame it was pushed, so the world's own ActiveLevel \
         already agrees with the reply — a frame late and this reads 0",
    );
    assert_eq!(
        body.full_view,
        ViewModeNet::DownToActive,
        "a level step leaves the view mode alone: {body:?}",
    );
}

#[test]
fn a_level_down_from_the_ground_floor_clamps_and_reports_the_unchanged_storey() {
    let (mut app, tx) = menu_app_with_mcp();
    drive_into_battle_running(&mut app);
    assert_eq!(
        active_level(&app),
        LevelNet::new(0),
        "the battle opens on the ground floor, which is the clamp this case drives into",
    );

    let body: ShownBody = decoded(
        VIEW_LEVEL_DOWN,
        run_one_frame(&mut app, &tx, VIEW_LEVEL_DOWN, "()"),
    );

    assert_eq!(
        body.level,
        LevelNet::new(0),
        "there is no storey below the ground floor, so the step clamps and the reply reports the \
         storey the view is still on: {body:?}",
    );
    assert_eq!(
        active_level(&app),
        LevelNet::new(0),
        "the clamp is the sim's, not the reply's — the world's ActiveLevel must not have moved",
    );
}

#[test]
fn a_level_up_still_lands_while_the_screen_is_playing_the_act_log_back() {
    let (mut app, tx) = menu_app_with_mcp();
    drive_into_battle_running(&mut app);
    let held = hold_the_screen_still(&mut app);
    assert!(
        held.is_ok(),
        "the case needs the playback gate shut to mean anything: {held:?}",
    );

    let body: ShownBody = decoded(
        VIEW_LEVEL_UP,
        run_one_frame(&mut app, &tx, VIEW_LEVEL_UP, "()"),
    );

    assert_eq!(
        body.level,
        LevelNet::new(1),
        "a view step is not an act: the screen being behind the log gates neither the command's \
         availability nor the intent's drain, so the step lands with the gate shut: {body:?}",
    );
    assert_eq!(
        active_level(&app),
        LevelNet::new(1),
        "the world's own ActiveLevel is what says the intent was drained rather than dropped by \
         the shut gate",
    );
}

#[test]
fn a_full_view_toggle_flips_the_view_mode_in_the_frame_that_follows_the_call() {
    let (mut app, tx) = menu_app_with_mcp();
    drive_into_battle_running(&mut app);
    assert_eq!(
        view_mode(&app),
        ViewModeNet::DownToActive,
        "the battle opens showing storeys down to the active one, which the toggle flips",
    );

    let body: ShownBody = decoded(
        VIEW_TOGGLE_FULL_VIEW,
        run_one_frame(&mut app, &tx, VIEW_TOGGLE_FULL_VIEW, "()"),
    );

    assert_eq!(
        body.full_view,
        ViewModeNet::FullView,
        "the reply reports the view mode the flip lands on: {body:?}",
    );
    assert_eq!(
        view_mode(&app),
        ViewModeNet::FullView,
        "the intent has to be drained in the frame it was pushed, so the world's own ViewMode \
         already agrees with the reply",
    );
    assert_eq!(
        body.level,
        LevelNet::new(0),
        "a toggle leaves the storey alone: {body:?}",
    );
}

#[test]
fn a_reachable_overlay_toggle_turns_it_on_and_a_second_call_turns_it_back_off() {
    let (mut app, tx) = menu_app_with_mcp();
    drive_into_battle_running(&mut app);
    assert_eq!(
        overlay_flag(&app),
        ReachableOverlayNet::Off,
        "the battle opens with the reachable-range overlay off, which is what the toggle flips",
    );

    let first: OverlayBody = decoded(
        VIEW_TOGGLE_REACHABLE_OVERLAY,
        run_one_frame(&mut app, &tx, VIEW_TOGGLE_REACHABLE_OVERLAY, "()"),
    );

    assert_eq!(
        first.overlay,
        ReachableOverlayNet::On,
        "the reply reports the overlay state the flip lands on: {first:?}",
    );
    assert_eq!(
        overlay_flag(&app),
        ReachableOverlayNet::On,
        "the flag is written in the frame the call was claimed in, so the world already agrees \
         with the reply",
    );

    let second: OverlayBody = decoded(
        VIEW_TOGGLE_REACHABLE_OVERLAY,
        run_one_frame(&mut app, &tx, VIEW_TOGGLE_REACHABLE_OVERLAY, "()"),
    );

    assert_eq!(
        second.overlay,
        ReachableOverlayNet::Off,
        "a second call puts the overlay back where it started: {second:?}",
    );
    assert_eq!(
        overlay_flag(&app),
        ReachableOverlayNet::Off,
        "the second flip is a write, not a reply-only value",
    );
}
