//! Headless tests for the GTW-764 [`CapturePresentPlugin`] wiring: the `WinitSettings`
//! override, the offscreen target's size / format / `COPY_SRC` usage, the camera retarget,
//! and the present camera targeting the window.
//!
//! No unwrap / expect / panic even in tests (the `net_qa` suite convention) — shape checks use
//! `assert!(matches!(…))` / `assert!(… .is_some_and(…))`. No non-black-pixel assertion: with
//! `backends: None` there is no GPU, so that is in-engine QA (the orchestrator runs it).

use bevy::{
    camera::RenderTarget,
    prelude::*,
    render::render_resource::{TextureFormat, TextureUsages},
    ui::IsDefaultUiCamera,
    window::PrimaryWindow,
    winit::{UpdateMode, WinitSettings},
};
use gdtf_battle_presenter::WorldCamera;

use super::harness::headless_windowed_app;
use crate::{
    dev::net_qa::present::{CapturePresentPlugin, QaCaptureTarget},
    states::running::UiCamera,
};

/// The present plugin overrides `WinitSettings` to continuous-in-both-focus-states (C5), so a
/// fresh offscreen frame renders every tick regardless of window focus.
#[test]
fn winit_settings_are_continuous_under_net_qa() {
    // A bare app is enough — `build` inserts the resource without any system running.
    let mut app = App::new();
    app.add_plugins(CapturePresentPlugin);
    let settings = app.world().get_resource::<WinitSettings>();
    assert!(
        settings.is_some_and(|s| s.focused_mode == UpdateMode::Continuous
            && s.unfocused_mode == UpdateMode::Continuous),
        "CapturePresentPlugin must set WinitSettings to Continuous in both focus states, got \
         {settings:?}",
    );
}

/// The offscreen [`QaCaptureTarget`] image is created at the primary window's PHYSICAL size,
/// as `Rgba8UnormSrgb`, and carries `COPY_SRC` — the single silent-failure gotcha (C1): a
/// missing `COPY_SRC` makes `Screenshot::image` never land.
#[test]
fn capture_target_is_window_sized_rgba_with_copy_src() {
    let mut app = headless_windowed_app();
    app.add_plugins(CapturePresentPlugin);
    // A couple of frames: the target is created on the first tick a window exists, then the
    // insert-resource command applies.
    for _ in 0..3 {
        app.update();
    }
    let mut windows = app
        .world_mut()
        .query_filtered::<&Window, With<PrimaryWindow>>();
    let expected = windows.iter(app.world()).next().map(Window::physical_size);
    let target = app.world().get_resource::<QaCaptureTarget>();
    let images = app.world().resource::<Assets<Image>>();
    let descriptor = target.and_then(|t| images.get(&**t));
    assert!(
        expected.is_some_and(|size| descriptor.is_some_and(|img| {
            img.size() == size
                && img.texture_descriptor.format == TextureFormat::Rgba8UnormSrgb
                && img
                    .texture_descriptor
                    .usage
                    .contains(TextureUsages::COPY_SRC)
        })),
        "the capture target must exist at the window physical size, Rgba8UnormSrgb, with \
         COPY_SRC set (expected size {expected:?})",
    );
}

/// After the retarget system runs, BOTH a world camera and a UI camera carry
/// `RenderTarget::Image` pointing at the offscreen target (C2) — so the HUD (a `bevy_ui` tree
/// bound to the UI camera) is captured, not just the world.
#[test]
fn world_and_ui_cameras_are_retargeted_to_the_offscreen_image() {
    let mut app = headless_windowed_app();
    app.add_plugins(CapturePresentPlugin);
    let world_cam = app.world_mut().spawn((Camera2d, WorldCamera)).id();
    let ui_cam = app.world_mut().spawn((Camera2d, UiCamera)).id();
    for _ in 0..3 {
        app.update();
    }
    let target_handle = (**app.world().resource::<QaCaptureTarget>()).clone();
    for (label, entity) in [("world", world_cam), ("ui", ui_cam)] {
        let render_target = app.world().entity(entity).get::<RenderTarget>();
        assert!(
            render_target.and_then(RenderTarget::as_image) == Some(&target_handle),
            "the {label} camera must be retargeted to the offscreen capture image, got \
             {render_target:?}",
        );
    }
}

/// A present camera targeting the WINDOW exists (C3), so the window still shows the game (and
/// what it shows equals what the pump captures).
#[test]
fn a_present_camera_targets_the_window() {
    let mut app = headless_windowed_app();
    app.add_plugins(CapturePresentPlugin);
    for _ in 0..3 {
        app.update();
    }
    let mut present = app
        .world_mut()
        .query_filtered::<&RenderTarget, With<super::super::blit::QaPresentCamera>>();
    let targets: Vec<&RenderTarget> = present.iter(app.world()).collect();
    assert!(
        targets.len() == 1 && matches!(targets[0], RenderTarget::Window(_)),
        "exactly one present camera must target the window, got {targets:?}",
    );
}

/// The `UiCamera` is marked `IsDefaultUiCamera` and NO other camera is — so the `bevy_ui` HUD
/// binds to the offscreen UI camera (renders into the capture) rather than the order-100
/// present camera (which renders only its own layer, culling the HUD). Regression guard for the
/// in-engine defect where the HUD was missing from the capture after the present camera was
/// added.
#[test]
fn only_the_ui_camera_is_the_default_ui_camera() {
    let mut app = headless_windowed_app();
    app.add_plugins(CapturePresentPlugin);
    let world_cam = app.world_mut().spawn((Camera2d, WorldCamera)).id();
    let ui_cam = app.world_mut().spawn((Camera2d, UiCamera)).id();
    for _ in 0..3 {
        app.update();
    }
    let mut defaults = app
        .world_mut()
        .query_filtered::<Entity, With<IsDefaultUiCamera>>();
    let marked: Vec<Entity> = defaults.iter(app.world()).collect();
    assert!(
        marked == [ui_cam],
        "only the UI camera must be the default UI camera (ui={ui_cam:?}, world={world_cam:?}), \
         got {marked:?}",
    );
}
