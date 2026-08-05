use bevy::{camera::RenderTarget, prelude::*, ui::IsDefaultUiCamera};
use gdtf_battle_presenter::WorldCamera;
use gdtf_screenshot::QaCaptureTarget;

use super::harness::{HARNESS_SCALE_FACTOR, headless_windowed_app, settle, with_present_path};
use crate::states::running::UiCamera;

#[test]
fn world_and_ui_cameras_are_retargeted_to_the_whole_capture_target() {
    let mut app = headless_windowed_app();
    with_present_path(&mut app);
    let world_cam = app.world_mut().spawn((Camera2d, WorldCamera)).id();
    let ui_cam = app.world_mut().spawn((Camera2d, UiCamera)).id();
    settle(&mut app);

    let created = (**app.world().resource::<QaCaptureTarget>()).clone();
    for (label, entity) in [("world", world_cam), ("ui", ui_cam)] {
        let aimed = match app.world().entity(entity).get::<RenderTarget>() {
            Some(RenderTarget::Image(image)) => Some(image.clone()),
            _ => None,
        };
        assert!(
            aimed.as_ref() == Some(&created),
            "the {label} camera must aim at the whole capture target — handle AND scale factor \
             {HARNESS_SCALE_FACTOR}, since Bevy keys a view's output attachment by both; it aims \
             at {aimed:?} against {created:?}",
        );
    }
}

#[test]
fn only_the_ui_camera_is_the_default_ui_camera() {
    let mut app = headless_windowed_app();
    with_present_path(&mut app);
    let world_cam = app.world_mut().spawn((Camera2d, WorldCamera)).id();
    let ui_cam = app.world_mut().spawn((Camera2d, UiCamera)).id();
    settle(&mut app);

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
