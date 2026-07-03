//! GTW-577 C5 — the headless pin on the game-side capture EXIT (`poll_then_quit`).
//!
//! `dev_capture`-gated like the module it pins (run with
//! `cargo test -p gdtf_app --features dev_capture --test capture_quit`): the whole file
//! compiles to an empty test crate without the feature, exactly like the capture hooks
//! themselves.
//!
//! Two paths, both through the REAL pipeline (`gdtf_screenshot::settle_then_capture` chained
//! into the exported `poll_then_quit`, exactly as the gang-editor / procgen-viz hooks chain
//! them):
//!
//! - **completion**: the PNG exists on disk once the shot is requested → ONE poll frame sets
//!   [`RunningState::Quit`] — and NEVER writes `AppExit` (the exit rides the Quit →
//!   `AppState::Teardown` cascade; a direct `AppExit` is the macOS winit hang, Bevy #23313).
//! - **poll-cap**: the PNG never lands → the [`PollCap`] safety net still sets
//!   [`RunningState::Quit`] (no hang), still never writing `AppExit`.
#![cfg(feature = "dev_capture")]

use bevy::{ecs::schedule::IntoScheduleConfigs, state::state::State};
use gdtf_app::test_support::{AppState, RunningState, poll_then_quit};
use gdtf_screenshot::{CapturePath, CaptureProgress, PollCap, SettleFrames, settle_then_capture};
use gdtf_test_utils::{GdtfTestAppBuilder, advance_until};

/// A short settle so the pipeline requests the shot on the first driven frame.
const TEST_SETTLE: SettleFrames = SettleFrames::new(1);

/// A tiny poll cap so the no-PNG path resolves within a few frames.
const TEST_POLL_CAP: PollCap = PollCap::new(3);

/// Update budget for the walk down to `RunningState::Menu`.
const MENU_BUDGET: u32 = 32;

/// Update budget for reaching `RunningState::Quit` after the capture resources land.
const QUIT_BUDGET: usize = 32;

/// Build the real headless state machine resting in `RunningState::Menu`, with the REAL
/// capture pipeline (`settle_then_capture` → `poll_then_quit`, the scene hooks' exact chain)
/// registered and armed for `path`.
fn menu_app_with_capture_chain(path: CapturePath) -> bevy::app::App {
    // Scene support: entering `Running` spawns the UI camera + menu via `bsn!` `spawn_scene`,
    // which reads the `AssetServer` — the `new_with_scene_support` harness tier.
    let mut app = GdtfTestAppBuilder::new_with_scene_support()
        .starting_in(AppState::Running)
        .build();
    // The chain the gang-editor / procgen-viz hooks register (minus their scene drives):
    // gate on the CapturePath exactly as they do.
    app.add_systems(
        bevy::app::Update,
        (settle_then_capture, poll_then_quit)
            .chain()
            .run_if(bevy::prelude::resource_exists::<CapturePath>),
    );
    // Rest in Menu (the launch point every capture run drives from).
    let reached_menu = advance_until(
        &mut app,
        |app| {
            app.world()
                .get_resource::<State<RunningState>>()
                .is_some_and(|state| *state.get() == RunningState::Menu)
        },
        MENU_BUDGET,
    );
    assert!(
        reached_menu,
        "the headless walk rests in RunningState::Menu"
    );
    // Arm the pipeline: path + this test's settle/poll magnitudes + fresh progress.
    app.insert_resource(path);
    app.insert_resource(TEST_SETTLE);
    app.insert_resource(TEST_POLL_CAP);
    app.init_resource::<CaptureProgress>();
    app
}

/// Drive until `RunningState` rests in `Quit`, asserting `AppExit` is NEVER written on the
/// way (the cascade owns the eventual exit — `poll_then_quit` must not short-circuit it).
fn drive_to_quit_asserting_no_app_exit(app: &mut bevy::app::App) {
    for _ in 0..QUIT_BUDGET {
        app.update();
        assert!(
            app.should_exit().is_none(),
            "poll_then_quit must NEVER write AppExit — the exit rides the RunningState::Quit \
             cascade (Bevy #23313)",
        );
        let quit = app
            .world()
            .get_resource::<State<RunningState>>()
            .is_some_and(|state| *state.get() == RunningState::Quit);
        if quit {
            return;
        }
    }
    let observed = app
        .world()
        .get_resource::<State<RunningState>>()
        .map(|state| *state.get());
    unreachable!(
        "RunningState must reach Quit within {QUIT_BUDGET} updates; rested in {observed:?}"
    );
}

/// COMPLETION: with the PNG already on disk (the readback landed), the first poll frame after
/// the shot is requested sets `RunningState::Quit` — never `AppExit`.
#[test]
fn png_on_disk_quits_via_the_running_state_cascade() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("gdtf_capture_quit_done_{}.png", std::process::id()));
    // Pre-write the "captured" PNG so `shot.exists()` is true the moment the poll starts —
    // the headless stand-in for the async GPU readback having flushed.
    assert!(std::fs::write(&path, b"png-bytes").is_ok());

    let mut app = menu_app_with_capture_chain(CapturePath::new(path.clone()));
    drive_to_quit_asserting_no_app_exit(&mut app);

    // The pipeline really ran: the shot was requested (settle elapsed) before the quit.
    let requested = app
        .world()
        .get_resource::<CaptureProgress>()
        .is_some_and(CaptureProgress::is_requested);
    assert!(requested, "settle_then_capture requested the shot first");
    drop(std::fs::remove_file(&path));
}

/// POLL-CAP: with a PNG that never lands, the safety net still quits via the cascade after
/// [`TEST_POLL_CAP`] poll frames — no hang, and still never a direct `AppExit`.
#[test]
fn missing_png_quits_via_the_poll_cap_safety_net() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "gdtf_capture_quit_never_{}.png",
        std::process::id()
    ));
    drop(std::fs::remove_file(&path));

    let mut app = menu_app_with_capture_chain(CapturePath::new(path));
    drive_to_quit_asserting_no_app_exit(&mut app);
}
