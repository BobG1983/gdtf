use bevy::{camera::RenderTarget, prelude::*};
use bevy_egui::PrimaryEguiContext;

use super::harness::{
    HARNESS_SCALE_FACTOR, capture_target, egui_camera_target, headless_windowed_app,
    record_egui_window_mapping, settle, spawn_editor_like_camera,
};
use crate::net_qa::present::{EditorCapturePresentPlugin, blit::EditorQaPresentCamera};

#[test]
fn the_retarget_waits_for_the_egui_input_mapping() {
    let mut ungated = headless_windowed_app();
    ungated.add_plugins(EditorCapturePresentPlugin);
    let camera = spawn_editor_like_camera(&mut ungated);
    settle(&mut ungated);
    let target = ungated.world().entity(camera).get::<RenderTarget>();
    assert!(
        matches!(target, Some(RenderTarget::Window(_))),
        "with no bevy_egui input-map entry the editor camera must stay WINDOW-targeted \
         (retargeting first would leave the UI dead to input), got {target:?}",
    );

    let mut gated = headless_windowed_app();
    gated.add_plugins(EditorCapturePresentPlugin);
    let camera = spawn_editor_like_camera(&mut gated);
    record_egui_window_mapping(&mut gated, camera);
    settle(&mut gated);
    let created = capture_target(&gated);
    let target = gated.world().entity(camera).get::<RenderTarget>();
    let aimed = match target {
        Some(RenderTarget::Image(image)) => Some(image.clone()),
        _ => None,
    };
    assert!(
        created.is_some() && aimed == created,
        "once the bevy_egui input-map entry exists the editor camera must be retargeted to the \
         offscreen capture target; camera aims at {aimed:?}, target is {created:?}",
    );
}

#[test]
fn the_camera_aims_at_the_whole_capture_target_value() {
    let mut app = headless_windowed_app();
    app.add_plugins(EditorCapturePresentPlugin);
    let camera = spawn_editor_like_camera(&mut app);
    record_egui_window_mapping(&mut app, camera);
    settle(&mut app);

    let created = capture_target(&app);
    let aimed = egui_camera_target(&mut app);
    let same = match (&aimed, &created) {
        (Some(RenderTarget::Image(left)), Some(right)) => left == right,
        _ => false,
    };
    assert!(
        same,
        "the editor camera must aim at exactly the ImageRenderTarget the present path created \
         (handle AND scale factor); it aims at {aimed:?}, the target is {created:?}",
    );
}

#[test]
fn the_image_render_target_carries_the_windows_scale_factor() {
    let mut app = headless_windowed_app();
    app.add_plugins(EditorCapturePresentPlugin);
    let camera = spawn_editor_like_camera(&mut app);
    record_egui_window_mapping(&mut app, camera);
    settle(&mut app);

    let target = app.world().entity(camera).get::<RenderTarget>();
    let scale = match target {
        Some(RenderTarget::Image(image)) => Some(image.scale_factor),
        _ => None,
    };
    assert!(
        scale == Some(HARNESS_SCALE_FACTOR),
        "the retarget must write the window's own scale factor ({HARNESS_SCALE_FACTOR}) into \
         ImageRenderTarget, never the From-impl's 1.0; got {scale:?}",
    );
}

#[test]
fn the_capture_target_resource_holds_the_windows_scale_factor() {
    let mut app = headless_windowed_app();
    app.add_plugins(EditorCapturePresentPlugin);
    settle(&mut app);

    let created = capture_target(&app);
    assert!(
        created.as_ref().map(|target| target.scale_factor) == Some(HARNESS_SCALE_FACTOR),
        "EditorQaCaptureTarget must carry the window's scale factor ({HARNESS_SCALE_FACTOR}) \
         alongside its handle; it holds {created:?}",
    );
}

#[test]
fn the_primary_egui_context_stays_on_the_retargeted_camera() {
    let mut app = headless_windowed_app();
    app.add_plugins(EditorCapturePresentPlugin);
    let camera = spawn_editor_like_camera(&mut app);
    record_egui_window_mapping(&mut app, camera);
    settle(&mut app);

    let mut contexts = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryEguiContext>>();
    let holders: Vec<Entity> = contexts.iter(app.world()).collect();
    assert!(
        holders == [camera],
        "the primary egui context must stay on the editor camera alone (expected {camera:?}), \
         got {holders:?}",
    );

    let mut present = app
        .world_mut()
        .query_filtered::<Entity, With<EditorQaPresentCamera>>();
    let present_cameras: Vec<Entity> = present.iter(app.world()).collect();
    assert!(
        present_cameras.len() == 1 && !present_cameras.contains(&camera),
        "the present camera must be a SEPARATE entity from the egui camera, got \
         {present_cameras:?} against {camera:?}",
    );

    let aimed = app
        .world()
        .entity(camera)
        .get::<RenderTarget>()
        .and_then(RenderTarget::as_image)
        .is_some();
    assert!(
        aimed,
        "the camera holding the primary egui context must be the one aimed at the offscreen \
         image — that is what puts egui's output in the capture",
    );
}
