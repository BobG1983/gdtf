//! Headless tests for GTW-764 C4: the T7 pump captures the OFFSCREEN image when the
//! [`QaCaptureTarget`] exists, and falls back to the window swapchain when it does not.
//!
//! Asserts on the spawned [`Screenshot`] component's target (a pure-ECS check — no GPU
//! needed), so it catches the capture-source choice directly. No unwrap / expect / panic (the
//! `net_qa` suite convention).

use bevy::{camera::RenderTarget, prelude::*, render::view::window::screenshot::Screenshot};
use gdtf_net_qa_transport::{PendingQueue, Responder};

use super::harness::headless_windowed_app;
use crate::dev::net_qa::{
    present::CapturePresentPlugin,
    screenshot::{
        InFlightShots, QaShotDir, ScreenshotPayload, ShotPollBudget, ShotSequence,
        drive_screenshots,
    },
};

/// Register the T7 pump's resources (confined to `dir`, tiny budget) + the pump system so a
/// pushed [`ScreenshotPayload`] is claimed and its capture spawned.
fn wire_pump(app: &mut App, dir: std::path::PathBuf) {
    app.init_resource::<PendingQueue<ScreenshotPayload>>();
    app.init_resource::<InFlightShots>();
    app.init_resource::<ShotSequence>();
    app.insert_resource(QaShotDir::new(dir));
    app.insert_resource(ShotPollBudget::new(2));
    app.add_systems(Update, drive_screenshots);
}

/// Push a screenshot request exactly as the router would.
fn enqueue(app: &mut App) {
    let (responder, _reply) = Responder::channel();
    app.world_mut()
        .resource_mut::<PendingQueue<ScreenshotPayload>>()
        .push_new(ScreenshotPayload::new(None), responder);
}

/// The `RenderTarget` of the single spawned [`Screenshot`], if exactly one exists.
fn spawned_screenshot_target(app: &mut App) -> Option<RenderTarget> {
    let mut shots = app.world_mut().query::<&Screenshot>();
    let targets: Vec<RenderTarget> = shots.iter(app.world()).map(|s| s.0.clone()).collect();
    match targets.as_slice() {
        [one] => Some(one.clone()),
        _ => None,
    }
}

/// With the present path active (a [`QaCaptureTarget`] exists), the pump captures the OFFSCREEN
/// image (`Screenshot::image` → `RenderTarget::Image`), never the window swapchain — the fix
/// for the black backgrounded-window capture.
#[test]
fn pump_captures_the_offscreen_image_when_the_target_exists() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        return;
    };
    let mut app = headless_windowed_app();
    app.add_plugins(CapturePresentPlugin);
    // Let the present path create the offscreen target before the pump claims.
    for _ in 0..3 {
        app.update();
    }
    wire_pump(&mut app, tmp.path().to_path_buf());
    enqueue(&mut app);
    app.update();
    let target = spawned_screenshot_target(&mut app);
    assert!(
        matches!(target, Some(RenderTarget::Image(_))),
        "with a QaCaptureTarget the pump must capture the offscreen image, got {target:?}",
    );
}

/// With NO present path (no [`QaCaptureTarget`]), the pump falls back to the window swapchain
/// (`Screenshot::primary_window` → `RenderTarget::Window`) — a build without the offscreen
/// path is unchanged.
#[test]
fn pump_falls_back_to_the_window_without_a_target() {
    let Ok(tmp) = tempfile::TempDir::new() else {
        return;
    };
    let mut app = headless_windowed_app();
    wire_pump(&mut app, tmp.path().to_path_buf());
    enqueue(&mut app);
    app.update();
    let target = spawned_screenshot_target(&mut app);
    assert!(
        matches!(target, Some(RenderTarget::Window(_))),
        "with no QaCaptureTarget the pump must fall back to the window, got {target:?}",
    );
}
