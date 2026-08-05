//! Editor present-command net-QA integration (debug + `net_qa`).
#![cfg(debug_assertions)]

mod harness;
mod support;

use bevy::{camera::RenderTarget, prelude::*};
use bevy_egui::PrimaryEguiContext;
use gdtf_screenshot::CaptureSource;

use crate::{
    harness::{HARNESS_SCALE_FACTOR, advance_to_editing, windowed_editor_app},
    support::TestResult,
};

#[test]
fn the_running_editor_captures_the_offscreen_image_its_egui_camera_renders_into() -> TestResult {
    let (mut app, _port) = windowed_editor_app()?;
    advance_to_editing(&mut app);

    let source = app.world().get_resource::<CaptureSource>();
    let named = match source {
        Some(CaptureSource::Offscreen(handle)) => Some(handle.clone()),
        _ => None,
    };
    assert!(
        named.is_some(),
        "the running editor must capture through CaptureSource::Offscreen with no caller \
         inserting it; it holds {source:?}",
    );

    let mut cameras = app
        .world_mut()
        .query_filtered::<Entity, With<PrimaryEguiContext>>();
    let egui_cameras: Vec<Entity> = cameras.iter(app.world()).collect();
    let [egui_camera] = egui_cameras.as_slice() else {
        return Err(format!(
            "the editor must have exactly one primary egui context, on its own camera; found \
             {egui_cameras:?}"
        )
        .into());
    };

    let target = app.world().entity(*egui_camera).get::<RenderTarget>();
    assert!(
        matches!(target, Some(RenderTarget::Image(_))),
        "the editor's egui camera must be retargeted to an image once bevy_egui has recorded \
         its input mapping, got {target:?}",
    );
    let aimed = match target {
        Some(RenderTarget::Image(image)) => Some(image.clone()),
        _ => None,
    };
    assert!(
        aimed.is_some() && aimed == named,
        "the editor's egui camera must render into the very render target the capture source \
         names — handle AND scale factor, since Bevy keys a view's output attachment by both \
         ; camera aims at {aimed:?}, source names {named:?}",
    );
    let scale = match target {
        Some(RenderTarget::Image(image)) => Some(image.scale_factor),
        _ => None,
    };
    assert!(
        scale == Some(HARNESS_SCALE_FACTOR),
        "the image render target must carry the window's scale factor ({HARNESS_SCALE_FACTOR}), \
         not ImageRenderTarget's From-impl 1.0 — egui derives its screen rect and its pointer \
         mapping from it; got {scale:?}",
    );
    Ok(())
}
