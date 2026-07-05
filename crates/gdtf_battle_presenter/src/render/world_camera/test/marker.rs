//! Tests of the world-camera marker surface: render-layer isolation and the
//! GTW-263 spawn/zoom.

use bevy::{asset::AssetPlugin, camera::visibility::RenderLayers, prelude::*, scene::ScenePlugin};

use super::super::marker::{WORLD_RENDER_LAYER, WorldCamera, spawn_world_camera};

/// The configured world render layer does not intersect the UI camera's default
/// layer 0 (the local guarantee behind AC3) — exercised through the real
/// `RenderLayers::intersects` so it is genuine runtime behaviour, not a constant.
#[test]
fn world_render_layer_misses_layer_zero() {
    let world_layer = RenderLayers::layer(WORLD_RENDER_LAYER);
    let ui_layer = RenderLayers::layer(0);
    assert!(
        !world_layer.intersects(&ui_layer),
        "the world camera's render layer ({WORLD_RENDER_LAYER}) must not intersect the UI \
         camera's default layer 0",
    );
}

/// GTW-263 — `spawn_world_camera` spawns the `WorldCamera` with an orthographic
/// projection at `scale = 0.5` (a 2x zoom-in), OVERRIDING the `Camera2d` default of `1.0`.
///
/// Pin-discriminating: if the explicit projection were dropped, the `#[require]`d
/// `default_2d` projection (`scale = 1.0`) would be on the entity instead and the assert
/// would fail. Halving the scale halves the visible half-extent (`window * 0.5 * scale`),
/// so the battlefield draws at twice the size — the play-test "too zoomed out" fix.
#[test]
fn world_camera_spawns_at_half_orthographic_scale() {
    let mut app = App::new();
    // GTW-322 — `spawn_world_camera` now authors its camera via `bsn!` / `spawn_scene`, which
    // resolves in the `SpawnScene` schedule and PANICS without the scene resources; add
    // `AssetPlugin` + `ScenePlugin` (the camera has no asset deps, so the scene materializes
    // on the first `update`, with a second `update` to settle before the query reads it).
    app.add_plugins((MinimalPlugins, AssetPlugin::default(), ScenePlugin));
    app.add_systems(Startup, spawn_world_camera);
    app.update();
    app.update();

    let mut cameras = app
        .world_mut()
        .query_filtered::<&Projection, With<WorldCamera>>();
    let scale = cameras.iter(app.world()).next().and_then(|projection| {
        if let Projection::Orthographic(ortho) = projection {
            Some(ortho.scale)
        } else {
            None
        }
    });
    assert_eq!(
        scale.map(f32::to_bits),
        Some(0.5_f32.to_bits()),
        "the world camera must spawn one orthographic projection at scale 0.5 (2x zoom), \
         not the default_2d 1.0",
    );
}
