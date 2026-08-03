use bevy::{
    camera::{ClearColorConfig, visibility::RenderLayers},
    prelude::*,
    scene::{CommandsSceneExt, bsn, template_value},
};

pub const WORLD_RENDER_LAYER: usize = 1;

const WORLD_CAMERA_ORDER: isize = -1;

const MARGIN_BG: Color = Color::srgb(0.06, 0.06, 0.08);

const WORLD_CAMERA_SCALE: f32 = 0.5;

#[derive(Component, Debug, Clone, Copy, Eq, PartialEq, Hash, Default)]
pub struct WorldCamera;

/// `#[require]`.
pub fn spawn_world_camera(mut commands: Commands) {
    let camera = Camera {
        order: WORLD_CAMERA_ORDER,
        clear_color: ClearColorConfig::Custom(MARGIN_BG),
        ..default()
    };
    let layers = RenderLayers::layer(WORLD_RENDER_LAYER);
    // GTW-263 — zoom the battle in 2x. `Camera2d` `#[require]`s a default-2d
    let projection = Projection::Orthographic(OrthographicProjection {
        scale: WORLD_CAMERA_SCALE,
        ..OrthographicProjection::default_2d()
    });
    commands.spawn_scene((
        bsn! { Camera2d WorldCamera },
        template_value(camera),
        template_value(layers),
        template_value(projection),
    ));
}

pub fn despawn_world_camera(mut commands: Commands, cameras: Query<Entity, With<WorldCamera>>) {
    for camera in &cameras {
        commands.entity(camera).despawn();
    }
}
