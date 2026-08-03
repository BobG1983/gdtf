//! Pure direction helpers for camera pan inputs.

use bevy::prelude::*;

use super::tunables::{EdgeBandPx, PanSpeed, StickDeadzone};

/// Pan direction from mouse position inside a full window.
#[must_use]
pub fn mouse_edge_dir(cursor: Vec2, size: Vec2, edge: EdgeBandPx) -> Vec2 {
    let band = *edge;
    let mut dir = Vec2::ZERO;
    if cursor.x < band {
        dir.x -= 1.0;
    } else if cursor.x > size.x - band {
        dir.x += 1.0;
    }
    if cursor.y < band {
        dir.y += 1.0;
    } else if cursor.y > size.y - band {
        dir.y -= 1.0;
    }
    dir
}

/// Pan direction from mouse position inside a camera viewport rect.
#[must_use]
pub fn viewport_edge_dir(cursor: Vec2, viewport: Rect, edge: EdgeBandPx) -> Vec2 {
    if !viewport.contains(cursor) {
        return Vec2::ZERO;
    }
    mouse_edge_dir(cursor - viewport.min, viewport.size(), edge)
}

/// Keyboard WASD / arrow pan direction from four independent key states.
#[expect(
    clippy::fn_params_excessive_bools,
    reason = "the four pan-key pressed states are the contract's specified keyboard_pan_dir \
              signature (up/down/left/right); they are independent inputs, not a flag soup"
)]
#[must_use]
pub fn keyboard_pan_dir(up: bool, down: bool, left: bool, right: bool) -> Vec2 {
    let x = f32::from(right) - f32::from(left);
    let y = f32::from(up) - f32::from(down);
    Vec2::new(x, y)
}

/// Stick pan direction after applying the deadzone.
#[must_use]
pub fn stick_pan_dir(stick: Vec2, deadzone: StickDeadzone) -> Vec2 {
    if stick.length() <= *deadzone {
        return Vec2::ZERO;
    }
    stick
}

/// Scale a unit-or-smaller direction by pan speed.
#[must_use]
pub fn pan_velocity(dir: Vec2, speed: PanSpeed) -> Vec2 {
    let length = dir.length();
    if length == 0.0 {
        return Vec2::ZERO;
    }
    let clamped = if length > 1.0 { dir / length } else { dir };
    clamped * *speed
}
