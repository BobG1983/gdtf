use bevy::{prelude::*, render::gpu_readback::Readback, window::PrimaryWindow};

use super::harness::{
    FRAME_BUDGET, a_capture_is_in_flight, drive_until, enqueue_capture, headless_capture_app,
};
use crate::window_capture::CaptureImage;

fn readbacks(app: &mut App) -> Vec<Readback> {
    let world = app.world_mut();
    let mut query = world.query::<&Readback>();
    query.iter(world).cloned().collect()
}

fn primary_window_px(app: &mut App) -> UVec2 {
    let world = app.world_mut();
    let mut windows = world.query_filtered::<&Window, With<PrimaryWindow>>();
    let Ok(window) = windows.single(world) else {
        unreachable!("the harness builds exactly one primary window");
    };
    window.physical_size()
}

#[test]
fn a_capture_reads_the_window_sized_image_the_plugin_owns() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        unreachable!("a temp directory is available");
    };
    let mut app = headless_capture_app(tmp.path());
    enqueue_capture(&mut app, "readback_target");

    assert!(
        drive_until(&mut app, a_capture_is_in_flight),
        "test setup: the pump must put a capture in flight inside {FRAME_BUDGET} frames",
    );

    let found = readbacks(&mut app);
    assert_eq!(
        found.len(),
        1,
        "a capture in flight must queue exactly one readback, not {}: {found:?}",
        found.len(),
    );
    let owned = app.world().resource::<CaptureImage>().clone();
    let reads_the_capture_image = found
        .iter()
        .any(|readback| matches!(readback, Readback::Texture(handle) if handle == &*owned));
    assert!(
        reads_the_capture_image,
        "the capture must read the image the plugin owns, {:?}, not {found:?} — a readback on any \
         other handle comes back empty and the capture times out with no PNG",
        *owned,
    );

    let window_px = primary_window_px(&mut app);
    let held = app
        .world()
        .resource::<Assets<Image>>()
        .get(&*owned)
        .map(Image::size);
    assert_eq!(
        held,
        Some(window_px),
        "the image a capture reads must be the primary window's own physical size, so the \
         attachment override is a size the window can render into",
    );
}
