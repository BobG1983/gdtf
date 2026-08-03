use bevy::{asset::AssetPlugin, camera::visibility::RenderLayers, prelude::*, scene::ScenePlugin};

use super::super::marker::{WORLD_RENDER_LAYER, WorldCamera, spawn_world_camera};

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

/// Pin-discriminating: if the explicit projection were dropped, the `#[require]`d
#[test]
fn world_camera_spawns_at_half_orthographic_scale() {
    let mut app = App::new();
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
