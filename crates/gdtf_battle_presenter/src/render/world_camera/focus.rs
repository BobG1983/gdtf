//! The one place a world camera's translation is written.

use bevy::prelude::*;

/// Put the world camera's centre on `focus`, leaving z alone.
pub const fn set_camera_focus(transform: &mut Transform, focus: Vec2) {
    transform.translation.x = focus.x;
    transform.translation.y = focus.y;
}

/// Shift the world camera by `delta` world units.
pub fn pan_camera_by(transform: &mut Transform, delta: Vec2) {
    let current = Vec2::new(transform.translation.x, transform.translation.y);
    set_camera_focus(transform, current + delta);
}
