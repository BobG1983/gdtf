//! Gamepad virtual cursor and pointer ownership.

use bevy::prelude::*;

/// Screen-space gamepad cursor position.
#[derive(Resource, Deref, Debug, Clone, Copy, PartialEq)]
pub struct GamepadCursor(Vec2);

impl GamepadCursor {
    /// Build from a screen position.
    #[must_use]
    pub const fn new(pos: Vec2) -> Self {
        Self(pos)
    }

    /// Default centre of a 1280×720 viewport.
    pub const DEFAULT_CENTRE: Vec2 = Vec2::new(640.0, 360.0);
}

impl Default for GamepadCursor {
    fn default() -> Self {
        Self(Self::DEFAULT_CENTRE)
    }
}

/// Which device currently owns the pointer.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ActivePointer {
    /// Mouse owns the pointer.
    #[default]
    Mouse,
    /// Gamepad stick owns the pointer.
    Gamepad,
}

/// Cursor movement speed in pixels per second.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct CursorSpeed(f32);

impl CursorSpeed {
    /// Build from pixels per second.
    #[must_use]
    pub const fn new(px_per_second: f32) -> Self {
        Self(px_per_second)
    }
}

/// Stick magnitude below which movement is ignored.
#[derive(Deref, Clone, Copy, PartialEq, Debug)]
pub struct CursorStickDeadzone(f32);

impl CursorStickDeadzone {
    /// Build from a unit-stick magnitude threshold.
    #[must_use]
    pub const fn new(magnitude: f32) -> Self {
        Self(magnitude)
    }
}

/// Default cursor speed.
pub const CURSOR_SPEED: CursorSpeed = CursorSpeed::new(900.0);

/// Default stick deadzone.
pub const CURSOR_STICK_DEADZONE: CursorStickDeadzone = CursorStickDeadzone::new(0.15);

/// Advance the cursor by stick input, clamped to the window.
#[must_use]
pub fn move_cursor(pos: Vec2, stick: Vec2, speed: CursorSpeed, dt: f32, window: Vec2) -> Vec2 {
    let delta = Vec2::new(stick.x, -stick.y) * *speed * dt;
    let moved = pos + delta;
    moved.clamp(Vec2::ZERO, window)
}
