use std::time::Duration;

use bevy::{app::App, ecs::system::RunSystemOnce};
use gdtf_battle_presenter::{PlaybackCursor, PlaybackGate, playback_caught_up};
use gdtf_battle_sim::{
    act_log::{ActDeed, ActLog, ActProvenance},
    ganger::Direction,
};

use super::harness::*;

// A named fn, not a closure, so the SystemParam lifetime is inferred.
fn gate_is_open(gate: PlaybackGate) -> bool {
    gate.is_open()
}

fn readings(app: &mut App) -> (bool, bool) {
    let gate = app.world_mut().run_system_once(gate_is_open);
    let filter = app.world_mut().run_system_once(playback_caught_up);
    let (Ok(gate), Ok(filter)) = (gate, filter) else {
        unreachable!("both readings run as one-shot systems");
    };
    (gate, filter)
}

fn assert_both_read(app: &mut App, expected: bool, state: &str) {
    let (gate, filter) = readings(app);
    assert_eq!(
        gate, filter,
        "PlaybackGate::is_open and playback_caught_up must give the same answer ({state})",
    );
    assert_eq!(gate, expected, "both readings must be {expected} ({state})");
}

fn app_with_one_appended_act() -> App {
    let mut app = playback_app();
    let bleeder = spawn_ganger(&mut app, ground(0, 0), Direction::East);
    seed(&mut app);
    append(
        &mut app,
        bleeder,
        ActProvenance::Commanded,
        ActDeed::BleedStarted,
    );
    app
}

#[test]
fn both_readings_open_when_neither_resource_exists() {
    let mut app = App::new();

    assert_both_read(&mut app, true, "no act log and no playback cursor");
}

#[test]
fn both_readings_open_when_only_the_act_log_exists() {
    let mut app = App::new();
    app.insert_resource(ActLog::default());

    assert!(
        app.world().get_resource::<PlaybackCursor>().is_none(),
        "this state has a log and no cursor",
    );

    assert_both_read(&mut app, true, "only the act log exists");
}

#[test]
fn both_readings_open_when_only_the_playback_cursor_exists() {
    let mut app = App::new();
    app.init_resource::<PlaybackCursor>();

    assert!(
        app.world().get_resource::<ActLog>().is_none(),
        "this state has a cursor and no log",
    );

    assert_both_read(&mut app, true, "only the playback cursor exists");
}

#[test]
fn both_readings_close_while_the_cursor_lags_the_log_head() {
    let mut app = app_with_one_appended_act();

    assert_eq!(*shown(&app), 0, "nothing is shown before the cursor runs");

    assert_both_read(&mut app, false, "shown is behind the log head");
}

#[test]
fn both_readings_close_while_the_cursor_holds_at_the_log_head() {
    let mut app = app_with_one_appended_act();

    step(&mut app, Duration::from_millis(1));
    assert_eq!(*shown(&app), 1, "the cursor shows the appended entry");
    assert!(
        holding(&app),
        "the cursor holds on the entry it just showed"
    );

    assert_both_read(&mut app, false, "shown is at the head but the cursor holds");
}

#[test]
fn both_readings_open_once_the_hold_ends() {
    let mut app = app_with_one_appended_act();
    let beat = *tuning(&app).minor_seconds;

    step(&mut app, Duration::from_millis(1));
    step(&mut app, Duration::from_secs_f32(beat * 2.0));
    assert!(!holding(&app), "the minor beat ({beat}s) ends the hold");

    assert_both_read(&mut app, true, "shown is at the head and the hold is over");
}

#[test]
fn both_readings_open_when_the_cursor_is_past_the_log_head() {
    let mut app = playback_app();
    app.world_mut()
        .resource_mut::<PlaybackCursor>()
        .advance_past_shown();

    // A step here would erase the state: advance_playback resets a cursor past the head.
    let head = app.world().resource::<ActLog>().head();
    assert!(shown(&app) > head, "the cursor sits past the log head");
    assert!(!holding(&app), "the cursor holds nothing");

    assert_both_read(&mut app, true, "shown is past the log head");
}
