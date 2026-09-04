use bevy::{camera::RenderTarget, prelude::*, window::WindowRef};

use super::harness::{
    FRAME_BUDGET, a_capture_is_in_flight, drive_until, enqueue_capture, headless_capture_app,
    the_queue_is_idle,
};

fn target_of(app: &App, camera: Entity) -> Option<RenderTarget> {
    app.world().get::<RenderTarget>(camera).cloned()
}

#[test]
fn the_capture_path_leaves_a_window_camera_on_the_window() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let mut app = headless_capture_app(tmp.path());
    let camera = app
        .world_mut()
        .spawn((Camera2d, RenderTarget::Window(WindowRef::Primary)))
        .id();

    enqueue_capture(&mut app, "window_camera");

    assert!(
        drive_until(&mut app, a_capture_is_in_flight),
        "test setup: the pump must put a capture in flight inside {FRAME_BUDGET} frames",
    );
    let in_flight = target_of(&app, camera);
    assert!(
        matches!(in_flight, Some(RenderTarget::Window(_))),
        "a capture in flight must leave a window camera on the window; it found {in_flight:?}. \
         bevy_ui's ui_focus_system only writes Interaction for cameras rendering to a window, so \
         an image target here kills every button while a capture runs",
    );

    assert!(
        drive_until(&mut app, the_queue_is_idle),
        "test setup: the capture queue must reach idle inside {FRAME_BUDGET} frames",
    );
    app.update();
    let after = target_of(&app, camera);
    assert!(
        matches!(after, Some(RenderTarget::Window(_))),
        "the frame after the capture queue reports idle must still leave the camera on the \
         window; it found {after:?}",
    );
}
