use bevy::{
    camera::{RenderTarget, visibility::RenderLayers},
    prelude::*,
};

use super::harness::{aim_a_camera_at_the_target, capture_target, headless_windowed_app, settle};
use crate::present::{
    CapturePresentPlugin, PRESENT_LAYER, PRESENT_ORDER, PresentCamera, PresentSprite,
};

fn app_with_a_camera_on_the_target() -> App {
    let mut app = headless_windowed_app();
    app.add_plugins(CapturePresentPlugin);
    settle(&mut app);
    let aimed = aim_a_camera_at_the_target(&mut app);
    assert!(
        aimed.is_some(),
        "test setup: the present path must have created a capture target to aim at",
    );
    settle(&mut app);
    app
}

#[test]
fn a_present_camera_targets_the_window() {
    let mut app = app_with_a_camera_on_the_target();

    let mut present = app
        .world_mut()
        .query_filtered::<&RenderTarget, With<PresentCamera>>();
    let targets: Vec<RenderTarget> = present.iter(app.world()).cloned().collect();
    assert!(
        targets.len() == 1 && matches!(targets.first(), Some(RenderTarget::Window(_))),
        "exactly one present camera must target the window, got {targets:?}",
    );
}

#[test]
fn the_present_camera_composites_last_on_its_own_layer() {
    let mut app = app_with_a_camera_on_the_target();

    let mut others = app
        .world_mut()
        .query_filtered::<&Camera, Without<PresentCamera>>();
    let other_orders: Vec<isize> = others
        .iter(app.world())
        .map(|camera| camera.order)
        .collect();
    assert!(
        !other_orders.is_empty(),
        "test setup: another camera must exist to compare orders against",
    );
    let mut present = app
        .world_mut()
        .query_filtered::<(&Camera, &RenderLayers), With<PresentCamera>>();
    let found: Vec<(isize, RenderLayers)> = present
        .iter(app.world())
        .map(|(camera, layers)| (camera.order, layers.clone()))
        .collect();
    let [(order, layers)] = found.as_slice() else {
        unreachable!("exactly one present camera exists: {found:?}");
    };
    assert!(
        *order == PRESENT_ORDER && other_orders.iter().all(|other| order > other),
        "the present camera must carry PRESENT_ORDER ({PRESENT_ORDER}) and sort above every \
         other camera in the app ({other_orders:?}), so the blit composites last; got {order}",
    );
    assert!(
        *layers == RenderLayers::layer(PRESENT_LAYER) && PRESENT_LAYER > 1,
        "the present camera must render ONLY PRESENT_LAYER ({PRESENT_LAYER}), a layer nothing \
         else draws to; got {layers:?}",
    );
}

#[test]
fn the_blit_sprite_shows_the_capture_image_at_the_windows_logical_size() {
    let mut app = app_with_a_camera_on_the_target();
    let created = capture_target(&app);

    let mut sprites = app
        .world_mut()
        .query_filtered::<(&Sprite, &RenderLayers), With<PresentSprite>>();
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
fn no_present_pass_appears_until_a_camera_renders_into_the_target() {
    let mut app = headless_windowed_app();
    app.add_plugins(CapturePresentPlugin);
    settle(&mut app);

    assert!(
        capture_target(&app).is_some(),
        "the offscreen target must still be created — the gate is on what RENDERS into it, not \
         on the target",
    );
    let mut cameras = app
        .world_mut()
        .query_filtered::<Entity, With<PresentCamera>>();
    let present: Vec<Entity> = cameras.iter(app.world()).collect();
    let mut sprites = app
        .world_mut()
        .query_filtered::<Entity, With<PresentSprite>>();
    let blits: Vec<Entity> = sprites.iter(app.world()).collect();
    assert!(
        present.is_empty() && blits.is_empty(),
        "no present camera and no blit sprite may exist while nothing renders into the capture \
         image — compositing an untouched image over a live window would blank it; got cameras \
         {present:?}, sprites {blits:?}",
    );
}
