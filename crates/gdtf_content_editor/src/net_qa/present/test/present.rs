use bevy::{
    app::App,
    prelude::*,
    render::render_resource::{TextureFormat, TextureUsages},
    window::PrimaryWindow,
    winit::{UpdateMode, WinitSettings},
};
use bevy_egui::EguiPrimaryContextPass;

use super::harness::{headless_windowed_app, settle};
use crate::net_qa::{
    present::{EditorCapturePresentPlugin, target::EditorQaCaptureTarget},
    screenshot::EditorShotSource,
};

#[test]
fn winit_settings_are_continuous_on_both_modes() {
    let mut app = App::new();
    app.add_plugins(EditorCapturePresentPlugin);
    let settings = app.world().get_resource::<WinitSettings>();
    assert!(
        settings.is_some_and(|s| s.focused_mode == UpdateMode::Continuous
            && s.unfocused_mode == UpdateMode::Continuous),
        "EditorCapturePresentPlugin must set WinitSettings to Continuous in both focus states, \
         got {settings:?}",
    );
}

#[test]
fn the_capture_target_is_window_sized_rgba_with_copy_src() {
    let mut app = headless_windowed_app();
    app.add_plugins(EditorCapturePresentPlugin);
    settle(&mut app);

    let mut windows = app
        .world_mut()
        .query_filtered::<&Window, With<PrimaryWindow>>();
    let expected = windows.iter(app.world()).next().map(Window::physical_size);
    let target = app.world().get_resource::<EditorQaCaptureTarget>();
    let images = app.world().resource::<Assets<Image>>();
    let image = target.and_then(|target| images.get(&target.handle));

    assert!(
        expected.is_some_and(|size| image.is_some_and(|img| img.size() == size)),
        "the editor capture target must exist at the window's physical size (expected \
         {expected:?})",
    );
    assert!(
        image.is_some_and(|img| img.texture_descriptor.format == TextureFormat::Rgba8UnormSrgb),
        "the editor capture target must be Rgba8UnormSrgb — the format bevy_egui's extract \
         picks for a non-HDR view",
    );
    assert!(
        image.is_some_and(|img| img
            .texture_descriptor
            .usage
            .contains(TextureUsages::COPY_SRC)),
        "the editor capture target must carry TextureUsages::COPY_SRC — without it \
         a Screenshot of it copies nothing and the capture silently never lands",
    );
}

#[test]
fn the_capture_source_names_the_created_target() {
    let mut app = headless_windowed_app();
    app.init_resource::<EditorShotSource>();
    app.add_plugins(EditorCapturePresentPlugin);
    settle(&mut app);

    let created = app
        .world()
        .get_resource::<EditorQaCaptureTarget>()
        .map(|target| (**target).clone());
    let source = app.world().get_resource::<EditorShotSource>();
    let named = match source {
        Some(EditorShotSource::Offscreen(target)) => Some(target.clone()),
        _ => None,
    };
    assert!(
        created.is_some() && named == created,
        "the present path must install EditorShotSource::Offscreen naming the whole target it \
         created (handle AND scale factor); created {created:?}, source {source:?}",
    );
}

#[test]
fn the_present_systems_run_in_update_never_in_the_egui_pass() {
    let mut app = headless_windowed_app();
    app.add_plugins(EditorCapturePresentPlugin);
    app.update();

    let names: Vec<String> = app.get_schedule(Update).map_or_else(Vec::new, |schedule| {
        schedule.systems().map_or_else(
            |_| Vec::new(),
            |systems| {
                systems
                    .map(|(_, system)| system.name().to_string())
                    .collect()
            },
        )
    });
    for expected in [
        "ensure_editor_capture_target",
        "retarget_editor_camera_to_offscreen",
        "spawn_editor_present_pass",
    ] {
        assert!(
            names.iter().any(|name| name.contains(expected)),
            "{expected} must be registered in Update; Update holds {names:?}",
        );
    }
    assert!(
        app.get_schedule(EguiPrimaryContextPass).is_none(),
        "EditorCapturePresentPlugin must add nothing to EguiPrimaryContextPass — it did not \
         even exist in this app before the plugin was added",
    );
}
