//! Headless pins for the GTW-918 [`EditorCapturePresentPlugin`] wiring: the `WinitSettings`
//! override, the offscreen target's size / format / `COPY_SRC` usage, the capture source that
//! names it, the present camera targeting the window, and the schedule the systems live in.
//!
//! No unwrap / expect / panic even in tests (the `net_qa` suite convention) — shape checks use
//! `assert!(matches!(…))` / `assert!(….is_some_and(…))`. No non-black-pixel assertion: with
//! `backends: None` there is no GPU, so that is in-engine QA (GTW-904 owns it).

use bevy::{
    app::App,
    camera::RenderTarget,
    prelude::*,
    render::render_resource::{TextureFormat, TextureUsages},
    window::PrimaryWindow,
    winit::{UpdateMode, WinitSettings},
};
use bevy_egui::EguiPrimaryContextPass;

use super::harness::{headless_windowed_app, settle};
use crate::net_qa::{
    present::{
        EditorCapturePresentPlugin, blit::EditorQaPresentCamera, target::EditorQaCaptureTarget,
    },
    screenshot::EditorShotSource,
};

/// The present plugin overrides `WinitSettings` to continuous-in-both-focus-states (clause 3),
/// so a fresh offscreen frame renders every tick regardless of window focus and a capture never
/// races a reactive-low-power throttle.
#[test]
fn winit_settings_are_continuous_on_both_modes() {
    // A bare app is enough — `build` inserts the resource without any system running.
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

/// The offscreen [`EditorQaCaptureTarget`] image is created at the primary window's PHYSICAL
/// size, as `Rgba8UnormSrgb`, and carries `COPY_SRC` — the single silent-failure gotcha
/// (clause 3): without `COPY_SRC`, `Screenshot::image` never lands a file.
///
/// MUTATION PROOF: deleting the `image.texture_descriptor.usage |= TextureUsages::COPY_SRC;`
/// line in `present/target.rs` fails THIS assertion (demonstrated during GTW-918).
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
    let image = target.and_then(|t| images.get(&**t));

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
         Screenshot::image copies nothing and the capture silently never lands",
    );
}

/// The present path points [`EditorShotSource`] at the target it created (clause 1's mechanism):
/// the running editor captures the image its camera renders into, not the placeholder handle the
/// plugin's `init_resource` default carries and not the window swapchain.
#[test]
fn the_capture_source_names_the_created_target() {
    let mut app = headless_windowed_app();
    app.init_resource::<EditorShotSource>();
    app.add_plugins(EditorCapturePresentPlugin);
    settle(&mut app);

    let created = app
        .world()
        .get_resource::<EditorQaCaptureTarget>()
        .map(|t| (**t).clone());
    let source = app.world().get_resource::<EditorShotSource>();
    let named = match source {
        Some(EditorShotSource::Offscreen(handle)) => Some(handle.clone()),
        _ => None,
    };
    assert!(
        created.is_some() && named == created,
        "the present path must install EditorShotSource::Offscreen naming the target it \
         created; created {created:?}, source {source:?}",
    );
}

/// A present camera targeting the WINDOW exists (clause 3), so a focused editor still shows its
/// UI — and what the window shows equals what the pump captures.
#[test]
fn a_present_camera_targets_the_window() {
    let mut app = headless_windowed_app();
    app.add_plugins(EditorCapturePresentPlugin);
    settle(&mut app);

    let mut present = app
        .world_mut()
        .query_filtered::<&RenderTarget, With<EditorQaPresentCamera>>();
    let targets: Vec<&RenderTarget> = present.iter(app.world()).collect();
    assert!(
        targets.len() == 1 && matches!(targets[0], RenderTarget::Window(_)),
        "exactly one present camera must target the window, got {targets:?}",
    );
}

/// All three present systems live in [`Update`], and the plugin adds NOTHING to
/// [`EguiPrimaryContextPass`] (clause 6 / bevy-traps #8 — that schedule can run twice per frame
/// under multipass, and creating a render target or spawning a camera twice a frame is not the
/// kind of thing that survives it).
#[test]
fn the_present_systems_run_in_update_never_in_the_egui_pass() {
    let mut app = headless_windowed_app();
    app.add_plugins(EditorCapturePresentPlugin);
    // `Schedule::systems` needs an initialized executor, which one run provides.
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
