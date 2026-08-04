//! Initial camera focus and bounds clamp.

use bevy::{prelude::*, window::PrimaryWindow};
use gdtf_battle_sim::{
    battle::PlayerFaction,
    occupancy::{GRID_HEIGHT, GRID_WIDTH},
    prelude::{Cell, Faction, Level, Position},
};

use super::{
    marker::WorldCamera,
    tuning::{BoundsMarginWorld, PanTuning},
};
use crate::cell_to_world;

/// Average of world-space centers, or `None` when empty.
#[must_use]
pub fn camera_focus(centers: impl IntoIterator<Item = Vec2>) -> Option<Vec2> {
    let mut count: u32 = 0;
    let mut sum = Vec2::ZERO;
    for center in centers {
        sum += center;
        count += 1;
    }
    if count == 0 {
        return None;
    }
    let divisor = count as f32;
    Some(sum / divisor)
}

/// Clamp a camera translation so the viewport stays over the world rect.
#[must_use]
pub fn clamp_camera(
    translation: Vec2,
    half_viewport: Vec2,
    world_min: Vec2,
    world_max: Vec2,
) -> Vec2 {
    Vec2::new(
        clamp_axis(translation.x, half_viewport.x, world_min.x, world_max.x),
        clamp_axis(translation.y, half_viewport.y, world_min.y, world_max.y),
    )
}

fn clamp_axis(value: f32, half: f32, min: f32, max: f32) -> f32 {
    let span = max - min;
    if span < 2.0 * half {
        return span.mul_add(0.5, min);
    }
    value.clamp(min + half, max - half)
}

fn battlefield_world_bounds() -> (Vec2, Vec2) {
    let max_x = i32::try_from(GRID_WIDTH).unwrap_or(i32::MAX);
    let max_y = i32::try_from(GRID_HEIGHT).unwrap_or(i32::MAX);
    let ground = Level::new(0);

    let corners = [
        cell_to_world(Cell::new(0, 0), ground),
        cell_to_world(Cell::new(max_x, 0), ground),
        cell_to_world(Cell::new(0, max_y), ground),
        cell_to_world(Cell::new(max_x, max_y), ground),
    ];

    let mut world_min = Vec2::new(corners[0].x, corners[0].y);
    let mut world_max = world_min;
    for corner in corners {
        let p = Vec2::new(corner.x, corner.y);
        world_min = world_min.min(p);
        world_max = world_max.max(p);
    }
    (world_min, world_max)
}

fn viewport_half_extent(
    camera: &Camera,
    projection: &Projection,
    window: Option<&Window>,
) -> Option<Vec2> {
    let Projection::Orthographic(ortho) = projection else {
        return None;
    };
    let area_half = ortho.area.half_size();
    if area_half.x > 0.0 && area_half.y > 0.0 {
        return Some(area_half);
    }
    let window = window?;
    let logical_size = match &camera.viewport {
        Some(viewport) => viewport.physical_size.as_vec2() / window.scale_factor(),
        None => window.size(),
    };
    Some(logical_size * 0.5 * ortho.scale)
}

/// Once per battle, center the camera on the player's gangers.
pub fn frame_camera_on_units(
    mut already_framed: Local<bool>,
    player: Res<PlayerFaction>,
    gangers: Query<(&Faction, &Position)>,
    mut cameras: Query<&mut Transform, With<WorldCamera>>,
) {
    if *already_framed {
        return;
    }
    let player_faction = **player;
    let centers = gangers
        .iter()
        .filter(|(faction, _)| **faction == player_faction)
        .map(|(_, pos)| {
            let world = cell_to_world(pos.cell(), Level::new(0));
            Vec2::new(world.x, world.y)
        });
    let Some(focus) = camera_focus(centers) else {
        return;
    };
    for mut transform in &mut cameras {
        transform.translation.x = focus.x;
        transform.translation.y = focus.y;
    }
    *already_framed = true;
}

/// Keep the world camera inside the battlefield plus configured margin.
pub fn clamp_camera_to_bounds(
    tuning: Option<Res<PanTuning>>,
    mut cameras: Query<(&Camera, &mut Transform, &Projection), With<WorldCamera>>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let window = windows.iter().next();
    let margin = tuning
        .as_ref()
        .map_or_else(BoundsMarginWorld::default, |t| t.bounds_margin_world);
    let (hard_min, hard_max) = battlefield_world_bounds();
    let world_min = hard_min - Vec2::splat(*margin);
    let world_max = hard_max + Vec2::splat(*margin);
    for (camera, mut transform, projection) in &mut cameras {
        let Some(half_viewport) = viewport_half_extent(camera, projection, window) else {
            continue;
        };
        let current = Vec2::new(transform.translation.x, transform.translation.y);
        let clamped = clamp_camera(current, half_viewport, world_min, world_max);
        transform.translation.x = clamped.x;
        transform.translation.y = clamped.y;
    }
}
