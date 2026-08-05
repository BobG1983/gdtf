use bevy::{
    app::App,
    prelude::*,
    render::render_resource::{TextureFormat, TextureUsages},
    window::PrimaryWindow,
    winit::{UpdateMode, WinitSettings},
};

use super::harness::{HARNESS_SCALE_FACTOR, capture_target, headless_windowed_app, settle};
use crate::{capture::CaptureSource, present::CapturePresentPlugin};

#[test]
fn winit_settings_are_continuous_on_both_modes() {
    let mut app = App::new();
    app.add_plugins(CapturePresentPlugin);
    let settings = app.world().get_resource::<WinitSettings>();
    assert!(
        settings.is_some_and(|winit| winit.focused_mode == UpdateMode::Continuous
            && winit.unfocused_mode == UpdateMode::Continuous),
        "CapturePresentPlugin must set WinitSettings to Continuous in both focus states, got \
         {settings:?}",
    );
}

#[test]
fn the_capture_target_is_window_sized_rgba_with_copy_src() {
    let mut app = headless_windowed_app();
    app.add_plugins(CapturePresentPlugin);
    settle(&mut app);

    let mut windows = app
        .world_mut()
        .query_filtered::<&Window, With<PrimaryWindow>>();
    let expected = windows.iter(app.world()).next().map(Window::physical_size);
    let target = capture_target(&app);
    let images = app.world().resource::<Assets<Image>>();
    let image = target
        .as_ref()
        .and_then(|target| images.get(&target.handle));

    assert!(
        expected.is_some_and(|size| image.is_some_and(|found| found.size() == size)),
        "the capture target must exist at the window's physical size (expected {expected:?})",
    );
    assert!(
        image.is_some_and(|found| found.texture_descriptor.format == TextureFormat::Rgba8UnormSrgb),
        "the capture target must be Rgba8UnormSrgb — the format bevy_egui's extract picks for a \
         non-HDR view",
    );
    assert!(
        image.is_some_and(|found| found
            .texture_descriptor
            .usage
            .contains(TextureUsages::COPY_SRC)),
        "the capture target must carry TextureUsages::COPY_SRC — without it a Screenshot of it \
         copies nothing and the capture silently never lands",
    );
}

#[test]
fn the_capture_target_carries_the_windows_own_scale_factor() {
    let mut app = headless_windowed_app();
    app.add_plugins(CapturePresentPlugin);
    settle(&mut app);

    let created = capture_target(&app);
    assert!(
        created.as_ref().map(|target| target.scale_factor) == Some(HARNESS_SCALE_FACTOR),
        "QaCaptureTarget must carry the window's scale factor ({HARNESS_SCALE_FACTOR}) alongside \
         its handle, never ImageRenderTarget's From-impl 1.0; it holds {created:?}",
    );
}

#[test]
fn the_capture_source_names_the_created_target() {
    let mut app = headless_windowed_app();
    app.add_plugins(CapturePresentPlugin);
    settle(&mut app);

    let created = capture_target(&app);
    let source = app.world().get_resource::<CaptureSource>();
    let named = match source {
        Some(CaptureSource::Offscreen(target)) => Some(target.clone()),
        _ => None,
    };
    assert!(
        created.is_some() && named == created,
        "the present path must install CaptureSource::Offscreen naming the whole target it \
         created (handle AND scale factor); created {created:?}, source {source:?}",
    );
}
