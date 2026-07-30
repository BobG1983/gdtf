//! Headless pins for the present pass that blits the offscreen image back onto the window
//! (GTW-922 clauses 5 and 6).
//!
//! GTW-918 asserted none of this: no test read `PRESENT_LAYER`, `PRESENT_ORDER` or the blit
//! `Sprite`, so the pass could have rendered on the editor's own layer, under the editor camera,
//! or with no sprite at all and the suite stayed green. The pass is also now GATED on the
//! retarget having happened, which is the clause-5 resolution, and that gate is asserted in both
//! directions here.

use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    prelude::*,
};

use super::harness::{
    capture_target, headless_windowed_app, record_egui_window_mapping, settle,
    spawn_editor_like_camera,
};
use crate::net_qa::present::{
    EditorCapturePresentPlugin,
    blit::{EditorQaPresentCamera, EditorQaPresentSprite, PRESENT_LAYER, PRESENT_ORDER},
};

/// Build the present path in the state where the retarget HAS fired: an editor-shaped camera with
/// its `bevy_egui` input mapping already recorded, so the offscreen target is created and the
/// camera is aimed at it.
fn app_with_retargeted_camera() -> App {
    let mut app = headless_windowed_app();
    app.add_plugins(EditorCapturePresentPlugin);
    let camera = spawn_editor_like_camera(&mut app);
    record_egui_window_mapping(&mut app, camera);
    settle(&mut app);
    app
}

/// A present camera targeting the WINDOW exists once the retarget has fired, so a focused editor
/// still shows its UI — and what the window shows equals what the pump captures.
#[test]
fn a_present_camera_targets_the_window() {
    let mut app = app_with_retargeted_camera();

    let mut present = app
        .world_mut()
        .query_filtered::<&RenderTarget, With<EditorQaPresentCamera>>();
    let targets: Vec<RenderTarget> = present.iter(app.world()).cloned().collect();
    assert!(
        targets.len() == 1 && matches!(targets.first(), Some(RenderTarget::Window(_))),
        "exactly one present camera must target the window, got {targets:?}",
    );
}

/// The present camera composites LAST, on a layer of its own (clause 6).
///
/// [`PRESENT_ORDER`] must be above the editor camera's `0` and the prefab preview camera's `-1`,
/// so the blit runs after the egui pass has written the offscreen image this frame; and
/// [`PRESENT_LAYER`] must be the camera's ONLY layer, disjoint from the editor's default `0` and
/// the preview's `1`, so the present camera renders the blit sprite and nothing else.
#[test]
fn the_present_camera_composites_last_on_its_own_layer() {
    let mut app = app_with_retargeted_camera();

    let mut present = app
        .world_mut()
        .query_filtered::<(&Camera, &RenderLayers), With<EditorQaPresentCamera>>();
    let found: Vec<(isize, RenderLayers)> = present
        .iter(app.world())
        .map(|(camera, layers)| (camera.order, layers.clone()))
        .collect();
    let [(order, layers)] = found.as_slice() else {
        unreachable!("exactly one present camera exists: {found:?}");
    };
    assert!(
        *order == PRESENT_ORDER && *order > 0,
        "the present camera must carry PRESENT_ORDER ({PRESENT_ORDER}), above the editor camera's \
         0 and the preview camera's -1, so the blit composites after the egui pass; got {order}",
    );
    assert!(
        *layers == RenderLayers::layer(PRESENT_LAYER) && PRESENT_LAYER > 1,
        "the present camera must render ONLY PRESENT_LAYER ({PRESENT_LAYER}), disjoint from the \
         editor camera's default 0 and the preview camera's 1; got {layers:?}",
    );
}

/// The blit SPRITE exists, shows the capture image, and lives on the present camera's layer
/// (clause 6).
///
/// Without it the present camera renders an empty pass and the window goes blank while the
/// capture still works — a state no GTW-918 assertion could see.
#[test]
fn the_blit_sprite_shows_the_capture_image_on_the_present_layer() {
    let mut app = app_with_retargeted_camera();
    let created = capture_target(&app);

    let mut sprites = app
        .world_mut()
        .query_filtered::<(&Sprite, &RenderLayers), With<EditorQaPresentSprite>>();
    let found: Vec<(Handle<Image>, Option<Vec2>, RenderLayers)> = sprites
        .iter(app.world())
        .map(|(sprite, layers)| (sprite.image.clone(), sprite.custom_size, layers.clone()))
        .collect();
    let [(image, size, layers)] = found.as_slice() else {
        unreachable!("exactly one blit sprite must exist: {found:?}");
    };
    assert!(
        created
            .as_ref()
            .is_some_and(|target| *image == target.handle),
        "the blit sprite must display the capture target's own image; it shows {image:?} against \
         target {created:?}",
    );
    assert!(
        *layers == RenderLayers::layer(PRESENT_LAYER),
        "the blit sprite must live on PRESENT_LAYER ({PRESENT_LAYER}) so only the present camera \
         renders it; got {layers:?}",
    );
    let mut windows = app
        .world_mut()
        .query_filtered::<&Window, With<bevy::window::PrimaryWindow>>();
    let expected = windows.iter(app.world()).next().map(Window::size);
    assert!(
        *size == expected,
        "the blit sprite must be sized to the window's LOGICAL size ({expected:?}) so the \
         physical-resolution image fills the window 1:1; got {size:?}",
    );
}

/// The present pass is NOT spawned while the editor's egui camera is still window-targeted
/// (clause 5, resolved: the pass IS gated on the retarget).
///
/// Today's failure mode without this gate: the retarget waits for `bevy_egui`'s input mapping and
/// may never fire, while an `order: 100` camera composites the untouched offscreen image over the
/// editor's own window — a permanently blank editor. Gated, the window keeps showing the egui
/// camera's own output and the editor stays usable; the capture's own problem is reported by the
/// pump's typed refusal instead.
#[test]
fn no_present_pass_appears_until_the_camera_is_retargeted() {
    let mut app = headless_windowed_app();
    app.add_plugins(EditorCapturePresentPlugin);
    // An editor-shaped camera with NO `bevy_egui` input-map entry, so the retarget never fires.
    let camera = spawn_editor_like_camera(&mut app);
    settle(&mut app);

    assert!(
        capture_target(&app).is_some(),
        "the offscreen target must still be created — the gate is on the RETARGET, not on the \
         target",
    );
    let still_window = app
        .world()
        .entity(camera)
        .get::<RenderTarget>()
        .is_some_and(|target| matches!(target, RenderTarget::Window(_)));
    assert!(
        still_window,
        "test setup: with no input-map entry the editor camera must still be window-targeted",
    );

    let mut present = app
        .world_mut()
        .query_filtered::<Entity, With<EditorQaPresentCamera>>();
    let cameras: Vec<Entity> = present.iter(app.world()).collect();
    let mut sprites = app
        .world_mut()
        .query_filtered::<Entity, With<EditorQaPresentSprite>>();
    let blits: Vec<Entity> = sprites.iter(app.world()).collect();
    assert!(
        cameras.is_empty() && blits.is_empty(),
        "no present camera and no blit sprite may exist while the egui camera still draws to the \
         window — compositing an untouched image over it would blank the editor; got cameras \
         {cameras:?}, sprites {blits:?}",
    );
}
