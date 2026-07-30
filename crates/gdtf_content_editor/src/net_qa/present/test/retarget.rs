//! Headless pins for the retarget itself (GTW-918 clauses 4, 5 and 7): the `bevy_egui`
//! input-mapping gate that keeps the editor UI alive to input, the explicit scale factor, and
//! the primary egui context staying on the retargeted camera.

use bevy::{camera::RenderTarget, prelude::*};
use bevy_egui::PrimaryEguiContext;

use super::harness::{
    HARNESS_SCALE_FACTOR, headless_windowed_app, record_egui_window_mapping, settle,
    spawn_editor_like_camera,
};
use crate::net_qa::present::{
    EditorCapturePresentPlugin, blit::EditorQaPresentCamera, target::EditorQaCaptureTarget,
};

/// The retarget WAITS for `bevy_egui`'s `WindowToEguiContextMap` entry (clause 4).
///
/// Two assertions, one app each:
///
/// 1. With NO map entry the camera is left WINDOW-targeted. This is the one that matters:
///    `bevy_egui` records a context's window mapping once, on `Added<EguiContext>`, and only for
///    a window-targeted context, so retargeting before that observation would permanently kill
///    every pointer / keyboard / IME event reaching the editor UI.
/// 2. With the entry present the camera IS retargeted to the offscreen image — so the gate is a
///    delay, not a permanent refusal.
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
    let handle = gated
        .world()
        .get_resource::<EditorQaCaptureTarget>()
        .map(|t| (**t).clone());
    let target = gated.world().entity(camera).get::<RenderTarget>();
    let aimed = target.and_then(RenderTarget::as_image);
    assert!(
        handle.is_some() && aimed == handle.as_ref(),
        "once the bevy_egui input-map entry exists the editor camera must be retargeted to the \
         offscreen capture image; target {target:?}, capture image {handle:?}",
    );
}

/// The image render target carries the WINDOW's real scale factor, not `1.0` (clause 5).
///
/// `impl From<Handle<Image>> for ImageRenderTarget` hardcodes `1.0`, and egui derives its screen
/// rect from the camera's target scaling factor while pointer positions arrive in LOGICAL window
/// coordinates — so a `1.0` factor on a 2x display renders the UI at half apparent size in the
/// capture AND lands clicks at half the correct position. The harness window is deliberately at
/// scale factor `2.0`, so taking the `.into()` shortcut fails here.
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

/// The primary egui context stays on the RETARGETED camera and the present camera never carries
/// one (clause 7) — so egui still draws, into the capture image.
///
/// egui's context is a component on the camera entity, so this is what "egui renders into the
/// capture" means structurally. A GPU frame proving non-uniform pixels is left to GTW-904: this
/// harness runs with `backends: None` and cannot read an image back.
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
