use bevy::{camera::RenderTarget, prelude::*, render::view::window::screenshot::Screenshot};
use gdtf_screenshot::{
    CapturePipelinePlugin, CaptureQueue, PollCap, SettleFrames, ShotDir, ShotStem,
};

use super::harness::{headless_windowed_app, settle, with_present_path};

const TEST_SETTLE: u32 = 0;

const TEST_POLL_BUDGET: u32 = 2;

fn wire_pump(app: &mut App, dir: std::path::PathBuf) {
    app.add_plugins(CapturePipelinePlugin::<()>::new());
    app.insert_resource(ShotDir::new(dir));
    app.insert_resource(SettleFrames::new(TEST_SETTLE));
    app.insert_resource(PollCap::new(TEST_POLL_BUDGET));
}

fn enqueue(app: &mut App) {
    app.world_mut()
        .resource_mut::<CaptureQueue<()>>()
        .push(Some(ShotStem::new("game_source")), ());
}

fn spawned_screenshot_target(app: &mut App) -> Option<RenderTarget> {
    let mut shots = app.world_mut().query::<&Screenshot>();
    let targets: Vec<RenderTarget> = shots.iter(app.world()).map(|shot| shot.0.clone()).collect();
    match targets.as_slice() {
        [one] => Some(one.clone()),
        _ => None,
    }
}

#[test]
fn the_pump_captures_the_offscreen_image_when_the_present_path_created_one() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let mut app = headless_windowed_app();
    with_present_path(&mut app);
    wire_pump(&mut app, tmp.path().to_path_buf());
    app.world_mut()
        .spawn((Camera2d, crate::states::running::UiCamera));
    settle(&mut app);

    enqueue(&mut app);
    app.update();
    app.update();
    let target = spawned_screenshot_target(&mut app);
    assert!(
        matches!(target, Some(RenderTarget::Image(_))),
        "with a capture target the game's pump must capture the offscreen image, got {target:?}",
    );
}

#[test]
fn the_pump_falls_back_to_the_window_without_a_target() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let mut app = headless_windowed_app();
    wire_pump(&mut app, tmp.path().to_path_buf());

    enqueue(&mut app);
    app.update();
    app.update();
    let target = spawned_screenshot_target(&mut app);
    assert!(
        matches!(target, Some(RenderTarget::Window(_))),
        "with no capture target the pump must fall back to the window, got {target:?}",
    );
}
