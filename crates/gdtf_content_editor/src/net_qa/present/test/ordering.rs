use bevy::{camera::RenderTarget, prelude::*, render::view::window::screenshot::Screenshot};
use gdtf_screenshot::{
    CaptureCompletions, CaptureOutcome, CapturePipelinePlugin, CaptureQueue, PollCap, SettleFrames,
    ShotDir, ShotStem,
};

use super::harness::{
    egui_camera_target, headless_windowed_app, record_egui_window_mapping,
    spawn_editor_like_camera, with_present_path,
};

const TEST_POLL_BUDGET: u32 = 3;

fn spawned_captures(app: &mut App) -> usize {
    let world = app.world_mut();
    let mut query = world.query::<&Screenshot>();
    query.iter(world).count()
}

fn refusals(app: &App) -> Vec<CaptureOutcome> {
    app.world()
        .resource::<CaptureCompletions<()>>()
        .iter()
        .map(|completion| completion.outcome().clone())
        .filter(|outcome| matches!(outcome, CaptureOutcome::Refused(_)))
        .collect()
}

#[test]
fn a_shot_settling_on_the_retarget_frame_is_captured_not_refused() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let mut app = headless_windowed_app();
    with_present_path(&mut app);
    app.add_plugins(CapturePipelinePlugin::<()>::new());
    app.insert_resource(ShotDir::new(tmp.path().to_path_buf()));
    app.insert_resource(SettleFrames::new(0));
    app.insert_resource(PollCap::new(TEST_POLL_BUDGET));
    let camera = spawn_editor_like_camera(&mut app);

    app.update();
    let before = egui_camera_target(&mut app);
    assert!(
        matches!(before, Some(RenderTarget::Window(_))),
        "test setup: the camera must still be window-targeted before the input-map entry exists, \
         so the retarget frame is still ahead of us; it aims at {before:?}",
    );

    app.world_mut()
        .resource_mut::<CaptureQueue<()>>()
        .push(Some(ShotStem::new("retarget_frame")), ());
    app.update();
    let claimed = egui_camera_target(&mut app);
    assert!(
        matches!(claimed, Some(RenderTarget::Window(_))),
        "test setup: the shot must be claimed while the camera is still window-targeted; it aims \
         at {claimed:?}",
    );
    assert_eq!(
        spawned_captures(&mut app),
        0,
        "test setup: the capture must not have spawned yet — its aim check belongs on the NEXT \
         frame, the retarget frame",
    );

    record_egui_window_mapping(&mut app, camera);
    app.update();

    let aimed = egui_camera_target(&mut app);
    assert!(
        matches!(aimed, Some(RenderTarget::Image(_))),
        "test setup: the retarget must have fired on this frame — that is the frame under test; \
         the camera aims at {aimed:?}",
    );
    assert!(
        refusals(&app).is_empty(),
        "a shot whose settle expired on the retarget frame must NOT be refused: the present set \
         runs before the capture set, so the pump sees the image the camera was just aimed at. \
         It finished {:?}",
        refusals(&app),
    );
    assert!(
        spawned_captures(&mut app) > 0,
        "the capture must have SPAWNED on the retarget frame rather than answering anything",
    );
}
