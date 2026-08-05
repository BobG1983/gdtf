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

/// Which way one pan axis is being pushed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PanAxis {
    /// Neither key held, or both (they cancel).
    #[default]
    Still,
    /// Toward up / right.
    Positive,
    /// Toward down / left.
    Negative,
}

impl PanAxis {
    /// Resolve one axis from its two opposed keys.
    #[must_use]
    pub const fn from_keys(positive: bool, negative: bool) -> Self {
        match (positive, negative) {
            (true, false) => Self::Positive,
            (false, true) => Self::Negative,
            _ => Self::Still,
        }
    }

    /// Signed unit contribution of this axis.
    pub(crate) const fn signum(self) -> PanUnit {
        match self {
            Self::Still => PanUnit::new(0.0),
            Self::Positive => PanUnit::new(1.0),
            Self::Negative => PanUnit::new(-1.0),
        }
    }
}

/// One axis's signed unit share of the pan direction.
#[derive(Deref, Debug, Clone, Copy)]
pub(crate) struct PanUnit(f32);

impl PanUnit {
    /// Wrap a signed unit share.
    const fn new(unit: f32) -> Self {
        Self(unit)
    }
}

/// Keyboard WASD / arrow pan direction from both resolved axes.
#[must_use]
pub fn keyboard_pan_dir(vertical: PanAxis, horizontal: PanAxis) -> Vec2 {
    Vec2::new(*horizontal.signum(), *vertical.signum())
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
