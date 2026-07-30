//! GTW-918: the RUNNING editor captures offscreen — asserted on the app the binary builds.
//!
//! The `#![cfg(all(debug_assertions, feature = "net_qa"))]` gate (below, after this crate doc so
//! the doc survives a feature-off build — the GTW-804 `net_qa_hello` precedent) compiles the
//! whole dir-form suite to an empty crate without the feature. Run it with the feature on:
//! `cargo test -p gdtf_content_editor --features net_qa --test net_qa_editor_present`.
//!
//! The in-src tests (`src/net_qa/present/test/`) pin each piece of the wiring on small apps that
//! carry the present plugin alone. This suite answers the question those cannot: does the EDITOR
//! — its own `MapEditorPlugin`, its own `EguiPlugin`, its own `NetQaEditorPlugin`, its own
//! `OnEnter(Editing)` camera spawn, reaching `Editing` through its real `Load` pass — end up
//! capturing offscreen, with nothing in the test inserting the source or the target?
//!
//! ## Members (one concern per file)
//!
//! - [`support`] — the shared error aliases.
//! - [`harness`] — the real editor app with a real primary window, and the drive-to-`Editing`
//!   driver.

#![cfg(all(debug_assertions, feature = "net_qa"))]

mod harness;
mod support;

use bevy::{camera::RenderTarget, prelude::*};
use bevy_egui::PrimaryEguiContext;
use gdtf_content_editor::EditorShotSource;

use crate::{
    harness::{HARNESS_SCALE_FACTOR, advance_to_editing, windowed_editor_app},
    support::TestResult,
};

/// Clause 1 — the running editor captures OFFSCREEN by default, and clauses 4/5/7 hold on the
/// real editor camera.
///
/// Nothing here inserts an [`EditorShotSource`] or a capture image: the editor's own listener-arm
/// wiring creates the target and names it. The assertions, in order:
///
/// 1. The capture source is `Offscreen(..)` — never `PrimaryWindow`.
/// 2. The editor's egui camera — the entity carrying [`PrimaryEguiContext`], spawned
///    window-targeted by `OnEnter(Editing)` — now carries `RenderTarget::Image`.
/// 3. That image is the very one the capture source names, so the pump reads the pixels this
///    camera writes.
/// 4. The image render target carries the WINDOW's scale factor, not `ImageRenderTarget`'s
///    `From`-impl `1.0` — which for egui governs both the rendered size and where clicks land.
/// 5. Exactly one entity holds the primary egui context, and it is that camera: the present
///    camera never stole it.
#[test]
fn the_running_editor_captures_the_offscreen_image_its_egui_camera_renders_into() -> TestResult {
    let (mut app, _port) = windowed_editor_app()?;
    advance_to_editing(&mut app);

    let source = app.world().get_resource::<EditorShotSource>();
    let named = match source {
        Some(EditorShotSource::Offscreen(handle)) => Some(handle.clone()),
        _ => None,
    };
    assert!(
        named.is_some(),
        "the running editor must capture through EditorShotSource::Offscreen with no caller \
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
         (GTW-922); camera aims at {aimed:?}, source names {named:?}",
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
