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

#[test]
fn winit_settings_are_continuous_under_net_qa() {
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

#[test]
fn capture_target_is_window_sized_rgba_with_copy_src() {
    let mut app = headless_windowed_app();
    app.add_plugins(CapturePresentPlugin);
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
