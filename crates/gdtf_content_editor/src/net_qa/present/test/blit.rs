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

fn app_with_retargeted_camera() -> App {
    let mut app = headless_windowed_app();
    app.add_plugins(EditorCapturePresentPlugin);
    let camera = spawn_editor_like_camera(&mut app);
    record_egui_window_mapping(&mut app, camera);
    settle(&mut app);
    app
}

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

#[test]
fn no_present_pass_appears_until_the_camera_is_retargeted() {
    let mut app = headless_windowed_app();
    app.add_plugins(EditorCapturePresentPlugin);
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
