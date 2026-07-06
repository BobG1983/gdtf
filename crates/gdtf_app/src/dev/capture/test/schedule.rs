//! GTW-590 C4b — the capture system is REGISTERED and REACHABLE in the schedule/state
//! it must run in: [`DevCapturePlugin`] (built from a resolved snapshot, the REAL
//! construction path) registers `capture_when_ready` in `Update` gated on
//! [`BattleScapeState::BattleRunning`], and on the real state chain walked down to
//! `BattleRunning` it spawns one [`Screenshot`] entity per scheduled frame — never
//! before the state, never off-schedule. Also pins the GTW-590 C3 output-directory
//! materialization ([`ensure_output_dir`]) on both branches.
//!
//! The actual GPU readback / PNG encode cannot run headless
//! (`Screenshot::primary_window` needs a real window + render device), so the write
//! itself is the in-engine C5 evidence; THIS pin proves the trigger side — the layer
//! that was structurally dead in the GTW-590 symptom — fires on the right frames.

use bevy::{prelude::*, render::view::window::screenshot::Screenshot, state::app::StatesPlugin};

use super::super::{
    capture_config::CaptureConfig,
    diagnostics::ensure_output_dir,
    plugin::DevCapturePlugin,
    resolve::{RawCaptureEnv, resolve},
};
use crate::states::{AppState, BattleScapeState, GameState, RunningState};

/// The number of `Screenshot` capture entities in the world. Without bevy's render
/// stack (this is a `MinimalPlugins` app) nothing consumes or despawns them, so the
/// count is the cumulative number of capture requests the system fired.
fn screenshot_count(app: &mut App) -> usize {
    app.world_mut()
        .query::<&Screenshot>()
        .iter(app.world())
        .count()
}

/// Walk the REAL state chain (`AppState::Running` → `RunningState::Game` →
/// `GameState::BattleScape` → `BattleScapeState::BattleRunning`), asserting the
/// capture system spawns NOTHING on the way down — the state gate under test.
fn walk_to_battle_running(app: &mut App) {
    app.update(); // Rest a frame in AppState::Init: the gate must hold outside battle.
    app.world_mut()
        .resource_mut::<NextState<AppState>>()
        .set(AppState::Running);
    app.update(); // RunningState spawns at its default (Menu).
    app.world_mut()
        .resource_mut::<NextState<RunningState>>()
        .set(RunningState::Game);
    app.update(); // GameState spawns at its default (Setup).
    app.world_mut()
        .resource_mut::<NextState<GameState>>()
        .set(GameState::BattleScape);
    app.update(); // BattleScapeState spawns at its default (Generation).
    assert_eq!(
        screenshot_count(app),
        0,
        "no capture fires anywhere on the walk down to the battle",
    );
    app.world_mut()
        .resource_mut::<NextState<BattleScapeState>>()
        .set(BattleScapeState::BattleRunning);
}

/// C4b — with the pinned QA env shape (a path + a frame list), the REAL
/// `DevCapturePlugin` registers `capture_when_ready` reachable under
/// `BattleScapeState::BattleRunning`, and it requests a capture on EXACTLY the
/// scheduled `BattleRunning` frames (2 and 3 here): nothing before the state, nothing
/// on frame 1, one per scheduled frame, nothing after.
///
/// PIN: goes red if the system loses its registration, its `Update` placement, its
/// state gate, or its frame-schedule match — the "registered and reachable" layer of
/// the GTW-590 contract.
#[test]
fn capture_fires_on_exactly_the_scheduled_battle_running_frames() {
    let Ok(dir) = tempfile::TempDir::new() else {
        unreachable!("tempfile::TempDir::new must succeed in a standard test environment")
    };
    let raw = RawCaptureEnv {
        path: Some(dir.path().join("shot.png").to_string_lossy().into_owned()),
        frames: Some("2,3".to_owned()),
        ..RawCaptureEnv::default()
    };

    let mut app = App::new();
    app.add_plugins(MinimalPlugins).add_plugins(StatesPlugin);
    // The REAL state chain the production ScenesPlugin registers (states only — the
    // scenes are unrelated to the gate under test).
    app.init_state::<AppState>();
    app.add_sub_state::<RunningState>();
    app.add_sub_state::<GameState>();
    app.add_sub_state::<BattleScapeState>();
    // The REAL plugin, built from the resolved snapshot exactly as `from_env` does.
    app.add_plugins(DevCapturePlugin::from_resolved(resolve(&raw)));
    assert!(
        app.world().contains_resource::<CaptureConfig>(),
        "build() registers the capture config resource",
    );

    walk_to_battle_running(&mut app);

    app.update(); // BattleRunning frame 1: not scheduled.
    assert_eq!(screenshot_count(&mut app), 0, "frame 1 is not scheduled");
    app.update(); // BattleRunning frame 2: scheduled.
    assert_eq!(screenshot_count(&mut app), 1, "frame 2 captures");
    app.update(); // BattleRunning frame 3: scheduled (and last).
    assert_eq!(screenshot_count(&mut app), 2, "frame 3 captures");
    app.update(); // Past the schedule: no further capture.
    assert_eq!(screenshot_count(&mut app), 2, "the schedule is exhausted");
}

/// C3 — [`ensure_output_dir`] MATERIALIZES a missing parent chain (so every scheduled
/// PNG can land) and CLASSIFIES an uncreatable one as the `Err` that `build()` logs
/// as its loud `error!`; a bare filename (no parent) is a clean no-op.
#[test]
fn output_dir_is_materialized_or_classified_loudly() {
    let Ok(dir) = tempfile::TempDir::new() else {
        unreachable!("tempfile::TempDir::new must succeed in a standard test environment")
    };

    // A missing nested parent chain is created.
    let nested = dir.path().join("qa/frames/shot.png");
    let config = CaptureConfig::for_test(nested.clone());
    assert!(ensure_output_dir(&config).is_ok(), "creation succeeds");
    assert!(
        nested.parent().is_some_and(std::path::Path::exists),
        "the parent directory chain exists after ensure_output_dir",
    );

    // A parent BLOCKED by a plain file is the loud-error branch.
    let plain_file = dir.path().join("plain_file");
    assert!(std::fs::write(&plain_file, b"not a directory").is_ok());
    let blocked = CaptureConfig::for_test(plain_file.join("shot.png"));
    assert!(
        ensure_output_dir(&blocked).is_err(),
        "an uncreatable parent classifies as Err (build() error!s it, naming the path)",
    );

    // A bare filename has nothing to create.
    let bare = CaptureConfig::for_test(std::path::PathBuf::from("shot.png"));
    assert!(ensure_output_dir(&bare).is_ok(), "bare filename is a no-op");
}
